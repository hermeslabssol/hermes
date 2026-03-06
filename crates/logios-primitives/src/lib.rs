//! # logios-primitives
//!
//! Core primitive types for **Logios**, the chain that writes itself.
//!
//! Logios is **Solana-native**: it runs the SVM (Sealevel) runtime, addresses
//! state through *accounts* owned by *programs*, meters execution in *compute
//! units* (CU) against a per-slot compute budget, prices priority in
//! *lamports*, and identifies everything — pubkeys, blockhashes, signatures —
//! with **base58** strings.
//!
//! This crate is deliberately tiny and dependency-light. It defines the
//! vocabulary the rest of the protocol (`logios-runtime`, `logios-consensus`,
//! `logios-ledger`) is written in.
//!
//! ## Encoding conventions
//!
//! * [`Pubkey`] — 32 bytes, displayed as base58 (e.g. an account address or
//!   program id). Never hex.
//! * [`Hash`] — 32 bytes, displayed as base58 (e.g. a slot blockhash).
//! * [`Signature`] — 64 bytes, displayed as base58 (Ed25519 over the message).
//!
//! ## Units
//!
//! * [`Slot`] — monotonic slot height; the unit of block production.
//! * [`Epoch`] — a fixed number of slots; the unit of leader rotation and
//!   validator jail accounting.
//! * [`Lamports`] — the indivisible unit of $HERMES value
//!   ([`LAMPORTS_PER_SOL`] lamports to one whole token).
//! * [`ComputeUnits`] — Sealevel execution cost, capped per slot at
//!   [`MAX_COMPUTE_UNITS_PER_SLOT`].

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod base58;
mod hash;
mod pubkey;
mod signature;
mod units;

pub use base58::{decode_base58, encode_base58, Base58Error};
pub use hash::{Hash, ParseHashError};
pub use pubkey::{ParsePubkeyError, Pubkey};
pub use signature::{ParseSignatureError, Signature};
pub use units::{ComputeUnits, Epoch, Lamports, Slot};

/// Per-slot Sealevel compute budget ceiling, in compute units.
///
/// A transaction batch sealed into a slot may consume at most this many CU in
/// aggregate. Mirrors Solana mainnet's block compute limit.
pub const MAX_COMPUTE_UNITS_PER_SLOT: u64 = 48_000_000;

/// Lamports in one whole token. $HERMES, like SOL, has nine decimals.
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Width, in bytes, of a [`Pubkey`] or [`Hash`].
pub const PUBKEY_BYTES: usize = 32;

/// Width, in bytes, of an Ed25519 [`Signature`].
pub const SIGNATURE_BYTES: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_svm_native() {
        assert_eq!(MAX_COMPUTE_UNITS_PER_SLOT, 48_000_000);
        assert_eq!(LAMPORTS_PER_SOL, 1_000_000_000);
        assert_eq!(PUBKEY_BYTES, 32);
        assert_eq!(SIGNATURE_BYTES, 64);
    }
}
