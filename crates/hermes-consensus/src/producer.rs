//! [`SlotProducer`]: seal a slot by running its transactions through the SVM.

use crate::leader::LeaderSchedule;
use hermes_ledger::Receipt;
use hermes_primitives::{ComputeUnits, Hash, Pubkey, Signature, Slot, MAX_COMPUTE_UNITS_PER_SLOT};
use hermes_runtime::{AccountsDb, Executor, Transaction};
use thiserror::Error;

/// Errors raised while producing a slot.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProducerError {
    /// The producing identity is not the scheduled leader for the slot.
    #[error("identity {who} is not the leader for slot {slot}")]
    NotLeader {
        /// Identity that attempted production.
        who: Pubkey,
        /// Slot it tried to produce.
        slot: u64,
    },
    /// A transaction failed to execute; the slot was not sealed.
    #[error("transaction {index} failed: {source}")]
    TransactionFailed {
        /// Index of the failing transaction within the slot.
        index: usize,
        /// Underlying runtime error.
        #[source]
        source: hermes_runtime::RuntimeError,
    },
}

/// The output of producing a slot: a base58-identified block plus its receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedSlot {
    /// The slot height.
    pub slot: Slot,
    /// The leader that authored it.
    pub leader: Pubkey,
    /// The sealing blockhash (running SHA-256 over executed entries).
    pub blockhash: Hash,
    /// The signed decision receipt summarizing the slot.
    pub receipt: Receipt,
}

/// Produces slots for a configured leader identity.
///
/// The producer is a thin coordinator: it checks the [`LeaderSchedule`], runs
/// each transaction through the [`Executor`] against a shared [`AccountsDb`],
/// folds the per-transaction results into aggregate counts, derives the slot
/// blockhash, and emits a [`Receipt`].
pub struct SlotProducer<'a> {
    identity: Pubkey,
    schedule: &'a LeaderSchedule,
    executor: &'a Executor,
}

impl<'a> SlotProducer<'a> {
    /// Create a producer for `identity` using `schedule` and `executor`.
    #[must_use]
    pub fn new(identity: Pubkey, schedule: &'a LeaderSchedule, executor: &'a Executor) -> Self {
        Self {
            identity,
            schedule,
            executor,
        }
    }

