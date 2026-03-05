//! [`Signature`] — a 64-byte Ed25519 signature, displayed as base58.

use crate::base58::{decode_base58_array, encode_base58, Base58Error};
use crate::SIGNATURE_BYTES;
use core::fmt;
use core::str::FromStr;
use thiserror::Error;

/// A 64-byte Ed25519 signature.
///
/// Every Logios decision receipt is signed by the autonomous leader; the
/// resulting `Signature` is the on-ledger proof that the slot was authored by
/// the scheduled identity. Like transaction signatures on Solana, it doubles as
/// a unique id and is rendered in base58.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Signature(#[cfg_attr(feature = "serde", serde(with = "serde_bytes64"))] [u8; SIGNATURE_BYTES]);

/// Error parsing a [`Signature`] from a base58 string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid signature: {0}")]
pub struct ParseSignatureError(#[from] Base58Error);

impl Signature {
    /// The all-zero signature (an unsigned / placeholder receipt).
    pub const UNSIGNED: Signature = Signature([0u8; SIGNATURE_BYTES]);

    /// Construct from raw bytes.
    #[must_use]
    pub const fn new(bytes: [u8; SIGNATURE_BYTES]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw 64-byte representation.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; SIGNATURE_BYTES] {
        &self.0
    }

    /// Render as base58.
    #[must_use]
    pub fn to_base58(&self) -> String {
        encode_base58(&self.0)
    }

    /// True if this is the all-zero placeholder signature.
    #[must_use]
    pub fn is_unsigned(&self) -> bool {
        self.0 == [0u8; SIGNATURE_BYTES]
    }
}

impl From<[u8; SIGNATURE_BYTES]> for Signature {
    fn from(bytes: [u8; SIGNATURE_BYTES]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Default for Signature {
    fn default() -> Self {
        Self::UNSIGNED
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base58())
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({})", self.to_base58())
    }
}

impl FromStr for Signature {
    type Err = ParseSignatureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(decode_base58_array::<SIGNATURE_BYTES>(s)?))
    }
}

// serde does not derive `Serialize`/`Deserialize` for `[u8; 64]` out of the box
// (arrays > 32 predate const generics in serde's impls), so we provide a small
// adapter that round-trips the array as a byte sequence.
#[cfg(feature = "serde")]
mod serde_bytes64 {
    use crate::SIGNATURE_BYTES;
    use serde::de::{Error, SeqAccess, Visitor};
    use serde::ser::SerializeTuple;
    use serde::{Deserializer, Serializer};
    use core::fmt;

    pub fn serialize<S: Serializer>(
        bytes: &[u8; SIGNATURE_BYTES],
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let mut tup = ser.serialize_tuple(SIGNATURE_BYTES)?;
        for b in bytes {
            tup.serialize_element(b)?;
        }
        tup.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        de: D,
    ) -> Result<[u8; SIGNATURE_BYTES], D::Error> {
        struct ArrVisitor;
        impl<'de> Visitor<'de> for ArrVisitor {
            type Value = [u8; SIGNATURE_BYTES];
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a 64-byte signature")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = [0u8; SIGNATURE_BYTES];
                for (i, slot) in out.iter_mut().enumerate() {
                    *slot = seq
                        .next_element()?
                        .ok_or_else(|| A::Error::invalid_length(i, &self))?;
                }
                Ok(out)
            }
        }
        de.deserialize_tuple(SIGNATURE_BYTES, ArrVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base58_roundtrip() {
        let sig = Signature::new([3u8; SIGNATURE_BYTES]);
        let parsed: Signature = sig.to_string().parse().unwrap();
        assert_eq!(sig, parsed);
    }

    #[test]
    fn unsigned_default() {
        assert!(Signature::default().is_unsigned());
        assert!(Signature::UNSIGNED.is_unsigned());
    }

    #[test]
    fn display_is_base58_not_hex() {
        let sig = Signature::new([0xcd; SIGNATURE_BYTES]);
        let s = sig.to_string();
        assert!(!s.contains("0x"));
    }

    #[test]
    fn parse_rejects_short_input() {
        let short = encode_base58(&[1u8; 32]);
        assert!(short.parse::<Signature>().is_err());
    }
}
