//! [`Hash`] — a 32-byte base58 digest, used for slot blockhashes.

use crate::base58::{decode_base58_array, encode_base58, Base58Error};
use crate::PUBKEY_BYTES;
use core::fmt;
use core::str::FromStr;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// A 32-byte hash, displayed as base58.
///
/// In Hermes a `Hash` is most often a *blockhash*: the SHA-256 digest that
/// seals a slot's contents and is referenced by transactions for recent-
/// blockhash deduplication, exactly as on Solana.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Hash([u8; PUBKEY_BYTES]);

/// Error parsing a [`Hash`] from a base58 string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid hash: {0}")]
pub struct ParseHashError(#[from] Base58Error);

impl Hash {
    /// The all-zero hash (the genesis / default blockhash).
    pub const ZERO: Hash = Hash([0u8; PUBKEY_BYTES]);

    /// Construct from raw bytes.
    #[must_use]
    pub const fn new(bytes: [u8; PUBKEY_BYTES]) -> Self {
        Self(bytes)
    }

    /// Compute the SHA-256 digest of `data` as a `Hash`.
    #[must_use]
    #[allow(clippy::self_named_constructors)]
    pub fn hash(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let out = hasher.finalize();
        let mut bytes = [0u8; PUBKEY_BYTES];
        bytes.copy_from_slice(&out);
        Self(bytes)
    }

    /// Chain two hashes: `SHA256(self || next)`. Used to extend a slot's
    /// running blockhash over a sequence of entries.
    #[must_use]
    pub fn extend(&self, next: &Hash) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.0);
        hasher.update(next.0);
        let out = hasher.finalize();
        let mut bytes = [0u8; PUBKEY_BYTES];
        bytes.copy_from_slice(&out);
        Hash(bytes)
    }

    /// Borrow the raw bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; PUBKEY_BYTES] {
        &self.0
    }

    /// Render as base58.
    #[must_use]
    pub fn to_base58(&self) -> String {
        encode_base58(&self.0)
    }
}

impl From<[u8; PUBKEY_BYTES]> for Hash {
    fn from(bytes: [u8; PUBKEY_BYTES]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base58())
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", self.to_base58())
    }
}

impl FromStr for Hash {
    type Err = ParseHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(decode_base58_array::<PUBKEY_BYTES>(s)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_deterministic() {
        let a = Hash::hash(b"the chain that writes itself");
        let b = Hash::hash(b"the chain that writes itself");
        assert_eq!(a, b);
        assert_ne!(a, Hash::hash(b"something else"));
    }

    #[test]
    fn base58_roundtrip() {
        let h = Hash::hash(b"slot 42");
        let parsed: Hash = h.to_string().parse().unwrap();
        assert_eq!(h, parsed);
    }

    #[test]
    fn extend_is_order_sensitive() {
        let a = Hash::hash(b"a");
        let b = Hash::hash(b"b");
        assert_ne!(a.extend(&b), b.extend(&a));
    }

    #[test]
    fn zero_hash_is_base58_ones() {
        assert_eq!(Hash::ZERO.to_string(), "1".repeat(32));
    }

    #[test]
    fn known_sha256_vector() {
        // SHA-256("") well-known digest, base58-encoded for our Display.
        let empty = Hash::hash(b"");
        let expected_hex = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let got: String = empty
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(got, expected_hex);
    }
}
