//! Runtime error taxonomy.

use logios_primitives::Pubkey;
use thiserror::Error;

/// Errors raised while loading, executing, or committing a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuntimeError {
    /// The transaction's instructions tried to consume more compute units than
    /// the per-slot compute budget allows.
    #[error("compute budget exceeded: requested {requested} CU but only {remaining} CU remain")]
    ComputeBudgetExceeded {
        /// CU the program asked to consume.
        requested: u64,
        /// CU still available in the budget.
        remaining: u64,
    },

    /// An instruction referenced an account that was not loaded for it.
    #[error("account not found in invoke context: {0}")]
    AccountNotFound(Pubkey),

    /// An instruction referenced an account index outside the loaded set.
    #[error("account index {0} out of bounds ({1} accounts loaded)")]
    AccountIndexOutOfBounds(usize, usize),

    /// A program attempted to debit or mutate an account it does not own.
    #[error("privilege violation: program {program} may not modify account {account} owned by {owner}")]
    OwnerMismatch {
        /// The executing program id.
        program: Pubkey,
        /// The account being modified.
        account: Pubkey,
        /// The account's actual owner.
        owner: Pubkey,
    },

    /// A lamport debit would underflow an account's balance.
    #[error("insufficient lamports in {account}: balance {balance}, debit {debit}")]
    InsufficientFunds {
        /// Account being debited.
        account: Pubkey,
        /// Current balance.
        balance: u64,
        /// Requested debit.
        debit: u64,
    },

    /// The sum of lamports changed across the transaction (creation/destruction
    /// of value). The runtime enforces conservation, as Solana does.
    #[error("lamport balance not conserved: {before} before, {after} after")]
    UnbalancedLamports {
        /// Total lamports across touched accounts before execution.
        before: u64,
        /// Total lamports after execution.
        after: u64,
    },

    /// The named program id is not registered with the executor.
    #[error("unknown program: {0}")]
    UnknownProgram(Pubkey),

    /// A program returned a custom, program-defined error code.
    #[error("program error {code}: {message}")]
    Custom {
        /// Program-defined error code.
        code: u32,
        /// Human-readable description.
        message: String,
    },
}

impl RuntimeError {
    /// Build a program-defined custom error.
    #[must_use]
    pub fn custom(code: u32, message: impl Into<String>) -> Self {
        RuntimeError::Custom {
            code,
            message: message.into(),
        }
    }
}
