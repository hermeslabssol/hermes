//! The [`Executor`]: transaction-level orchestration over the SVM.

use crate::account::{Account, AccountsDb};
use crate::budget::ComputeBudget;
use crate::context::{AccountMeta, InvokeContext};
use crate::error::RuntimeError;
use crate::program::Program;
use hermes_primitives::{ComputeUnits, Lamports, Pubkey};
use std::collections::HashMap;

/// A single instruction: a call to one program over a set of accounts.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Instruction {
    /// Program to invoke.
    pub program_id: Pubkey,
    /// Accounts the instruction touches, with their access flags.
    pub accounts: Vec<AccountMeta>,
    /// Opaque instruction data.
    pub data: Vec<u8>,
}

impl Instruction {
    /// Build an instruction.
    #[must_use]
    pub fn new(program_id: Pubkey, accounts: Vec<AccountMeta>, data: Vec<u8>) -> Self {
        Self {
            program_id,
            accounts,
            data,
        }
    }
}

/// A transaction: an ordered list of instructions executed atomically under one
/// shared compute budget.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transaction {
    /// Instructions, executed in order.
    pub instructions: Vec<Instruction>,
}

impl Transaction {
    /// A transaction wrapping a single instruction.
    #[must_use]
    pub fn single(ix: Instruction) -> Self {
        Self {
            instructions: vec![ix],
        }
    }
}

/// The committed effect of a successful transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecResult {
    /// Accounts whose state changed, as `(pubkey, new_state)` pairs.
    pub account_writes: Vec<(Pubkey, Account)>,
    /// Total compute units charged across all instructions.
    pub compute_units_consumed: ComputeUnits,
}

impl ExecResult {
    /// Number of accounts written.
    #[must_use]
    pub fn writes(&self) -> usize {
        self.account_writes.len()
    }
}

/// Runs transactions against an [`AccountsDb`] using a registry of programs.
///
/// Execution is all-or-nothing: instructions run against a private working copy
/// of the touched accounts, and the copy is committed to the live `AccountsDb`
/// only if every instruction succeeds *and* invariants (owner authority,
/// lamport conservation) hold. Otherwise the `AccountsDb` is left untouched.
#[derive(Default)]
pub struct Executor {
    programs: HashMap<Pubkey, Box<dyn Program>>,
}

impl Executor {
    /// Create an executor with no programs registered.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a program under its id.
    pub fn register(&mut self, program: Box<dyn Program>) {
        self.programs.insert(program.id(), program);
    }

    /// Execute `tx` against `db` under a compute budget of `budget_cu` units.
    ///
    /// On success, commits account writes to `db` and returns the diff plus
    /// total CU consumed. On any error, `db` is left unchanged.
    ///
    /// # Errors
    ///
    /// Any [`RuntimeError`] surfaced by a program, the compute meter, or the
    /// post-execution invariant checks.
    pub fn execute(
        &self,
        db: &mut AccountsDb,
        tx: &Transaction,
        budget_cu: ComputeUnits,
    ) -> Result<ExecResult, RuntimeError> {
        let mut budget = ComputeBudget::new(budget_cu);

        // Working copy of every account the transaction touches. Untouched
        // accounts stay only in `db`. We snapshot pre-state for invariant checks.
        let mut working: HashMap<Pubkey, Account> = HashMap::new();
        let mut pre_lamports: u64 = 0;

        for ix in &tx.instructions {
            for meta in &ix.accounts {
                if working.contains_key(&meta.pubkey) {
                    continue;
                }
                let account = db
                    .load(&meta.pubkey)
                    .cloned()
                    .ok_or(RuntimeError::AccountNotFound(meta.pubkey))?;
                pre_lamports = pre_lamports
                    .checked_add(account.lamports.get())
                    .expect("pre-state lamport sum overflow");
                working.insert(meta.pubkey, account);
            }
        }

        // Execute each instruction against the shared working set.
        for ix in &tx.instructions {
            let program = self
                .programs
                .get(&ix.program_id)
                .ok_or(RuntimeError::UnknownProgram(ix.program_id))?;

            // Gather this instruction's accounts (in declared order) out of the
            // working set, run the program, then scatter the results back.
            let loaded: Vec<Account> = ix
                .accounts
                .iter()
                .map(|m| working.get(&m.pubkey).cloned().expect("preloaded above"))
                .collect();

            let mut ctx = InvokeContext {
                program_id: ix.program_id,
                metas: &ix.accounts,
                accounts: loaded,
                budget: &mut budget,
                instruction_data: &ix.data,
            };

            program.execute(&mut ctx)?;

            // Scatter mutated accounts back, enforcing owner authority on any
            // account whose state the program changed.
            for (meta, after) in ix.accounts.iter().zip(ctx.accounts) {
                let before = working.get(&meta.pubkey).expect("preloaded above");
                if before != &after {
                    Self::enforce_owner(ix.program_id, meta.pubkey, before)?;
                    working.insert(meta.pubkey, after);
                }
            }
        }

        // Lamport conservation across the whole transaction.
        let post_lamports: u64 = working.values().map(|a| a.lamports.get()).sum();
        if post_lamports != pre_lamports {
            return Err(RuntimeError::UnbalancedLamports {
                before: pre_lamports,
                after: post_lamports,
            });
        }

        // Commit: gather the diff against db, then store.
        let mut account_writes = Vec::new();
        for (key, after) in &working {
            let changed = db.load(key).map(|cur| cur != after).unwrap_or(true);
            if changed {
                account_writes.push((*key, after.clone()));
            }
        }
        // Deterministic ordering for reproducible receipts.
        account_writes.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

        for (key, after) in &account_writes {
            db.store(*key, after.clone());
        }

        Ok(ExecResult {
            account_writes,
            compute_units_consumed: budget.consumed(),
        })
    }

