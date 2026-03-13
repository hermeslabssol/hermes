//! The [`Program`] trait — the unit of on-chain logic in the SVM.

use crate::context::InvokeContext;
use crate::error::RuntimeError;
use logios_primitives::Pubkey;

/// A deployed SVM program.
///
/// A program is pure logic: it reads and mutates the accounts handed to it via
/// the [`InvokeContext`] and meters its own work against the compute budget. It
/// owns no state of its own beyond its account data — exactly the Sealevel
/// model that lets non-overlapping transactions execute in parallel.
pub trait Program: Send + Sync {
    /// The program's id (its on-chain address).
    fn id(&self) -> Pubkey;

    /// Execute one instruction within `ctx`.
    ///
    /// Implementations should call [`InvokeContext::consume`] to charge compute
    /// units commensurate with the work performed, then mutate accounts via
    /// [`InvokeContext::account_mut`].
    ///
    /// # Errors
    ///
    /// Any [`RuntimeError`]; returning an error aborts the whole transaction
    /// and rolls back all account writes.
    fn execute(&self, ctx: &mut InvokeContext) -> Result<(), RuntimeError>;
}
