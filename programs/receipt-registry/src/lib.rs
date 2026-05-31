//! # receipt-registry
//!
//! On-chain registry for **Hermes decision receipts** — Hermes Labs.
//!
//! Hermes is "the chain that writes itself": for every slot, the autonomous
//! Hermes agent authors the block and signs a *decision receipt* describing
//! what it chose to include and why. This program anchors those receipts on
//! Solana so that anyone can audit the agent's per-slot decisions against the
//! ledger.
//!
//! ## Account model
//!
//! - [`RegistryConfig`] — a singleton PDA (`seeds = [b"config"]`) that names the
//!   Hermes agent authority, tracks the highest committed slot, and counts the
//!   total receipts anchored so far.
//! - [`Receipt`] — one PDA per slot (`seeds = [b"receipt", slot.to_le_bytes()]`)
//!   holding the slot number, the slot blockhash bytes, the number of
//!   transactions packed into the block, the compute units consumed, and a
//!   SHA-256 hash of the agent's natural-language narration of the decision.
//!
//! ## Authority & invariants
//!
//! Only the registry authority (the Hermes agent keypair) may commit receipts.
//! Slots must be strictly monotonic — a receipt for slot `N` can only follow a
//! receipt for some slot `< N` — and the reported compute units may never exceed
//! the per-slot Sealevel budget cap of 48,000,000 CU.

use anchor_lang::prelude::*;

declare_id!("ATNjhU9qwjziAcnS5tcrPPFcrszB9RevcDs9MJkf6MH2");

/// Hard ceiling on the compute units a single Hermes-authored slot may report.
/// Mirrors the Sealevel per-block compute budget so a receipt can never claim a
/// block did more work than the runtime would have permitted.
pub const MAX_BLOCK_COMPUTE_UNITS: u64 = 48_000_000;

/// Length, in bytes, of a slot blockhash (a base58-encoded 32-byte digest).
pub const BLOCKHASH_LEN: usize = 32;

/// Length, in bytes, of the SHA-256 narration commitment.
pub const NARRATION_HASH_LEN: usize = 32;

#[program]
pub mod receipt_registry {
    use super::*;

    /// Initialize the singleton [`RegistryConfig`] and bind it to the Hermes
    /// agent `authority`. The payer funds rent; the authority is the only key
    /// that may subsequently commit receipts.
    ///
    /// `current_slot` starts at `0` and `total_receipts` at `0`. The first
    /// committed receipt must therefore be for a slot `> 0`.
    pub fn initialize(ctx: Context<Initialize>, authority: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = authority;
        config.current_slot = 0;
        config.total_receipts = 0;
        config.bump = ctx.bumps.config;

        msg!(
            "Hermes registry initialized · authority={} · config={}",
            authority,
            config.key()
        );
        Ok(())
    }

    /// Anchor a single per-slot decision receipt.
    ///
    /// # Arguments
    /// * `slot`           — the slot this receipt describes; must be strictly
    ///                      greater than [`RegistryConfig::current_slot`].
    /// * `blockhash`      — the 32-byte slot blockhash.
    /// * `txns`           — number of transactions the agent packed into the block.
    /// * `compute_units`  — total compute units consumed; capped at
    ///                      [`MAX_BLOCK_COMPUTE_UNITS`].
    /// * `narration_hash` — SHA-256 commitment to the agent's narration text.
    ///
    /// # Errors
    /// * [`RegistryError::Unauthorized`]         — signer is not the registry authority.
    /// * [`RegistryError::NonMonotonicSlot`]     — `slot <= current_slot`.
    /// * [`RegistryError::ComputeBudgetExceeded`] — `compute_units > MAX_BLOCK_COMPUTE_UNITS`.
    pub fn commit_receipt(
        ctx: Context<CommitReceipt>,
        slot: u64,
        blockhash: [u8; BLOCKHASH_LEN],
        txns: u32,
        compute_units: u64,
        narration_hash: [u8; NARRATION_HASH_LEN],
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;

        // Only the bound Hermes agent authority may write receipts.
        require_keys_eq!(
            ctx.accounts.authority.key(),
            config.authority,
            RegistryError::Unauthorized
        );

        // Slots are append-only and strictly increasing.
        require!(slot > config.current_slot, RegistryError::NonMonotonicSlot);

        // A receipt may never claim more work than the Sealevel block budget.
        require!(
            compute_units <= MAX_BLOCK_COMPUTE_UNITS,
            RegistryError::ComputeBudgetExceeded
        );

        let receipt = &mut ctx.accounts.receipt;
        receipt.slot = slot;
        receipt.blockhash = blockhash;
        receipt.txns = txns;
        receipt.compute_units = compute_units;
        receipt.narration_hash = narration_hash;
        receipt.authority = config.authority;
        receipt.bump = ctx.bumps.receipt;

        // Advance registry watermark.
        config.current_slot = slot;
        config.total_receipts = config
            .total_receipts
            .checked_add(1)
            .ok_or(RegistryError::CounterOverflow)?;

        emit!(ReceiptCommitted {
            slot,
            blockhash,
            txns,
            compute_units,
            narration_hash,
            total_receipts: config.total_receipts,
        });

        msg!(
            "Hermes receipt committed · slot={} · txns={} · cu={} · total={}",
            slot,
            txns,
            compute_units,
            config.total_receipts
        );
        Ok(())
    }
}

