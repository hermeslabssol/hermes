//! The Logios **slashing engine** — validator accountability as a state machine.
//!
//! Validators post stake (in [`Lamports`]) to participate in consensus. When a
//! validator misbehaves, a fraction of that stake is burned and the validator
//! may be *jailed* (temporarily removed from the leader schedule) or, after
//! enough infractions, *tombstoned* (permanently removed).
//!
//! ## Offenses
//!
//! | Offense                          | Severity | Auto-detected? |
//! |----------------------------------|----------|----------------|
//! | [`Offense::Downtime`]            | low      | yes (missed-slot window) |
//! | [`Offense::Equivocation`]        | high     | reported       |
//! | [`Offense::DoubleSign`]          | high     | reported       |
//! | [`Offense::InvalidStateTransition`] | critical | reported    |
//!
//! ## Guarantees the state machine enforces
//!
//! * **Cooldown** — the same validator cannot be slashed for the same offense
//!   class twice within [`SlashingParams::cooldown_epochs`]; replayed evidence
//!   is rejected as a no-op, not a double burn.
//! * **Jailing** — a slash that exceeds the jail policy moves the validator to
//!   [`ValidatorStatus::Jailed`] until a future epoch; it cannot lead until
//!   [`SlashingEngine::on_epoch`] advances past the release epoch and unjails it.
//! * **Tombstone** — once a validator accrues
//!   [`SlashingParams::tombstone_threshold`] infractions it becomes
//!   [`ValidatorStatus::Tombstoned`] and can never be unjailed.
//! * **Downtime** — missed slots are tracked in a sliding window; crossing the
//!   threshold auto-slashes a [`Offense::Downtime`].

use logios_primitives::{Epoch, Lamports, Pubkey};
use std::collections::HashMap;
use thiserror::Error;

/// A slashable offense, ordered loosely by severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Offense {
    /// Validator missed more than the allowed number of assigned slots inside
    /// the downtime window. Auto-detected.
    Downtime,
    /// Validator produced two different blocks for the same slot.
    Equivocation,
    /// Validator signed two conflicting votes / blocks (lockout violation).
    DoubleSign,
    /// Validator sealed a slot whose state transition does not replay — the
    /// most serious offense, since it attacks ledger integrity.
    InvalidStateTransition,
}

impl Offense {
    /// Default penalty in basis points for this offense, used when a caller
    /// does not override via [`SlashingParams`].
    #[must_use]
    pub fn default_bps(self, params: &SlashingParams) -> u16 {
        match self {
            Offense::Downtime => params.downtime_bps,
            Offense::Equivocation => params.equivocation_bps,
            Offense::DoubleSign => params.double_sign_bps,
            Offense::InvalidStateTransition => params.invalid_state_bps,
        }
    }

    /// Whether this offense, on its own, jails the validator.
    #[must_use]
    pub fn jails(self) -> bool {
        // Downtime is corrected by coming back online; the rest remove the
        // validator from production until reviewed.
        !matches!(self, Offense::Downtime)
    }
}

/// Tunable slashing policy (basis points: 1 bp = 0.01%).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlashingParams {
    /// Penalty for a double-sign, in basis points (e.g. 500 = 5%).
    pub double_sign_bps: u16,
    /// Penalty for equivocation, in basis points.
    pub equivocation_bps: u16,
    /// Penalty for an invalid state transition, in basis points.
    pub invalid_state_bps: u16,
    /// Penalty for crossing the downtime threshold, in basis points.
    pub downtime_bps: u16,
    /// Number of epochs a jailed validator stays jailed.
    pub jail_epochs: u64,
    /// Sliding window (in assigned slots) over which downtime is measured.
    pub downtime_window: u64,
    /// Missed-slot count within the window that triggers a downtime slash.
    pub downtime_threshold: u64,
    /// Number of infractions after which a validator is permanently tombstoned.
    pub tombstone_threshold: u32,
    /// Epochs that must pass before the *same* offense class can slash a
    /// validator again (replay / double-slash guard).
    pub cooldown_epochs: u64,
}

