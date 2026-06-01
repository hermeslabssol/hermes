//! Scalar newtypes for the Hermes economic and timing model.
//!
//! These are thin wrappers around `u64` that make the SVM vocabulary explicit
//! in signatures: you cannot accidentally pass a [`Slot`] where [`Lamports`]
//! are expected.

use crate::{LAMPORTS_PER_SOL, MAX_COMPUTE_UNITS_PER_SLOT};
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// Generate a `u64` newtype with arithmetic, `Display`, and conversions.
macro_rules! u64_newtype {
    ($(#[$meta:meta])* $name:ident, $unit:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(pub u64);

        impl $name {
            #[doc = concat!("The zero ", $unit, " value.")]
            pub const ZERO: $name = $name(0);

            #[doc = concat!("Wrap a raw `u64` as ", $unit, ".")]
            #[must_use]
            pub const fn new(v: u64) -> Self {
                $name(v)
            }

            #[doc = "Return the underlying `u64`."]
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }

            #[doc = "Checked addition; `None` on overflow."]
            #[must_use]
            pub fn checked_add(self, rhs: Self) -> Option<Self> {
                self.0.checked_add(rhs.0).map($name)
            }

            #[doc = "Saturating subtraction; floors at zero."]
            #[must_use]
            pub fn saturating_sub(self, rhs: Self) -> Self {
                $name(self.0.saturating_sub(rhs.0))
            }
        }

        impl From<u64> for $name {
            fn from(v: u64) -> Self {
                $name(v)
            }
        }

        impl From<$name> for u64 {
            fn from(v: $name) -> u64 {
                v.0
            }
        }

        impl Add for $name {
            type Output = $name;
            fn add(self, rhs: Self) -> Self {
                $name(self.0 + rhs.0)
            }
        }

        impl AddAssign for $name {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl Sub for $name {
            type Output = $name;
            fn sub(self, rhs: Self) -> Self {
                $name(self.0 - rhs.0)
            }
        }

        impl SubAssign for $name {
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{} {}", self.0, $unit)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.0)
            }
        }
    };
}

u64_newtype!(
    /// A slot height — the monotonic unit of block production. The autonomous
    /// leader seals exactly one block per slot.
    Slot,
    "slot"
);

u64_newtype!(
    /// An epoch number — a fixed-length window of slots over which the leader
    /// schedule is fixed and validator jail accounting is settled.
    Epoch,
    "epoch"
);

u64_newtype!(
    /// Lamports — the indivisible unit of $LABS value. Priority fees are
    /// denominated in lamports.
    Lamports,
    "lamports"
);

u64_newtype!(
    /// Compute units — the Sealevel measure of execution cost, metered against
    /// the per-slot compute budget.
    ComputeUnits,
    "CU"
);

impl Lamports {
    /// Construct from a whole-token amount (multiplies by [`LAMPORTS_PER_SOL`]).
    #[must_use]
    pub const fn from_sol(whole: u64) -> Self {
        Lamports(whole * LAMPORTS_PER_SOL)
    }

    /// Whole-token portion of this lamport amount (integer division).
    #[must_use]
    pub const fn whole_tokens(self) -> u64 {
        self.0 / LAMPORTS_PER_SOL
    }

    /// Apply a basis-point penalty (1 bp = 1/10_000), rounding down. Used by
    /// the slashing engine.
    #[must_use]
    pub fn apply_bps(self, bps: u16) -> Lamports {
        // u128 intermediate keeps the product from overflowing for any u64.
        let penalized = (self.0 as u128) * (bps as u128) / 10_000u128;
        Lamports(penalized as u64)
    }
}

impl ComputeUnits {
    /// The per-slot ceiling, as a typed value.
    pub const SLOT_LIMIT: ComputeUnits = ComputeUnits(MAX_COMPUTE_UNITS_PER_SLOT);

    /// True if this amount fits within the per-slot compute budget.
    #[must_use]
    pub fn within_slot_budget(self) -> bool {
        self.0 <= MAX_COMPUTE_UNITS_PER_SLOT
    }
}

impl Epoch {
    /// The next epoch.
    #[must_use]
    pub const fn next(self) -> Epoch {
        Epoch(self.0 + 1)
    }
}

impl Slot {
    /// The next slot.
    #[must_use]
    pub const fn next(self) -> Slot {
        Slot(self.0 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_and_display() {
        let a = Lamports::new(1_000);
        let b = Lamports::new(500);
        assert_eq!((a + b).get(), 1_500);
        assert_eq!((a - b).get(), 500);
        assert_eq!(a.to_string(), "1000 lamports");
    }

    #[test]
    fn from_sol_and_back() {
        let l = Lamports::from_sol(3);
        assert_eq!(l.get(), 3_000_000_000);
        assert_eq!(l.whole_tokens(), 3);
    }

    #[test]
    fn bps_penalty_rounds_down() {
        // 5% of 1_000_000 lamports = 50_000.
        assert_eq!(Lamports::new(1_000_000).apply_bps(500).get(), 50_000);
        // 1 bp of 9_999 lamports = 0 (rounds down).
        assert_eq!(Lamports::new(9_999).apply_bps(1).get(), 0);
        // No overflow at the high end.
        assert_eq!(Lamports::new(u64::MAX).apply_bps(10_000).get(), u64::MAX);
    }

    #[test]
    fn compute_budget_boundary() {
        assert!(ComputeUnits::new(MAX_COMPUTE_UNITS_PER_SLOT).within_slot_budget());
        assert!(!ComputeUnits::new(MAX_COMPUTE_UNITS_PER_SLOT + 1).within_slot_budget());
        assert_eq!(ComputeUnits::SLOT_LIMIT.get(), 48_000_000);
    }

    #[test]
    fn slot_and_epoch_increment() {
        assert_eq!(Slot::new(41).next(), Slot::new(42));
        assert_eq!(Epoch::new(0).next(), Epoch::new(1));
    }

    #[test]
    fn checked_add_overflow() {
        assert!(Slot::new(u64::MAX).checked_add(Slot::new(1)).is_none());
        assert_eq!(Slot::new(1).checked_add(Slot::new(1)), Some(Slot::new(2)));
    }
}
