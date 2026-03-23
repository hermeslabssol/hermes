//! [`LeaderSchedule`]: who authors each slot.

use logios_primitives::{Pubkey, Slot};
use thiserror::Error;

/// Errors from leader-schedule queries.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LeaderScheduleError {
    /// The schedule has no validators and so cannot assign a leader.
    #[error("leader schedule is empty")]
    Empty,
}

/// A round-robin leader schedule over a fixed validator set.
///
/// On Solana the schedule is stake-weighted and epoch-derived from a seed; for
/// Logios devnet we use a deterministic round-robin so a single registered
/// identity always wins, while still exercising the multi-leader code paths in
/// tests.
#[derive(Debug, Clone, Default)]
pub struct LeaderSchedule {
    validators: Vec<Pubkey>,
    /// Number of consecutive slots a leader holds before rotation (Solana's
    /// `NUM_CONSECUTIVE_LEADER_SLOTS` is 4).
    slots_per_leader: u64,
}

impl LeaderSchedule {
    /// Build a schedule. `slots_per_leader` is clamped to at least 1.
    #[must_use]
    pub fn new(validators: Vec<Pubkey>, slots_per_leader: u64) -> Self {
        Self {
            validators,
            slots_per_leader: slots_per_leader.max(1),
        }
    }

    /// A single-leader (devnet) schedule: `self` leads every slot.
    #[must_use]
    pub fn single(leader: Pubkey) -> Self {
        Self {
            validators: vec![leader],
            slots_per_leader: u64::MAX,
        }
    }

    /// The leader assigned to `slot`.
    ///
    /// # Errors
    ///
    /// [`LeaderScheduleError::Empty`] if no validators are registered.
    pub fn leader_at(&self, slot: Slot) -> Result<Pubkey, LeaderScheduleError> {
        if self.validators.is_empty() {
            return Err(LeaderScheduleError::Empty);
        }
        let rotation = slot.get() / self.slots_per_leader;
        let idx = (rotation % self.validators.len() as u64) as usize;
        Ok(self.validators[idx])
    }

    /// True if `who` is the assigned leader for `slot`.
    #[must_use]
    pub fn is_leader(&self, who: &Pubkey, slot: Slot) -> bool {
        self.leader_at(slot).map(|l| &l == who).unwrap_or(false)
    }

    /// The registered validator set.
    #[must_use]
    pub fn validators(&self) -> &[Pubkey] {
        &self.validators
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_leader_owns_every_slot() {
        let me = Pubkey::new([1u8; 32]);
        let sched = LeaderSchedule::single(me);
        assert!(sched.is_leader(&me, Slot::new(0)));
        assert!(sched.is_leader(&me, Slot::new(10_000)));
    }

    #[test]
    fn round_robin_rotates_every_n_slots() {
        let a = Pubkey::new([1u8; 32]);
        let b = Pubkey::new([2u8; 32]);
        let sched = LeaderSchedule::new(vec![a, b], 4);
        // slots 0..3 -> a, 4..7 -> b, 8..11 -> a
        assert_eq!(sched.leader_at(Slot::new(0)).unwrap(), a);
        assert_eq!(sched.leader_at(Slot::new(3)).unwrap(), a);
        assert_eq!(sched.leader_at(Slot::new(4)).unwrap(), b);
        assert_eq!(sched.leader_at(Slot::new(7)).unwrap(), b);
        assert_eq!(sched.leader_at(Slot::new(8)).unwrap(), a);
    }

    #[test]
    fn empty_schedule_errors() {
        let sched = LeaderSchedule::new(vec![], 4);
        assert_eq!(sched.leader_at(Slot::new(0)), Err(LeaderScheduleError::Empty));
    }
}