    /// Produce and seal `slot` from an ordered batch of `transactions`,
    /// committing account writes to `db`.
    ///
    /// `narration` is the agent's natural-language rationale for the slot; it is
    /// embedded in (and signed as part of) the receipt. `sign` is a closure that
    /// turns the receipt's signing payload into a [`Signature`] — in production
    /// this is the leader's Ed25519 keypair; tests pass a deterministic stub.
    ///
    /// # Errors
    ///
    /// * [`ProducerError::NotLeader`] if `self` is not scheduled for `slot`.
    /// * [`ProducerError::TransactionFailed`] if any transaction errors. Because
    ///   the underlying [`Executor`] is all-or-nothing per transaction, a
    ///   failure rolls back only that transaction's writes; the slot as a whole
    ///   is then abandoned and no receipt is produced.
    pub fn produce<F>(
        &self,
        slot: Slot,
        transactions: &[Transaction],
        db: &mut AccountsDb,
        narration: impl Into<String>,
        sign: F,
    ) -> Result<SealedSlot, ProducerError>
    where
        F: FnOnce(&[u8]) -> Signature,
    {
        if !self.schedule.is_leader(&self.identity, slot) {
            return Err(ProducerError::NotLeader {
                who: self.identity,
                slot: slot.get(),
            });
        }

        let mut blockhash = Hash::hash(format!("hermes:slot:{}", slot.get()).as_bytes());
        let mut total_cu: u64 = 0;
        let mut total_writes: u32 = 0;
        let txns = transactions.len() as u32;

        // Whatever budget remains in the slot after each tx is the cap for the
        // next, so the aggregate never exceeds the per-slot ceiling.
        let mut slot_budget_remaining = MAX_COMPUTE_UNITS_PER_SLOT;

        for (index, tx) in transactions.iter().enumerate() {
            let result = self
                .executor
                .execute(db, tx, ComputeUnits::new(slot_budget_remaining))
                .map_err(|source| ProducerError::TransactionFailed { index, source })?;

            let cu = result.compute_units_consumed.get();
            total_cu += cu;
            slot_budget_remaining = slot_budget_remaining.saturating_sub(cu);
            total_writes += result.writes() as u32;

            // Extend the running blockhash with a digest of this tx's effect,
            // so the blockhash commits to the exact executed order and outcome.
            let mut entry = Vec::new();
            entry.extend_from_slice(&(index as u64).to_le_bytes());
            entry.extend_from_slice(&cu.to_le_bytes());
            for (key, account) in &result.account_writes {
                entry.extend_from_slice(key.as_bytes());
                entry.extend_from_slice(&account.lamports.get().to_le_bytes());
                entry.extend_from_slice(&account.data);
            }
            blockhash = blockhash.extend(&Hash::hash(&entry));
        }

        let receipt = Receipt::new(
            slot,
            blockhash,
            txns,
            total_writes,
            ComputeUnits::new(total_cu),
            narration,
        );
        let signature = sign(&receipt.signing_payload());
        let receipt = receipt.sign_with(signature);

        Ok(SealedSlot {
            slot,
            leader: self.identity,
            blockhash,
            receipt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_runtime::{Account, AccountMeta, Instruction, InvokeContext, Lamports, Program};
    use hermes_runtime::RuntimeError;

    /// Memo program: charges fixed CU, mutates account 0's data, no value move.
    struct MemoProgram {
        id: Pubkey,
    }
    impl Program for MemoProgram {
        fn id(&self) -> Pubkey {
            self.id
        }
        fn execute(&self, ctx: &mut InvokeContext) -> Result<(), RuntimeError> {
            ctx.consume(ComputeUnits::new(2_000))?;
            let data = ctx.instruction_data.to_vec();
            ctx.account_mut(0)?.data = data;
            Ok(())
        }
    }

    fn stub_sign(payload: &[u8]) -> Signature {
        // Deterministic non-zero "signature": first 64 bytes of a hash chain.
        let mut bytes = [0u8; 64];
        let h1 = Hash::hash(payload);
        let h2 = Hash::hash(h1.as_bytes());
        bytes[..32].copy_from_slice(h1.as_bytes());
        bytes[32..].copy_from_slice(h2.as_bytes());
        Signature::new(bytes)
    }

    fn setup() -> (Executor, AccountsDb, Pubkey, Pubkey) {
        let prog = Pubkey::new([7u8; 32]);
        let acct = Pubkey::new([8u8; 32]);
        let mut exec = Executor::new();
        exec.register(Box::new(MemoProgram { id: prog }));
        let mut db = AccountsDb::new();
        db.store(acct, Account::empty(Lamports::new(0), Pubkey::SYSTEM));
        (exec, db, prog, acct)
    }

    fn memo_tx(prog: Pubkey, acct: Pubkey, msg: &[u8]) -> Transaction {
        Transaction::single(Instruction::new(
            prog,
            vec![AccountMeta::writable(acct)],
            msg.to_vec(),
        ))
    }

    #[test]
    fn produces_signed_sealed_slot() {
        let (exec, mut db, prog, acct) = setup();
        let me = Pubkey::new([1u8; 32]);
        let sched = LeaderSchedule::single(me);
        let producer = SlotProducer::new(me, &sched, &exec);

        let txs = vec![
            memo_tx(prog, acct, b"hello"),
            memo_tx(prog, acct, b"world"),
        ];
        let sealed = producer
            .produce(Slot::new(5), &txs, &mut db, "two memos", stub_sign)
            .unwrap();

        assert_eq!(sealed.slot, Slot::new(5));
        assert_eq!(sealed.leader, me);
        assert_eq!(sealed.receipt.txns, 2);
        assert_eq!(sealed.receipt.compute_units.get(), 4_000);
        assert!(sealed.receipt.is_signed());
        assert_eq!(sealed.receipt.narration, "two memos");
        // Last write wins on the account data.
        assert_eq!(db.load(&acct).unwrap().data, b"world");
    }

    #[test]
    fn non_leader_cannot_produce() {
        let (exec, mut db, _prog, _acct) = setup();
        let me = Pubkey::new([1u8; 32]);
        let other = Pubkey::new([2u8; 32]);
        // Schedule names `other` as the only validator.
        let sched = LeaderSchedule::single(other);
        let producer = SlotProducer::new(me, &sched, &exec);
        let err = producer
            .produce(Slot::new(0), &[], &mut db, "nope", stub_sign)
            .unwrap_err();
        assert!(matches!(err, ProducerError::NotLeader { .. }));
    }

    #[test]
    fn blockhash_commits_to_tx_order() {
        let (exec, mut db1, prog, acct) = setup();
        let (_e2, mut db2, _p2, _a2) = setup();
        let me = Pubkey::new([1u8; 32]);
        let sched = LeaderSchedule::single(me);
        let producer = SlotProducer::new(me, &sched, &exec);

        let order_a = vec![memo_tx(prog, acct, b"a"), memo_tx(prog, acct, b"b")];
        let order_b = vec![memo_tx(prog, acct, b"b"), memo_tx(prog, acct, b"a")];

        let s1 = producer
            .produce(Slot::new(1), &order_a, &mut db1, "ab", stub_sign)
            .unwrap();
        let s2 = producer
            .produce(Slot::new(1), &order_b, &mut db2, "ba", stub_sign)
            .unwrap();
        assert_ne!(s1.blockhash, s2.blockhash);
    }
}
