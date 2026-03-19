//! # logios-ledger
//!
//! The append-only **decision-receipt ledger** for Logios.
//!
//! Logios is "the chain that writes itself": each slot is authored by an
//! autonomous AI leader that, after sealing the slot through the SVM runtime,
//! emits a signed [`Receipt`] explaining and attesting to what it did. This
//! crate is the durable record of those receipts.
//!
//! A [`Receipt`] is intentionally lightweight — it summarizes a sealed slot
//! (its base58 blockhash, transaction count, account-write count, and total
//! compute units consumed against the per-slot budget), carries a short
//! human-readable `narration`, and is sealed with the leader's Ed25519
//! [`Signature`]. The full transaction set lives in the runtime/consensus
//! layers; the ledger keeps the audit trail.
//!
//! The [`Ledger`] supports push, point lookup by slot, slot ranges, and
//! pruning of receipts below a watermark (to bound memory on long-running
//! devnet nodes).

#![cfg_attr(docsrs, feature(doc_cfg))]

mod ledger;
mod receipt;

pub use ledger::{Ledger, LedgerError};
pub use receipt::Receipt;

// Re-export the primitive types that appear in the public API so downstream
// crates can name them without a direct dependency.
pub use logios_primitives::{ComputeUnits, Hash, Signature, Slot};
