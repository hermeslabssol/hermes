//! Tower-BFT vote tracking — **work in progress**.
//!
//! > ⚠️ **Status: WIP stub.** The types and lockout arithmetic below are
//! > implemented and unit-tested in isolation, but they are **not yet wired
//! > into finality** — [`SlotProducer`](crate::SlotProducer) does not consult a
//! > [`Tower`] when sealing, and no fork-choice rule reads these votes. This
//! > module exists so the consensus surface is honest about where the chain is
//! > headed (full Tower-BFT) without pretending it is already there.
//!
//! ## The model we are building toward
//!
//! Tower-BFT is Solana's BFT layer over Proof-of-History. Each validator
//! maintains a *tower* of votes. A new vote on a slot has an exponentially
//! growing **lockout**: vote `n` slots deep in the tower locks the validator out
//! of voting on a conflicting fork for `2^(n+1)` slots. Casting a vote that
//! would violate an existing lockout is a **double-sign**, which the
//! [`SlashingEngine`](crate::SlashingEngine) punishes via
//! [`Offense::DoubleSign`](crate::Offense::DoubleSign).
//!
//! Roots are committed once a vote reaches the maximum lockout depth
//! ([`MAX_LOCKOUT_HISTORY`]); everything at or below the root is final.

use logios_primitives::Slot;

/// Maximum number of votes retained in a tower before the deepest is rooted.
/// Matches Solana's `MAX_LOCKOUT_HISTORY`.
pub const MAX_LOCKOUT_HISTORY: usize = 31;

/// The initial lockout (in slots) applied to a freshly cast vote.
pub const INITIAL_LOCKOUT: u64 = 2;

/// A single vote with its current confirmation count.
///
/// `confirmation_count` grows each time a later vote is stacked on top, which is
/// what drives the exponential lockout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lockout {
    /// The slot this vote is for.
    pub slot: Slot,
    /// How many votes have been stacked on top of this one (inclusive of self).
    pub confirmation_count: u32,
}

impl Lockout {
    /// Create a fresh lockout for `slot` with one confirmation.
    #[must_use]
    pub fn new(slot: Slot) -> Self {
        Self {
            slot,
            confirmation_count: 1,
        }
    }

    /// Lockout period in slots: `INITIAL_LOCKOUT ^ confirmation_count`.
    #[must_use]
    pub fn lockout_slots(&self) -> u64 {
        INITIAL_LOCKOUT.saturating_pow(self.confirmation_count)
    }

    /// The last slot (inclusive) at which this vote still locks the validator.
    #[must_use]
    pub fn expiration_slot(&self) -> Slot {
        Slot::new(self.slot.get() + self.lockout_slots())
    }

    /// True if a vote on `candidate` would be locked out by this entry, i.e.
    /// `candidate` falls within `(slot, expiration_slot]` on a different fork.
    #[must_use]
    pub fn is_locked_out_at(&self, candidate: Slot) -> bool {
        candidate.get() > self.slot.get() && candidate.get() <= self.expiration_slot().get()
    }
}

/// A validator's vote tower. **WIP** — see the module docs.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tower {
    /// Votes, oldest (deepest, about-to-root) first.
    votes: Vec<Lockout>,
    /// The most recently rooted slot, if any. Everything ≤ root is final.
    root: Option<Slot>,
}

impl Tower {
    /// An empty tower.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current votes, oldest first.
    #[must_use]
    pub fn votes(&self) -> &[Lockout] {
        &self.votes
    }

    /// The rooted (final) slot, if any.
    #[must_use]
    pub fn root(&self) -> Option<Slot> {
        self.root
    }

    /// Record a vote for `slot`.
    ///
    /// **WIP semantics:** this performs the tower's lockout bookkeeping —
    /// popping expired votes, incrementing confirmation counts on the surviving
    /// stack, pushing the new vote, and rooting the deepest entry once the tower
    /// is full. It does **not** yet reject lockout-violating votes (that hook is
    /// [`Tower::would_double_sign`]); fork-choice integration is pending.
    pub fn record_vote(&mut self, slot: Slot) {
        // Pop votes whose lockout has expired relative to the new slot.
        while let Some(last) = self.votes.last() {
            if last.expiration_slot().get() < slot.get() {
                self.votes.pop();
            } else {
                break;
            }
        }

        // Every surviving vote gains a confirmation.
        for v in self.votes.iter_mut() {
            v.confirmation_count += 1;
        }

        self.votes.push(Lockout::new(slot));

        // Root the deepest vote once the tower overflows.
        if self.votes.len() > MAX_LOCKOUT_HISTORY {
            let rooted = self.votes.remove(0);
            self.root = Some(rooted.slot);
        }
    }

    /// **WIP:** whether voting on `candidate` would violate an existing lockout
    /// on a *different* fork (i.e. constitute a double-sign). Fork identity is
    /// not yet modeled, so callers must currently supply only conflicting
    /// candidates; this returns whether any tower entry locks the slot.
    #[must_use]
    pub fn would_double_sign(&self, candidate: Slot) -> bool {
        self.votes.iter().any(|v| v.is_locked_out_at(candidate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockout_is_exponential() {
        let mut l = Lockout::new(Slot::new(10));
        assert_eq!(l.lockout_slots(), 2); // 2^1
        l.confirmation_count = 2;
        assert_eq!(l.lockout_slots(), 4); // 2^2
        l.confirmation_count = 5;
        assert_eq!(l.lockout_slots(), 32); // 2^5
    }

    #[test]
    fn locked_out_window_is_exclusive_lower_inclusive_upper() {
        let l = Lockout::new(Slot::new(10)); // lockout 2 -> expires at 12
        assert!(!l.is_locked_out_at(Slot::new(10)));
        assert!(l.is_locked_out_at(Slot::new(11)));
        assert!(l.is_locked_out_at(Slot::new(12)));
        assert!(!l.is_locked_out_at(Slot::new(13)));
    }

    #[test]
    fn stacking_votes_increments_confirmations() {
        let mut t = Tower::new();
        t.record_vote(Slot::new(1));
        t.record_vote(Slot::new(2));
        t.record_vote(Slot::new(3));
        // Oldest vote has been confirmed three times.
        assert_eq!(t.votes()[0].confirmation_count, 3);
        assert_eq!(t.votes().last().unwrap().confirmation_count, 1);
    }

    #[test]
    fn expired_votes_are_popped() {
        let mut t = Tower::new();
        t.record_vote(Slot::new(1)); // expires at 1 + 2 = 3
        // Vote far in the future; the slot-1 vote (exp 3) is expired at 100.
        t.record_vote(Slot::new(100));
        assert_eq!(t.votes().len(), 1);
        assert_eq!(t.votes()[0].slot, Slot::new(100));
    }

    #[test]
    fn tower_roots_when_full() {
        let mut t = Tower::new();
        // Vote on tightly packed slots so nothing expires; overflow -> root.
        for s in 1..=(MAX_LOCKOUT_HISTORY as u64 + 1) {
            t.record_vote(Slot::new(s));
        }
        assert_eq!(t.votes().len(), MAX_LOCKOUT_HISTORY);
        assert_eq!(t.root(), Some(Slot::new(1)));
    }

    #[test]
    fn would_double_sign_detects_locked_slot() {
        let mut t = Tower::new();
        t.record_vote(Slot::new(10)); // after push, conf=1, exp=12
        assert!(t.would_double_sign(Slot::new(11)));
        assert!(!t.would_double_sign(Slot::new(50)));
    }
}

// reviewed 2026-06-04
