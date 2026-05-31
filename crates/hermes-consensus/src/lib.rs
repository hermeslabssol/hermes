//! # hermes-consensus
//!
//! Slot production and validator accountability for **Hermes**.
//!
//! Hermes runs Solana's consensus model — a rotating **leader schedule**, slots
//! as the unit of production, and **Tower-BFT** vote-lockout for finality. On
//! devnet there is a single autonomous leader (the Hermes agent), so the
//! schedule trivially resolves to "self"; the types are built to generalize to
//! a multi-validator set.
//!
//! This crate has three pillars:
//!
//! 1. [`LeaderSchedule`] — who is permitted to author a given [`Slot`].
//! 2. [`SlotProducer`] — orders a slot's transactions, drives the SVM runtime
//!    over them, and seals a [`SealedSlot`] with a base58 blockhash plus a
//!    decision [`Receipt`](hermes_ledger::Receipt).
//! 3. [`SlashingEngine`] — the validator-accountability state machine:
//!    registration, missed-slot tracking, offense recording, stake slashing in
//!    basis points, jailing/unjailing across epoch boundaries, a cooldown guard
//!    against double-slashing the same offense, and tombstoning after repeated
//!    infractions.
//!
//! The [`tower`] module is an honest **work-in-progress** stub for Tower-BFT
//! vote tracking — its lockout math is documented but not yet wired into
//! finality.
//!
//! No EVM concepts appear anywhere: production is measured in slots, penalties
//! in lamports/basis-points, and identities in base58 pubkeys.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod leader;
mod producer;
mod slashing;
pub mod tower;

pub use leader::{LeaderSchedule, LeaderScheduleError};
pub use producer::{ProducerError, SealedSlot, SlotProducer};
pub use slashing::{
    Offense, SlashOutcome, SlashingEngine, SlashingError, SlashingParams, ValidatorState,
    ValidatorStatus,
};

pub use hermes_primitives::{Epoch, Lamports, Pubkey, Slot};
