//! The [`Receipt`] type: one signed decision receipt per sealed slot.

use logios_primitives::{ComputeUnits, Hash, Signature, Slot};

/// A signed summary of one slot authored by the Logios leader.
///
/// Receipts are what make Logios auditable in public: anyone can replay the
/// runtime over the slot's transactions and check that the reported counts and
/// compute usage match, then verify the [`signature`](Receipt::signature)
/// against the scheduled leader's pubkey.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Receipt {
    /// Slot height this receipt seals.
    pub slot: Slot,
    /// Base58 blockhash sealing the slot's entries.
    pub blockhash: Hash,
    /// Number of transactions included in the slot.
    pub txns: u32,
    /// Number of account writes applied while executing those transactions.
    pub account_writes: u32,
    /// Total compute units consumed, metered against the per-slot budget.
    pub compute_units: ComputeUnits,
    /// Short human-readable narration authored by the agent ("why this slot").
    pub narration: String,
    /// Ed25519 signature by the slot leader over the receipt payload.
    pub signature: Signature,
}

impl Receipt {
    /// Build an unsigned receipt (signature defaults to
    /// [`Signature::UNSIGNED`]). Call [`Receipt::sign_with`] once the leader has
    /// produced a signature over [`Receipt::signing_payload`].
    #[must_use]
    pub fn new(
        slot: Slot,
        blockhash: Hash,
        txns: u32,
        account_writes: u32,
        compute_units: ComputeUnits,
        narration: impl Into<String>,
    ) -> Self {
        Self {
            slot,
            blockhash,
            txns,
            account_writes,
            compute_units,
            narration: narration.into(),
            signature: Signature::UNSIGNED,
        }
    }

    /// The canonical byte payload a leader signs to seal this receipt.
    ///
    /// Field order is fixed and length-free fields are domain-separated, so the
    /// encoding is unambiguous and replayable by any verifier.
    #[must_use]
    pub fn signing_payload(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(64 + self.narration.len());
        buf.extend_from_slice(b"logios:receipt:v1");
        buf.extend_from_slice(&self.slot.get().to_le_bytes());
        buf.extend_from_slice(self.blockhash.as_bytes());
        buf.extend_from_slice(&self.txns.to_le_bytes());
        buf.extend_from_slice(&self.account_writes.to_le_bytes());
        buf.extend_from_slice(&self.compute_units.get().to_le_bytes());
        buf.extend_from_slice(&(self.narration.len() as u64).to_le_bytes());
        buf.extend_from_slice(self.narration.as_bytes());
        buf
    }

    /// Attach a signature, consuming and returning `self` for chaining.
    #[must_use]
    pub fn sign_with(mut self, signature: Signature) -> Self {
        self.signature = signature;
        self
    }

    /// True once a non-placeholder signature has been attached.
    #[must_use]
    pub fn is_signed(&self) -> bool {
        !self.signature.is_unsigned()
    }
}

impl core::fmt::Debug for Receipt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Receipt")
            .field("slot", &self.slot.get())
            .field("blockhash", &self.blockhash.to_base58())
            .field("txns", &self.txns)
            .field("account_writes", &self.account_writes)
            .field("compute_units", &self.compute_units.get())
            .field("narration", &self.narration)
            .field("signed", &self.is_signed())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Receipt {
        Receipt::new(
            Slot::new(42),
            Hash::hash(b"slot 42 entries"),
            3,
            5,
            ComputeUnits::new(120_000),
            "routine slot: 3 transfers, fees swept to treasury",
        )
    }

    #[test]
    fn unsigned_by_default() {
        assert!(!sample().is_signed());
    }

    #[test]
    fn signing_payload_is_deterministic_and_field_sensitive() {
        let r = sample();
        assert_eq!(r.signing_payload(), r.clone().signing_payload());

        let mut other = sample();
        other.txns = 4;
        assert_ne!(r.signing_payload(), other.signing_payload());
    }

    #[test]
    fn sign_with_marks_signed() {
        let r = sample().sign_with(Signature::new([1u8; 64]));
        assert!(r.is_signed());
    }
}