/// Accounts for [`receipt_registry::initialize`].
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Singleton registry config PDA.
    #[account(
        init,
        payer = payer,
        space = RegistryConfig::SPACE,
        seeds = [RegistryConfig::SEED],
        bump
    )]
    pub config: Account<'info, RegistryConfig>,

    /// Rent payer for the config account.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Accounts for [`receipt_registry::commit_receipt`].
#[derive(Accounts)]
#[instruction(slot: u64)]
pub struct CommitReceipt<'info> {
    /// Registry config; mutated to advance the slot watermark and counter.
    #[account(
        mut,
        seeds = [RegistryConfig::SEED],
        bump = config.bump
    )]
    pub config: Account<'info, RegistryConfig>,

    /// Per-slot receipt PDA, created on commit. The slot is baked into the seed
    /// so each slot can be anchored at most once.
    #[account(
        init,
        payer = authority,
        space = Receipt::SPACE,
        seeds = [Receipt::SEED, slot.to_le_bytes().as_ref()],
        bump
    )]
    pub receipt: Account<'info, Receipt>,

    /// The Hermes agent authority. Must match `config.authority`; also funds the
    /// per-slot receipt rent.
    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Singleton registry configuration — one per program deployment.
#[account]
pub struct RegistryConfig {
    /// The Hermes agent pubkey permitted to commit receipts.
    pub authority: Pubkey,
    /// Highest slot anchored so far. New receipts must exceed this.
    pub current_slot: u64,
    /// Total number of receipts committed over the registry's lifetime.
    pub total_receipts: u64,
    /// PDA bump for `seeds = [b"config"]`.
    pub bump: u8,
}

impl RegistryConfig {
    /// PDA seed prefix.
    pub const SEED: &'static [u8] = b"config";

    /// 8 (discriminator) + 32 (authority) + 8 (current_slot) + 8 (total_receipts) + 1 (bump).
    pub const SPACE: usize = 8 + 32 + 8 + 8 + 1;
}

/// A single anchored per-slot decision receipt.
#[account]
pub struct Receipt {
    /// The slot this receipt describes.
    pub slot: u64,
    /// The 32-byte slot blockhash.
    pub blockhash: [u8; BLOCKHASH_LEN],
    /// Number of transactions packed into the slot.
    pub txns: u32,
    /// Compute units consumed by the slot (<= MAX_BLOCK_COMPUTE_UNITS).
    pub compute_units: u64,
    /// SHA-256 commitment to the agent's narration of the decision.
    pub narration_hash: [u8; NARRATION_HASH_LEN],
    /// The authority that committed this receipt (snapshot of config.authority).
    pub authority: Pubkey,
    /// PDA bump for `seeds = [b"receipt", slot]`.
    pub bump: u8,
}

impl Receipt {
    /// PDA seed prefix.
    pub const SEED: &'static [u8] = b"receipt";

    /// 8 (discriminator) + 8 (slot) + 32 (blockhash) + 4 (txns)
    /// + 8 (compute_units) + 32 (narration_hash) + 32 (authority) + 1 (bump).
    pub const SPACE: usize = 8 + 8 + BLOCKHASH_LEN + 4 + 8 + NARRATION_HASH_LEN + 32 + 1;
}

/// Emitted on every successful [`receipt_registry::commit_receipt`].
#[event]
pub struct ReceiptCommitted {
    pub slot: u64,
    pub blockhash: [u8; BLOCKHASH_LEN],
    pub txns: u32,
    pub compute_units: u64,
    pub narration_hash: [u8; NARRATION_HASH_LEN],
    pub total_receipts: u64,
}

/// Program error space for the receipt registry.
#[error_code]
pub enum RegistryError {
    #[msg("Signer is not the registry authority (the Hermes agent).")]
    Unauthorized,
    #[msg("Slot is not strictly greater than the current registry slot.")]
    NonMonotonicSlot,
    #[msg("Reported compute units exceed the per-slot Sealevel budget cap.")]
    ComputeBudgetExceeded,
    #[msg("Receipt counter overflowed.")]
    CounterOverflow,
}

// reviewed 2026-05-28
