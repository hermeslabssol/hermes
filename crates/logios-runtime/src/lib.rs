//! # logios-runtime
//!
//! The **SVM (Sealevel) execution engine** at the heart of Logios.
//!
//! ## The SVM model
//!
//! Logios state lives entirely in *accounts*. An [`Account`] is a buffer of
//! `data` with a `lamports` balance, an `owner` program ([`Pubkey`]), and an
//! `executable` flag. Only an account's owner program may debit its lamports or
//! mutate its data — the runtime enforces this on write-back.
//!
//! Execution is driven by *transactions*, which are ordered lists of
//! [`Instruction`]s. Each instruction names a `program_id` and the accounts it
//! touches. The runtime hands the program an [`InvokeContext`] — a scoped view
//! of those accounts plus a [`ComputeBudget`] — and the program mutates the
//! accounts in place while *metering* its work in **compute units** via
//! [`ComputeBudget::consume`]. Exceeding the per-slot budget aborts the
//! transaction with [`RuntimeError::ComputeBudgetExceeded`].
//!
//! The [`Executor`] ties it together: it loads accounts from an [`AccountsDb`],
//! runs every instruction in order under a shared compute budget, and — only if
//! the whole transaction succeeds — commits the account writes back and returns
//! an [`ExecResult`] with the writes and total CU consumed. Any failure leaves
//! the [`AccountsDb`] untouched (all-or-nothing, like a Solana transaction).
//!
//! There is **no EVM here**: no gas, no opcodes, no global state root — just
//! accounts, programs, and a compute budget.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod account;
mod budget;
mod context;
mod error;
mod executor;
mod program;

pub use account::{Account, AccountsDb};
pub use budget::ComputeBudget;
pub use context::{AccountMeta, InvokeContext};
pub use error::RuntimeError;
pub use executor::{transfer_lamports, ExecResult, Executor, Instruction, Transaction};
pub use program::Program;

// Convenience re-exports.
pub use logios_primitives::{ComputeUnits, Lamports, Pubkey};