impl Default for SlashingParams {
    fn default() -> Self {
        // Conservative Solana-flavored defaults.
        Self {
            double_sign_bps: 500,        // 5%
            equivocation_bps: 500,       // 5%
            invalid_state_bps: 10_000,   // 100% — integrity attack
            downtime_bps: 10,            // 0.1%
            jail_epochs: 2,
            downtime_window: 1_000,
            downtime_threshold: 500,     // miss half the window
            tombstone_threshold: 3,
            cooldown_epochs: 1,
        }
    }
}

/// Lifecycle status of a registered validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidatorStatus {
    /// Active and eligible to lead.
    Active,
    /// Temporarily removed; the field is the epoch at which it may be unjailed.
    Jailed {
        /// Epoch at or after which [`SlashingEngine::on_epoch`] will unjail.
        until: Epoch,
    },
    /// Permanently removed. Terminal state.
    Tombstoned,
}

/// Per-validator accounting tracked by the engine.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidatorState {
    /// Remaining (un-slashed) stake.
    pub stake: Lamports,
    /// Total lamports burned across all slashes.
    pub slashed_total: Lamports,
    /// Current lifecycle status.
    pub status: ValidatorStatus,
    /// Count of infractions (drives tombstoning).
    pub infractions: u32,
    /// Missed assigned slots inside the current downtime window.
    pub missed_in_window: u64,
    /// Assigned slots observed in the current downtime window.
    pub window_slots: u64,
    /// Last epoch in which each offense class was slashed (cooldown guard).
    last_slashed: HashMap<Offense, Epoch>,
}

impl ValidatorState {
    fn new(stake: Lamports) -> Self {
        Self {
            stake,
            slashed_total: Lamports::ZERO,
            status: ValidatorStatus::Active,
            infractions: 0,
            missed_in_window: 0,
            window_slots: 0,
            last_slashed: HashMap::new(),
        }
    }

    /// True if currently jailed (not active, not tombstoned).
    #[must_use]
    pub fn is_jailed(&self) -> bool {
        matches!(self.status, ValidatorStatus::Jailed { .. })
    }

    /// True if permanently removed.
    #[must_use]
    pub fn is_tombstoned(&self) -> bool {
        matches!(self.status, ValidatorStatus::Tombstoned)
    }

    /// True if active and eligible to lead.
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self.status, ValidatorStatus::Active)
    }
}

/// The result of a [`SlashingEngine::slash`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashOutcome {
    /// Lamports burned by this slash.
    pub burned: Lamports,
    /// Whether this slash jailed the validator.
    pub jailed: bool,
    /// Whether this slash tombstoned the validator.
    pub tombstoned: bool,
    /// The validator's status after the slash.
    pub status: ValidatorStatus,
}

/// Errors from slashing-engine operations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SlashingError {
    /// The validator is not registered.
    #[error("unknown validator: {0}")]
    Unknown(Pubkey),
    /// The validator is already registered.
    #[error("validator already registered: {0}")]
    AlreadyRegistered(Pubkey),
    /// The offense was already slashed within the cooldown window; ignored to
    /// prevent a double burn from replayed evidence.
    #[error("offense {offense:?} for {who} is within cooldown until epoch {until}")]
    WithinCooldown {
        /// Validator addressed.
        who: Pubkey,
        /// Offense class that is on cooldown.
        offense: Offense,
        /// Epoch at which the cooldown lifts.
        until: u64,
    },
    /// The validator is tombstoned and cannot be acted on further.
    #[error("validator {0} is tombstoned")]
    Tombstoned(Pubkey),
}

/// The slashing engine: registry of validators plus the accountability policy.
#[derive(Debug, Clone)]
pub struct SlashingEngine {
    params: SlashingParams,
    validators: HashMap<Pubkey, ValidatorState>,
    current_epoch: Epoch,
}

impl SlashingEngine {
    /// Create an engine with `params`, starting at epoch 0.
    #[must_use]
    pub fn new(params: SlashingParams) -> Self {
        Self {
            params,
            validators: HashMap::new(),
            current_epoch: Epoch::ZERO,
        }
    }

    /// The engine's parameters.
    #[must_use]
    pub fn params(&self) -> &SlashingParams {
        &self.params
    }

    /// The current epoch.
    #[must_use]
    pub fn epoch(&self) -> Epoch {
        self.current_epoch
    }

