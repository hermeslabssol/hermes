//! [`ComputeBudget`]: the Sealevel compute meter.

use crate::error::RuntimeError;
use logios_primitives::{ComputeUnits, MAX_COMPUTE_UNITS_PER_SLOT};

/// A compute-unit meter shared across a transaction's instructions.
///
/// Programs call [`ComputeBudget::consume`] as they work. When cumulative
/// consumption would exceed `limit`, the call fails with
/// [`RuntimeError::ComputeBudgetExceeded`] and nothing is charged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputeBudget {
    limit: u64,
    consumed: u64,
}

impl ComputeBudget {
    /// Create a budget with an explicit CU `limit`. The limit is clamped to the
    /// per-slot ceiling ([`MAX_COMPUTE_UNITS_PER_SLOT`]).
    #[must_use]
    pub fn new(limit: ComputeUnits) -> Self {
        Self {
            limit: limit.get().min(MAX_COMPUTE_UNITS_PER_SLOT),
            consumed: 0,
        }
    }

    /// A budget sized at the full per-slot ceiling.
    #[must_use]
    pub fn slot_max() -> Self {
        Self {
            limit: MAX_COMPUTE_UNITS_PER_SLOT,
            consumed: 0,
        }
    }

    /// Charge `cu` compute units.
    ///
    /// # Errors
    ///
    /// [`RuntimeError::ComputeBudgetExceeded`] if the charge would push
    /// cumulative consumption past the limit. On error, nothing is charged.
    pub fn consume(&mut self, cu: ComputeUnits) -> Result<(), RuntimeError> {
        let cu = cu.get();
        match self.consumed.checked_add(cu) {
            Some(next) if next <= self.limit => {
                self.consumed = next;
                Ok(())
            }
            _ => Err(RuntimeError::ComputeBudgetExceeded {
                requested: cu,
                remaining: self.remaining().get(),
            }),
        }
    }

    /// Total CU consumed so far.
    #[must_use]
    pub fn consumed(&self) -> ComputeUnits {
        ComputeUnits::new(self.consumed)
    }

    /// CU still available before the limit.
    #[must_use]
    pub fn remaining(&self) -> ComputeUnits {
        ComputeUnits::new(self.limit.saturating_sub(self.consumed))
    }

    /// The configured limit.
    #[must_use]
    pub fn limit(&self) -> ComputeUnits {
        ComputeUnits::new(self.limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consume_within_budget() {
        let mut b = ComputeBudget::new(ComputeUnits::new(1_000));
        b.consume(ComputeUnits::new(400)).unwrap();
        b.consume(ComputeUnits::new(600)).unwrap();
        assert_eq!(b.consumed().get(), 1_000);
        assert_eq!(b.remaining().get(), 0);
    }

    #[test]
    fn exceeding_budget_errors_and_charges_nothing() {
        let mut b = ComputeBudget::new(ComputeUnits::new(1_000));
        b.consume(ComputeUnits::new(900)).unwrap();
        let err = b.consume(ComputeUnits::new(200)).unwrap_err();
        assert_eq!(
            err,
            RuntimeError::ComputeBudgetExceeded {
                requested: 200,
                remaining: 100
            }
        );
        // The failed charge did not move the meter.
        assert_eq!(b.consumed().get(), 900);
    }

    #[test]
    fn limit_clamped_to_slot_ceiling() {
        let b = ComputeBudget::new(ComputeUnits::new(MAX_COMPUTE_UNITS_PER_SLOT + 5_000));
        assert_eq!(b.limit().get(), MAX_COMPUTE_UNITS_PER_SLOT);
    }

    #[test]
    fn overflow_is_treated_as_exceeded() {
        let mut b = ComputeBudget::slot_max();
        b.consume(ComputeUnits::new(1)).unwrap();
        let err = b.consume(ComputeUnits::new(u64::MAX)).unwrap_err();
        assert!(matches!(err, RuntimeError::ComputeBudgetExceeded { .. }));
    }
}

// reviewed 2026-05-21
