//! [`InvokeContext`]: the scoped view handed to a program during execution.

use crate::account::Account;
use crate::budget::ComputeBudget;
use crate::error::RuntimeError;
use logios_primitives::{ComputeUnits, Pubkey};

/// Per-instruction account reference plus its access flags.
///
/// Mirrors Solana's `AccountMeta`: an instruction declares, up front, which
/// accounts it reads, which it writes, and which must have signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountMeta {
    /// The account's address.
    pub pubkey: Pubkey,
    /// Whether the instruction may mutate the account.
    pub is_writable: bool,
    /// Whether the account authorized the transaction with a signature.
    pub is_signer: bool,
}

impl AccountMeta {
    /// A writable, signing account.
    #[must_use]
    pub fn writable_signer(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_writable: true,
            is_signer: true,
        }
    }

    /// A writable, non-signing account.
    #[must_use]
    pub fn writable(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_writable: true,
            is_signer: false,
        }
    }

    /// A read-only account.
    #[must_use]
    pub fn readonly(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_writable: false,
            is_signer: false,
        }
    }
}

/// The execution sandbox a [`Program`](crate::Program) sees for one
/// instruction.
///
/// Accounts are loaded by the executor into `accounts`, positionally aligned
/// with the instruction's `metas`. The program mutates them in place and meters
/// its work against `budget`; the executor commits the (validated) changes back
/// to the [`AccountsDb`](crate::AccountsDb) only on overall success.
pub struct InvokeContext<'a> {
    /// The program being invoked.
    pub program_id: Pubkey,
    /// Account metas declared by the instruction, in order.
    pub metas: &'a [AccountMeta],
    /// Loaded accounts, positionally aligned with `metas`.
    pub accounts: Vec<Account>,
    /// Shared compute meter for the whole transaction.
    pub budget: &'a mut ComputeBudget,
    /// Opaque instruction payload.
    pub instruction_data: &'a [u8],
}

impl<'a> InvokeContext<'a> {
    /// Charge `cu` to the shared compute budget.
    ///
    /// # Errors
    ///
    /// Propagates [`RuntimeError::ComputeBudgetExceeded`].
    pub fn consume(&mut self, cu: ComputeUnits) -> Result<(), RuntimeError> {
        self.budget.consume(cu)
    }

    /// Borrow the account at instruction index `i`.
    ///
    /// # Errors
    ///
    /// [`RuntimeError::AccountIndexOutOfBounds`] if `i` is past the loaded set.
    pub fn account(&self, i: usize) -> Result<&Account, RuntimeError> {
        let len = self.accounts.len();
        self.accounts
            .get(i)
            .ok_or(RuntimeError::AccountIndexOutOfBounds(i, len))
    }

    /// Mutably borrow the account at instruction index `i`, enforcing that the
    /// instruction declared it writable.
    ///
    /// # Errors
    ///
    /// * [`RuntimeError::AccountIndexOutOfBounds`] if `i` is out of range.
    /// * [`RuntimeError::OwnerMismatch`] if the account is not writable for this
    ///   instruction (modeled as a privilege violation).
    pub fn account_mut(&mut self, i: usize) -> Result<&mut Account, RuntimeError> {
        let metas_len = self.metas.len();
        let meta = self
            .metas
            .get(i)
            .copied()
            .ok_or(RuntimeError::AccountIndexOutOfBounds(i, metas_len))?;
        if !meta.is_writable {
            return Err(RuntimeError::OwnerMismatch {
                program: self.program_id,
                account: meta.pubkey,
                owner: self
                    .accounts
                    .get(i)
                    .map(|a| a.owner)
                    .unwrap_or(Pubkey::SYSTEM),
            });
        }
        self.accounts
            .get_mut(i)
            .ok_or(RuntimeError::AccountIndexOutOfBounds(i, 0))
    }

    /// True if the account at index `i` signed the transaction.
    #[must_use]
    pub fn is_signer(&self, i: usize) -> bool {
        self.metas.get(i).map(|m| m.is_signer).unwrap_or(false)
    }

    /// Find the loaded index of a pubkey within this instruction.
    #[must_use]
    pub fn index_of(&self, key: &Pubkey) -> Option<usize> {
        self.metas.iter().position(|m| &m.pubkey == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logios_primitives::Lamports;

    fn ctx_fixture(budget: &mut ComputeBudget) -> (Vec<AccountMeta>, Vec<Account>) {
        let payer = Pubkey::new([1u8; 32]);
        let dest = Pubkey::new([2u8; 32]);
        let metas = vec![
            AccountMeta::writable_signer(payer),
            AccountMeta::readonly(dest),
        ];
        let accounts = vec![
            Account::empty(Lamports::new(100), Pubkey::SYSTEM),
            Account::empty(Lamports::new(0), Pubkey::SYSTEM),
        ];
        let _ = budget;
        (metas, accounts)
    }

    #[test]
    fn writable_meta_allows_mut() {
        let mut budget = ComputeBudget::slot_max();
        let (metas, accounts) = ctx_fixture(&mut budget);
        let mut ctx = InvokeContext {
            program_id: Pubkey::SYSTEM,
            metas: &metas,
            accounts,
            budget: &mut budget,
            instruction_data: &[],
        };
        assert!(ctx.account_mut(0).is_ok());
    }

    #[test]
    fn readonly_meta_blocks_mut() {
        let mut budget = ComputeBudget::slot_max();
        let (metas, accounts) = ctx_fixture(&mut budget);
        let mut ctx = InvokeContext {
            program_id: Pubkey::SYSTEM,
            metas: &metas,
            accounts,
            budget: &mut budget,
            instruction_data: &[],
        };
        assert!(matches!(
            ctx.account_mut(1),
            Err(RuntimeError::OwnerMismatch { .. })
        ));
    }

    #[test]
    fn index_lookup_and_signer_flags() {
        let mut budget = ComputeBudget::slot_max();
        let (metas, accounts) = ctx_fixture(&mut budget);
        let ctx = InvokeContext {
            program_id: Pubkey::SYSTEM,
            metas: &metas,
            accounts,
            budget: &mut budget,
            instruction_data: &[],
        };
        assert_eq!(ctx.index_of(&Pubkey::new([2u8; 32])), Some(1));
        assert!(ctx.is_signer(0));
        assert!(!ctx.is_signer(1));
    }
}