    /// Register a validator with an initial `stake`.
    ///
    /// # Errors
    ///
    /// [`SlashingError::AlreadyRegistered`] if `who` is already known.
    pub fn register_validator(
        &mut self,
        who: Pubkey,
        stake: Lamports,
    ) -> Result<(), SlashingError> {
        if self.validators.contains_key(&who) {
            return Err(SlashingError::AlreadyRegistered(who));
        }
        self.validators.insert(who, ValidatorState::new(stake));
        Ok(())
    }

    /// Borrow a validator's state.
    #[must_use]
    pub fn validator(&self, who: &Pubkey) -> Option<&ValidatorState> {
        self.validators.get(who)
    }

    /// Record that `who` was the assigned leader for an observed slot but
    /// `produced` it (or not). Advances the downtime window and auto-slashes a
    /// [`Offense::Downtime`] if the miss threshold is crossed.
    ///
    /// Returns `Some(outcome)` if a downtime slash fired.
    ///
    /// # Errors
    ///
    /// [`SlashingError::Unknown`] if `who` is not registered.
    pub fn record_missed_slot(
        &mut self,
        who: &Pubkey,
        produced: bool,
    ) -> Result<Option<SlashOutcome>, SlashingError> {
        // Read window state without holding a borrow across the slash call.
        {
            let v = self
                .validators
                .get_mut(who)
                .ok_or(SlashingError::Unknown(*who))?;
            if v.is_tombstoned() {
                return Err(SlashingError::Tombstoned(*who));
            }
            v.window_slots += 1;
            if !produced {
                v.missed_in_window += 1;
            }
            // Roll the window when full, after evaluating the threshold below.
        }

        let (missed, window, full) = {
            let v = &self.validators[who];
            (
                v.missed_in_window,
                v.window_slots,
                v.window_slots >= self.params.downtime_window,
            )
        };

        let mut outcome = None;
        if missed >= self.params.downtime_threshold {
            // Crossed the threshold: auto-slash downtime, then reset the window
            // so we don't re-fire every subsequent missed slot.
            outcome = Some(self.slash(who, Offense::Downtime)?);
            if let Some(v) = self.validators.get_mut(who) {
                v.missed_in_window = 0;
                v.window_slots = 0;
            }
        } else if full {
            // Window elapsed without crossing threshold: reset the counters.
            if let Some(v) = self.validators.get_mut(who) {
                v.missed_in_window = 0;
                v.window_slots = 0;
            }
        }
        let _ = window;
        Ok(outcome)
    }

    /// Slash `who` for `offense`, burning stake per policy and updating status.
    ///
    /// # Errors
    ///
    /// * [`SlashingError::Unknown`] if `who` is not registered.
    /// * [`SlashingError::Tombstoned`] if `who` is already tombstoned.
    /// * [`SlashingError::WithinCooldown`] if the same offense class was slashed
    ///   within [`SlashingParams::cooldown_epochs`] (replay guard) — no burn.
    pub fn slash(
        &mut self,
        who: &Pubkey,
        offense: Offense,
    ) -> Result<SlashOutcome, SlashingError> {
        let params = self.params;
        let current = self.current_epoch;

        let v = self
            .validators
            .get_mut(who)
            .ok_or(SlashingError::Unknown(*who))?;

        if v.is_tombstoned() {
            return Err(SlashingError::Tombstoned(*who));
        }

        // Cooldown guard: reject a repeat slash of the same offense class while
        // still inside the cooldown window.
        if let Some(last) = v.last_slashed.get(&offense) {
            let lift = Epoch::new(last.get() + params.cooldown_epochs);
            if current.get() < lift.get() {
                return Err(SlashingError::WithinCooldown {
                    who: *who,
                    offense,
                    until: lift.get(),
                });
            }
        }

        // Burn the penalty.
        let bps = offense.default_bps(&params);
        let burned = v.stake.apply_bps(bps);
        v.stake = v.stake.saturating_sub(burned);
        v.slashed_total += burned;
        v.infractions += 1;
        v.last_slashed.insert(offense, current);

        // Determine status transition.
        let mut jailed = false;
        let mut tombstoned = false;

        if v.infractions >= params.tombstone_threshold {
            v.status = ValidatorStatus::Tombstoned;
            tombstoned = true;
        } else if offense.jails() {
            let until = Epoch::new(current.get() + params.jail_epochs);
            v.status = ValidatorStatus::Jailed { until };
            jailed = true;
        }

        Ok(SlashOutcome {
            burned,
            jailed,
            tombstoned,
            status: v.status,
        })
    }