    /// A program may only mutate accounts it owns (or accounts owned by the
    /// system program, the analogue of Solana's system-owned, assignable
    /// accounts). Anything else is a privilege violation.
    fn enforce_owner(
        program: Pubkey,
        account: Pubkey,
        before: &Account,
    ) -> Result<(), RuntimeError> {
        if before.owner == program || before.owner == Pubkey::SYSTEM {
            Ok(())
        } else {
            Err(RuntimeError::OwnerMismatch {
                program,
                account,
                owner: before.owner,
            })
        }
    }
}

/// Helper for programs/tests: debit `from` and credit `to`, checking funds.
///
/// # Errors
///
/// [`RuntimeError::InsufficientFunds`] if `from` cannot cover `amount`.
pub fn transfer_lamports(
    from: &mut Account,
    to: &mut Account,
    amount: Lamports,
    from_key: Pubkey,
) -> Result<(), RuntimeError> {
    if from.lamports.get() < amount.get() {
        return Err(RuntimeError::InsufficientFunds {
            account: from_key,
            balance: from.lamports.get(),
            debit: amount.get(),
        });
    }
    from.lamports -= amount;
    to.lamports += amount;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal system-style program: instruction data is a little-endian u64
    /// lamport amount to move from account 0 (signer) to account 1.
    struct TransferProgram {
        id: Pubkey,
        cu_per_call: u64,
    }

    impl Program for TransferProgram {
        fn id(&self) -> Pubkey {
            self.id
        }

        fn execute(&self, ctx: &mut InvokeContext) -> Result<(), RuntimeError> {
            ctx.consume(ComputeUnits::new(self.cu_per_call))?;

            if !ctx.is_signer(0) {
                return Err(RuntimeError::custom(1, "source must sign"));
            }
            let amount = {
                let mut buf = [0u8; 8];
                let d = ctx.instruction_data;
                if d.len() < 8 {
                    return Err(RuntimeError::custom(2, "instruction data too short"));
                }
                buf.copy_from_slice(&d[..8]);
                Lamports::new(u64::from_le_bytes(buf))
            };

            // Split the borrows: take both accounts out, mutate, put back.
            let mut from = ctx.account(0)?.clone();
            let mut to = ctx.account(1)?.clone();
            let from_key = ctx.metas[0].pubkey;
            transfer_lamports(&mut from, &mut to, amount, from_key)?;
            *ctx.account_mut(0)? = from;
            *ctx.account_mut(1)? = to;
            Ok(())
        }
    }

    fn setup() -> (Executor, AccountsDb, Pubkey, Pubkey, Pubkey) {
        let prog_id = Pubkey::new([42u8; 32]);
        let payer = Pubkey::new([1u8; 32]);
        let dest = Pubkey::new([2u8; 32]);

        let mut exec = Executor::new();
        exec.register(Box::new(TransferProgram {
            id: prog_id,
            cu_per_call: 5_000,
        }));

        let mut db = AccountsDb::new();
        db.store(payer, Account::empty(Lamports::new(1_000), Pubkey::SYSTEM));
        db.store(dest, Account::empty(Lamports::new(0), Pubkey::SYSTEM));

        (exec, db, prog_id, payer, dest)
    }

    fn transfer_ix(prog: Pubkey, payer: Pubkey, dest: Pubkey, amount: u64) -> Instruction {
        Instruction::new(
            prog,
            vec![
                AccountMeta::writable_signer(payer),
                AccountMeta::writable(dest),
            ],
            amount.to_le_bytes().to_vec(),
        )
    }

    #[test]
    fn executes_transfer_and_meters_cu() {
        let (exec, mut db, prog, payer, dest) = setup();
        let tx = Transaction::single(transfer_ix(prog, payer, dest, 250));

        let result = exec
            .execute(&mut db, &tx, ComputeUnits::new(1_000_000))
            .unwrap();

        assert_eq!(db.balance(&payer).get(), 750);
        assert_eq!(db.balance(&dest).get(), 250);
        assert_eq!(result.compute_units_consumed.get(), 5_000);
        // Both accounts changed.
        assert_eq!(result.writes(), 2);
    }

    #[test]
    fn compute_budget_exceeded_aborts_and_rolls_back() {
        let (exec, mut db, prog, payer, dest) = setup();
        // Two instructions @ 5_000 CU each, but a 6_000 CU budget.
        let tx = Transaction {
            instructions: vec![
                transfer_ix(prog, payer, dest, 100),
                transfer_ix(prog, payer, dest, 100),
            ],
        };

        let err = exec
            .execute(&mut db, &tx, ComputeUnits::new(6_000))
            .unwrap_err();
        assert!(matches!(err, RuntimeError::ComputeBudgetExceeded { .. }));

        // Rollback: balances untouched even though the first instruction ran.
        assert_eq!(db.balance(&payer).get(), 1_000);
        assert_eq!(db.balance(&dest).get(), 0);
    }

    #[test]
    fn insufficient_funds_rolls_back() {
        let (exec, mut db, prog, payer, dest) = setup();
        let tx = Transaction::single(transfer_ix(prog, payer, dest, 5_000));
        let err = exec
            .execute(&mut db, &tx, ComputeUnits::new(1_000_000))
            .unwrap_err();
        assert!(matches!(err, RuntimeError::InsufficientFunds { .. }));
        assert_eq!(db.balance(&payer).get(), 1_000);
    }

    #[test]
    fn unknown_program_errors() {
        let (exec, mut db, _prog, payer, dest) = setup();
        let bogus = Pubkey::new([99u8; 32]);
        let tx = Transaction::single(transfer_ix(bogus, payer, dest, 1));
        let err = exec
            .execute(&mut db, &tx, ComputeUnits::new(1_000_000))
            .unwrap_err();
        assert!(matches!(err, RuntimeError::UnknownProgram(_)));
    }

    #[test]
    fn missing_account_errors() {
        let (exec, mut db, prog, payer, _dest) = setup();
        let ghost = Pubkey::new([7u8; 32]);
        let tx = Transaction::single(transfer_ix(prog, payer, ghost, 1));
        let err = exec
            .execute(&mut db, &tx, ComputeUnits::new(1_000_000))
            .unwrap_err();
        assert!(matches!(err, RuntimeError::AccountNotFound(_)));
    }

    #[test]
    fn two_instructions_accumulate_cu() {
        let (exec, mut db, prog, payer, dest) = setup();
        let tx = Transaction {
            instructions: vec![
                transfer_ix(prog, payer, dest, 100),
                transfer_ix(prog, payer, dest, 50),
            ],
        };
        let result = exec
            .execute(&mut db, &tx, ComputeUnits::new(1_000_000))
            .unwrap();
        assert_eq!(result.compute_units_consumed.get(), 10_000);
        assert_eq!(db.balance(&dest).get(), 150);
    }
}

// reviewed 2026-06-02
