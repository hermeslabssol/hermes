//! # hermes-faucet
//!
//! Testnet faucet for the **$HERMES** SPL token — Hermes Labs / Hermes.
//!
//! Anyone can call [`hermes_faucet::drip`] to receive a capped amount of
//! testnet $HERMES into their associated token account, subject to a
//! per-wallet cooldown. The faucet mint authority is a PDA owned by this
//! program, so the program itself signs the `mint_to` CPI into the SPL Token
//! program — no off-chain signer required.
//!
//! ## Account model
//!
//! - [`FaucetConfig`] — singleton PDA (`seeds = [b"faucet"]`) holding the
//!   $HERMES mint, the per-drip cap, and the cooldown length in slots. The
//!   faucet mint-authority PDA is derived as `seeds = [b"mint_auth"]`.
//! - [`Claim`] — one PDA per wallet (`seeds = [b"claim", wallet]`) recording the
//!   slot of that wallet's last successful drip, used to enforce the cooldown.
//!
//! ## Cooldown
//!
//! Cooldown is measured in slots. On a ~400ms slot time, a 30-second cooldown is
//! ~75 slots; the exact value is stored in `FaucetConfig::cooldown_slots` at
//! initialization. A wallet may drip again only once
//! `current_slot >= last_claim_slot + cooldown_slots`.

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};

declare_id!("AvPqHdw2HCY8RAEHVqpRiE98SaQENAuj6xjnH8nvWbF2");

/// Default cooldown: 30 seconds expressed in slots, assuming ~400ms per slot.
/// `30_000ms / 400ms = 75` slots.
pub const DEFAULT_COOLDOWN_SLOTS: u64 = 75;

/// Maximum $HERMES a single drip may mint (in base units / lamports of the
/// token's smallest denomination). Guards against draining the testnet supply.
pub const MAX_DRIP_AMOUNT: u64 = 1_000_000_000;

#[program]
pub mod hermes_faucet {
    use super::*;

    /// Initialize the faucet config for a given $HERMES `mint`.
    ///
    /// The mint authority of `mint` is expected to be the program's
    /// `mint_authority` PDA (`seeds = [b"mint_auth"]`) so that [`drip`] can sign
    /// the `mint_to` CPI.
    ///
    /// `cooldown_slots` of `0` falls back to [`DEFAULT_COOLDOWN_SLOTS`].
    pub fn initialize(ctx: Context<InitializeFaucet>, cooldown_slots: u64) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.mint = ctx.accounts.mint.key();
        config.authority = ctx.accounts.authority.key();
        config.cooldown_slots = if cooldown_slots == 0 {
            DEFAULT_COOLDOWN_SLOTS
        } else {
            cooldown_slots
        };
        config.mint_auth_bump = ctx.bumps.mint_authority;
        config.bump = ctx.bumps.config;

        msg!(
            "Hermes faucet initialized · mint={} · cooldown_slots={}",
            config.mint,
            config.cooldown_slots
        );
        Ok(())
    }

    /// Drip `amount` testnet $HERMES to the caller, enforcing the per-wallet
    /// cooldown.
    ///
    /// On first claim the [`Claim`] PDA is created; on subsequent claims it must
    /// already exist and its `last_claim_slot` must be old enough.
    ///
    /// # Errors
    /// * [`FaucetError::InvalidAmount`] — `amount == 0` or `> MAX_DRIP_AMOUNT`.
    /// * [`FaucetError::CooldownActive`] — cooldown has not yet elapsed.
    pub fn drip(ctx: Context<Drip>, amount: u64) -> Result<()> {
        require!(
            amount > 0 && amount <= MAX_DRIP_AMOUNT,
            FaucetError::InvalidAmount
        );

        let config = &ctx.accounts.config;
        let claim = &mut ctx.accounts.claim;
        let current_slot = Clock::get()?.slot;

        // `last_claim_slot == 0` means this is a freshly-created Claim PDA, so
        // the cooldown check is skipped on the very first drip.
        if claim.last_claim_slot != 0 {
            let ready_slot = claim
                .last_claim_slot
                .checked_add(config.cooldown_slots)
                .ok_or(FaucetError::ArithmeticOverflow)?;
            require!(current_slot >= ready_slot, FaucetError::CooldownActive);
        }

        // Sign the mint_to CPI with the faucet mint-authority PDA.
        let mint_auth_seeds: &[&[u8]] = &[b"mint_auth", &[config.mint_auth_bump]];
        let signer_seeds = &[mint_auth_seeds];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.recipient_token.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::mint_to(cpi_ctx, amount)?;

        // Record claim state.
        claim.wallet = ctx.accounts.wallet.key();
        claim.last_claim_slot = current_slot;
        claim.bump = ctx.bumps.claim;

        emit!(Dripped {
            wallet: ctx.accounts.wallet.key(),
            amount,
            slot: current_slot,
        });

        msg!(
            "Hermes drip · wallet={} · amount={} · slot={}",
            ctx.accounts.wallet.key(),
            amount,
            current_slot
        );
        Ok(())
    }
}

