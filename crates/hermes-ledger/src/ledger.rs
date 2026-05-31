//! The [`Ledger`]: an append-only, slot-indexed store of [`Receipt`]s.

use crate::receipt::Receipt;
use hermes_primitives::Slot;
use std::collections::BTreeMap;
use thiserror::Error;

/// Errors returned by [`Ledger`] mutation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LedgerError {
    /// A receipt was pushed for a slot at or below the highest stored slot.
    /// The ledger is strictly append-only and forward-only.
    #[error("non-monotonic push: slot {pushed} <= tip {tip}")]
    NonMonotonic {
        /// Slot of the rejected receipt.
        pushed: u64,
        /// Current ledger tip.
        tip: u64,
    },
    /// A receipt for this slot already exists.
    #[error("duplicate receipt for slot {0}")]
    Duplicate(u64),
}

/// An append-only ledger of decision receipts, keyed by slot.
///
/// Internally a [`BTreeMap`] keeps receipts ordered by slot so range scans and
/// pruning are cheap. Pushes must be strictly increasing in slot height,
/// reflecting the single-leader, one-block-per-slot production model.
#[derive(Debug, Default, Clone)]
pub struct Ledger {
    receipts: BTreeMap<u64, Receipt>,
    /// Highest slot ever pushed (survives pruning, so monotonicity holds even
    /// after the receipt itself is pruned away).
    tip: Option<u64>,
}

impl Ledger {
    /// Create an empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a receipt.
    ///
    /// # Errors
    ///
    /// * [`LedgerError::NonMonotonic`] if `receipt.slot` is not strictly greater
    ///   than the current tip.
    /// * [`LedgerError::Duplicate`] if a receipt already exists for the slot
    ///   (only reachable when the slot is below the tip *and* still resident,
    ///   which the monotonic guard already excludes — kept for completeness on
    ///   manual inserts).
    pub fn push(&mut self, receipt: Receipt) -> Result<(), LedgerError> {
        let slot = receipt.slot.get();
        if let Some(tip) = self.tip {
            if slot <= tip {
                return Err(LedgerError::NonMonotonic { pushed: slot, tip });
            }
        }
        if self.receipts.contains_key(&slot) {
            return Err(LedgerError::Duplicate(slot));
        }
        self.receipts.insert(slot, receipt);
        self.tip = Some(slot);
        Ok(())
    }

    /// Fetch the receipt sealing `slot`, if resident.
    #[must_use]
    pub fn get(&self, slot: Slot) -> Option<&Receipt> {
        self.receipts.get(&slot.get())
    }

    /// Iterate receipts whose slot falls in `[start, end)`, in ascending slot
    /// order.
    pub fn range(
        &self,
        start: Slot,
        end: Slot,
    ) -> impl Iterator<Item = &Receipt> {
        self.receipts
            .range(start.get()..end.get())
            .map(|(_, r)| r)
    }

    /// The most recently pushed (and still resident) receipt.
    #[must_use]
    pub fn latest(&self) -> Option<&Receipt> {
        self.receipts.values().next_back()
    }

    /// Highest slot ever accepted, regardless of pruning.
    #[must_use]
    pub fn tip(&self) -> Option<Slot> {
        self.tip.map(Slot::new)
    }

    /// Number of resident receipts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.receipts.len()
    }

    /// True if no receipts are resident.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }

    /// Drop every receipt with slot strictly less than `watermark`, returning
    /// the number pruned. Bounds memory on long-running nodes; the
    /// monotonicity guard still uses the preserved [`tip`](Ledger::tip).
    pub fn prune_below(&mut self, watermark: Slot) -> usize {
        let before = self.receipts.len();
        // BTreeMap::split_off keeps keys >= watermark.
        self.receipts = self.receipts.split_off(&watermark.get());
        before - self.receipts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_primitives::{ComputeUnits, Hash};

    fn receipt(slot: u64) -> Receipt {
        Receipt::new(
            Slot::new(slot),
            Hash::hash(format!("slot {slot}").as_bytes()),
            1,
            2,
            ComputeUnits::new(5_000),
            format!("slot {slot} narration"),
        )
    }

    #[test]
    fn push_and_get() {
        let mut l = Ledger::new();
        l.push(receipt(10)).unwrap();
        l.push(receipt(11)).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l.get(Slot::new(10)).unwrap().slot, Slot::new(10));
        assert!(l.get(Slot::new(99)).is_none());
        assert_eq!(l.latest().unwrap().slot, Slot::new(11));
        assert_eq!(l.tip(), Some(Slot::new(11)));
    }

    #[test]
    fn rejects_non_monotonic() {
        let mut l = Ledger::new();
        l.push(receipt(10)).unwrap();
        let err = l.push(receipt(10)).unwrap_err();
        assert_eq!(err, LedgerError::NonMonotonic { pushed: 10, tip: 10 });
        let err = l.push(receipt(5)).unwrap_err();
        assert_eq!(err, LedgerError::NonMonotonic { pushed: 5, tip: 10 });
    }

    #[test]
    fn range_is_half_open_and_ordered() {
        let mut l = Ledger::new();
        for s in 1..=5 {
            l.push(receipt(s)).unwrap();
        }
        let slots: Vec<u64> = l
            .range(Slot::new(2), Slot::new(5))
            .map(|r| r.slot.get())
            .collect();
        assert_eq!(slots, vec![2, 3, 4]);
    }

    #[test]
    fn prune_below_drops_old_and_preserves_tip() {
        let mut l = Ledger::new();
        for s in 1..=10 {
            l.push(receipt(s)).unwrap();
        }
        let pruned = l.prune_below(Slot::new(8));
        assert_eq!(pruned, 7);
        assert_eq!(l.len(), 3);
        assert!(l.get(Slot::new(7)).is_none());
        assert!(l.get(Slot::new(8)).is_some());
        // Tip survives pruning, so monotonicity still holds.
        assert_eq!(l.tip(), Some(Slot::new(10)));
        let err = l.push(receipt(9)).unwrap_err();
        assert_eq!(err, LedgerError::NonMonotonic { pushed: 9, tip: 10 });
    }

    #[test]
    fn empty_ledger_state() {
        let l = Ledger::new();
        assert!(l.is_empty());
        assert!(l.latest().is_none());
        assert!(l.tip().is_none());
    }
}

// reviewed 2026-05-25