    /// Advance to a new epoch, unjailing any validator whose jail term has
    /// elapsed. Tombstoned validators are never unjailed. Returns the set of
    /// validators that were unjailed.
    pub fn on_epoch(&mut self, epoch: Epoch) -> Vec<Pubkey> {
        self.current_epoch = epoch;
        let mut unjailed = Vec::new();
        for (key, v) in self.validators.iter_mut() {
            if let ValidatorStatus::Jailed { until } = v.status {
                if epoch.get() >= until.get() {
                    v.status = ValidatorStatus::Active;
                    unjailed.push(*key);
                }
            }
        }
        unjailed.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        unjailed
    }

    /// Manually unjail `who` (e.g. governance pardon). No-op error if the
    /// validator is tombstoned.
    ///
    /// # Errors
    ///
    /// [`SlashingError::Unknown`] / [`SlashingError::Tombstoned`].
    pub fn unjail(&mut self, who: &Pubkey) -> Result<(), SlashingError> {
        let v = self
            .validators
            .get_mut(who)
            .ok_or(SlashingError::Unknown(*who))?;
        if v.is_tombstoned() {
            return Err(SlashingError::Tombstoned(*who));
        }
        v.status = ValidatorStatus::Active;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> (SlashingEngine, Pubkey) {
        let mut e = SlashingEngine::new(SlashingParams::default());
        let v = Pubkey::new([1u8; 32]);
        e.register_validator(v, Lamports::new(1_000_000)).unwrap();
        (e, v)
    }

    #[test]
    fn double_sign_burns_five_percent_and_jails() {
        let (mut e, v) = engine();
        let out = e.slash(&v, Offense::DoubleSign).unwrap();
        // 5% of 1_000_000 = 50_000.
        assert_eq!(out.burned.get(), 50_000);
        assert!(out.jailed);
        assert!(!out.tombstoned);
        let state = e.validator(&v).unwrap();
        assert_eq!(state.stake.get(), 950_000);
        assert_eq!(state.slashed_total.get(), 50_000);
        assert!(state.is_jailed());
    }

    #[test]
    fn unjail_on_epoch_change() {
        let (mut e, v) = engine();
        e.slash(&v, Offense::DoubleSign).unwrap();
        assert!(e.validator(&v).unwrap().is_jailed());
        // jail_epochs default = 2, slashed at epoch 0 -> until epoch 2.
        let unjailed = e.on_epoch(Epoch::new(1));
        assert!(unjailed.is_empty());
        assert!(e.validator(&v).unwrap().is_jailed());
        let unjailed = e.on_epoch(Epoch::new(2));
        assert_eq!(unjailed, vec![v]);
        assert!(e.validator(&v).unwrap().is_active());
    }

    #[test]
    fn manual_unjail() {
        let (mut e, v) = engine();
        e.slash(&v, Offense::Equivocation).unwrap();
        assert!(e.validator(&v).unwrap().is_jailed());
        e.unjail(&v).unwrap();
        assert!(e.validator(&v).unwrap().is_active());
    }

    #[test]
    fn cooldown_prevents_double_slash() {
        let (mut e, v) = engine();
        let first = e.slash(&v, Offense::DoubleSign).unwrap();
        assert_eq!(first.burned.get(), 50_000);
        // Same offense, same epoch -> within cooldown, rejected, no extra burn.
        let err = e.slash(&v, Offense::DoubleSign).unwrap_err();
        assert!(matches!(err, SlashingError::WithinCooldown { .. }));
        assert_eq!(e.validator(&v).unwrap().stake.get(), 950_000);
        assert_eq!(e.validator(&v).unwrap().infractions, 1);
    }

    #[test]
    fn cooldown_lifts_after_window() {
        let mut e = SlashingEngine::new(SlashingParams {
            cooldown_epochs: 1,
            tombstone_threshold: 10, // keep alive across slashes
            ..SlashingParams::default()
        });
        let v = Pubkey::new([2u8; 32]);
        e.register_validator(v, Lamports::new(1_000_000)).unwrap();

        e.slash(&v, Offense::DoubleSign).unwrap(); // epoch 0
        e.on_epoch(Epoch::new(1)); // cooldown (1 epoch) now lifted
        let second = e.slash(&v, Offense::DoubleSign).unwrap();
        // 5% of remaining 950_000 = 47_500.
        assert_eq!(second.burned.get(), 47_500);
        assert_eq!(e.validator(&v).unwrap().infractions, 2);
    }

    #[test]
    fn downtime_auto_slashes_on_threshold() {
        let mut e = SlashingEngine::new(SlashingParams {
            downtime_window: 10,
            downtime_threshold: 3,
            ..SlashingParams::default()
        });
        let v = Pubkey::new([3u8; 32]);
        e.register_validator(v, Lamports::new(1_000_000)).unwrap();

        // Miss two slots: no slash yet.
        assert!(e.record_missed_slot(&v, false).unwrap().is_none());
        assert!(e.record_missed_slot(&v, false).unwrap().is_none());
        // Third miss crosses threshold -> downtime slash fires.
        let out = e.record_missed_slot(&v, false).unwrap();
        let out = out.expect("downtime slash should fire");
        // 0.1% of 1_000_000 = 1_000.
        assert_eq!(out.burned.get(), 1_000);
        // Downtime does not jail.
        assert!(!out.jailed);
        assert!(e.validator(&v).unwrap().is_active());
        // Window reset after the slash.
        assert_eq!(e.validator(&v).unwrap().missed_in_window, 0);
    }

    #[test]
    fn produced_slots_do_not_count_as_missed() {
        let mut e = SlashingEngine::new(SlashingParams {
            downtime_window: 10,
            downtime_threshold: 2,
            ..SlashingParams::default()
        });
        let v = Pubkey::new([4u8; 32]);
        e.register_validator(v, Lamports::new(1_000_000)).unwrap();
        for _ in 0..9 {
            assert!(e.record_missed_slot(&v, true).unwrap().is_none());
        }
        assert_eq!(e.validator(&v).unwrap().missed_in_window, 0);
    }

    #[test]
    fn tombstone_after_threshold_infractions() {
        let mut e = SlashingEngine::new(SlashingParams {
            tombstone_threshold: 3,
            cooldown_epochs: 0, // allow rapid distinct slashes
            ..SlashingParams::default()
        });
        let v = Pubkey::new([5u8; 32]);
        e.register_validator(v, Lamports::new(1_000_000)).unwrap();

        // Three high-severity infractions across cooldown-free epochs.
        e.slash(&v, Offense::DoubleSign).unwrap();
        let _ = e.on_epoch(Epoch::new(1));
        e.slash(&v, Offense::Equivocation).unwrap();
        let _ = e.on_epoch(Epoch::new(2));
        let out = e.slash(&v, Offense::InvalidStateTransition).unwrap();

        assert!(out.tombstoned);
        assert!(e.validator(&v).unwrap().is_tombstoned());
        // Tombstoned validators reject further action.
        assert!(matches!(
            e.slash(&v, Offense::DoubleSign),
            Err(SlashingError::Tombstoned(_))
        ));
        assert!(matches!(e.unjail(&v), Err(SlashingError::Tombstoned(_))));
        // on_epoch never resurrects a tombstoned validator.
        let _ = e.on_epoch(Epoch::new(100));
        assert!(e.validator(&v).unwrap().is_tombstoned());
    }

    #[test]
    fn invalid_state_transition_burns_full_stake() {
        let (mut e, v) = engine();
        let out = e.slash(&v, Offense::InvalidStateTransition).unwrap();
        assert_eq!(out.burned.get(), 1_000_000); // 100%
        assert_eq!(e.validator(&v).unwrap().stake.get(), 0);
    }

    #[test]
    fn unknown_validator_errors() {
        let mut e = SlashingEngine::new(SlashingParams::default());
        let ghost = Pubkey::new([9u8; 32]);
        assert!(matches!(
            e.slash(&ghost, Offense::DoubleSign),
            Err(SlashingError::Unknown(_))
        ));
    }

    #[test]
    fn double_register_errors() {
        let (mut e, v) = engine();
        assert!(matches!(
            e.register_validator(v, Lamports::new(1)),
            Err(SlashingError::AlreadyRegistered(_))
        ));
    }
}