/// Accounts for [`hermes_faucet::initialize`].
#[derive(Accounts)]
pub struct InitializeFaucet<'info> {
    /// Singleton faucet config PDA.
    #[account(
        init,
        payer = authority,
        space = FaucetConfig::SPACE,
        seeds = [FaucetConfig::SEED],
        bump
    )]
    pub config: Account<'info, FaucetConfig>,

    /// The $HERMES mint this faucet dispenses. Its mint authority should be the
    /// `mint_authority` PDA below.
    pub mint: Account<'info, Mint>,

    /// CHECK: PDA used purely as the mint authority signer for CPIs; never read
    /// or written as data. Derived as `seeds = [b"mint_auth"]`.
    #[account(seeds = [b"mint_auth"], bump)]
    pub mint_authority: UncheckedAccount<'info>,

    /// Faucet admin / rent payer.
    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Accounts for [`hermes_faucet::drip`].
#[derive(Accounts)]
pub struct Drip<'info> {
    #[account(
        seeds = [FaucetConfig::SEED],
        bump = config.bump,
        has_one = mint
    )]
    pub config: Account<'info, FaucetConfig>,

    /// The $HERMES mint. Must match `config.mint`.
    #[account(mut)]
    pub mint: Account<'info, Mint>,

    /// CHECK: faucet mint-authority PDA; signs the mint_to CPI. Bump validated
    /// against `config.mint_auth_bump`.
    #[account(seeds = [b"mint_auth"], bump = config.mint_auth_bump)]
    pub mint_authority: UncheckedAccount<'info>,

    /// Per-wallet cooldown record, created on first drip.
    #[account(
        init_if_needed,
        payer = wallet,
        space = Claim::SPACE,
        seeds = [Claim::SEED, wallet.key().as_ref()],
        bump
    )]
    pub claim: Account<'info, Claim>,

    /// Destination token account; must belong to `wallet` and be for `mint`.
    #[account(
        mut,
        constraint = recipient_token.mint == mint.key() @ FaucetError::MintMismatch,
        constraint = recipient_token.owner == wallet.key() @ FaucetError::OwnerMismatch
    )]
    pub recipient_token: Account<'info, TokenAccount>,

    /// The claiming wallet; pays rent for its `Claim` PDA on first drip.
    #[account(mut)]
    pub wallet: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// Singleton faucet configuration.
#[account]
pub struct FaucetConfig {
    /// The $HERMES mint dispensed by this faucet.
    pub mint: Pubkey,
    /// Faucet admin authority.
    pub authority: Pubkey,
    /// Per-wallet cooldown, in slots.
    pub cooldown_slots: u64,
    /// Bump for the `mint_authority` PDA (`seeds = [b"mint_auth"]`).
    pub mint_auth_bump: u8,
    /// Bump for this config PDA (`seeds = [b"faucet"]`).
    pub bump: u8,
}

impl FaucetConfig {
    pub const SEED: &'static [u8] = b"faucet";
    /// 8 (disc) + 32 (mint) + 32 (authority) + 8 (cooldown_slots)
    /// + 1 (mint_auth_bump) + 1 (bump).
    pub const SPACE: usize = 8 + 32 + 32 + 8 + 1 + 1;
}

/// Per-wallet cooldown record.
#[account]
pub struct Claim {
    /// The wallet this claim record belongs to.
    pub wallet: Pubkey,
    /// Slot of the wallet's most recent successful drip.
    pub last_claim_slot: u64,
    /// PDA bump for `seeds = [b"claim", wallet]`.
    pub bump: u8,
}

impl Claim {
    pub const SEED: &'static [u8] = b"claim";
    /// 8 (disc) + 32 (wallet) + 8 (last_claim_slot) + 1 (bump).
    pub const SPACE: usize = 8 + 32 + 8 + 1;
}

/// Emitted on every successful [`hermes_faucet::drip`].
#[event]
pub struct Dripped {
    pub wallet: Pubkey,
    pub amount: u64,
    pub slot: u64,
}

/// Program error space for the faucet.
#[error_code]
pub enum FaucetError {
    #[msg("Cooldown has not yet elapsed for this wallet.")]
    CooldownActive,
    #[msg("Drip amount must be non-zero and within the per-drip cap.")]
    InvalidAmount,
    #[msg("Recipient token account mint does not match the faucet mint.")]
    MintMismatch,
    #[msg("Recipient token account owner does not match the claiming wallet.")]
    OwnerMismatch,
    #[msg("Arithmetic overflow computing cooldown window.")]
    ArithmeticOverflow,
}
