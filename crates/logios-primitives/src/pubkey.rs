//! [`Pubkey`] — a 32-byte Solana account address or program id.

use crate::base58::{decode_base58_array, encode_base58, Base58Error};
use crate::PUBKEY_BYTES;
use core::fmt;
use core::str::FromStr;
use thiserror::Error;

/// A 32-byte public key identifying an account or program in the SVM.
///
/// Displayed and parsed as base58 — never hex. The Logios system program id
/// and the autonomous leader's identity are both `Pubkey`s.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Pubkey([u8; PUBKEY_BYTES]);

/// Error parsing a [`Pubkey`] from a base58 string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid pubkey: {0}")]
pub struct ParsePubkeyError(#[from] Base58Error);

impl Pubkey {
    /// The all-zero pubkey. Conventionally the SVM "system" / native owner id.
    pub const SYSTEM: Pubkey = Pubkey([0u8; PUBKEY_BYTES]);

    /// Construct from raw bytes.
    #[must_use]
    pub const fn new(bytes: [u8; PUBKEY_BYTES]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw 32-byte representation.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; PUBKEY_BYTES] {
        &self.0
    }

    /// Consume into raw bytes.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; PUBKEY_BYTES] {
        self.0
    }

    /// Render as a base58 string.
    #[must_use]
    pub fn to_base58(&self) -> String {
        encode_base58(&self.0)
    }

    /// True if this is the all-zero system pubkey.
    #[must_use]
    pub fn is_system(&self) -> bool {
        self.0 == [0u8; PUBKEY_BYTES]
    }
}

impl From<[u8; PUBKEY_BYTES]> for Pubkey {
    fn from(bytes: [u8; PUBKEY_BYTES]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base58())
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Mirror Solana tooling: pubkeys debug-print as their base58 form.
        write!(f, "Pubkey({})", self.to_base58())
    }
}

impl FromStr for Pubkey {
    type Err = ParsePubkeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(decode_base58_array::<PUBKEY_BYTES>(s)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base58_roundtrip() {
        let key = Pubkey::new([7u8; PUBKEY_BYTES]);
        let s = key.to_string();
        let parsed: Pubkey = s.parse().unwrap();
        assert_eq!(key, parsed);
    }

    #[test]
    fn system_is_all_ones_in_base58() {
        // 32 zero bytes encode to 32 leading '1' characters.
        assert_eq!(Pubkey::SYSTEM.to_string(), "1".repeat(32));
        assert!(Pubkey::SYSTEM.is_system());
    }

    #[test]
    fn display_never_contains_hex_prefix() {
        let key = Pubkey::new([0xab; PUBKEY_BYTES]);
        let s = key.to_string();
        assert!(!s.contains("0x"), "pubkey must be base58, not hex: {s}");
    }

    #[test]
    fn parse_wrong_length_fails() {
        // base58 of 16 bytes won't decode to 32.
        let short = encode_base58(&[1u8; 16]);
        assert!(short.parse::<Pubkey>().is_err());
    }

    #[test]
    fn debug_shows_base58() {
        let key = Pubkey::new([1u8; PUBKEY_BYTES]);
        let dbg = format!("{key:?}");
        assert!(dbg.starts_with("Pubkey("));
        assert!(!dbg.contains("0x"));
    }
}
