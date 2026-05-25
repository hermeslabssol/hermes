//! Base58 codec used for every human-facing identifier in Logios.
//!
//! Solana uses the Bitcoin base58 alphabet (no `0`, `O`, `I`, `l`) for
//! pubkeys, blockhashes, and signatures. We wrap the [`bs58`] crate behind a
//! pair of helpers plus a self-contained reference implementation that we test
//! against `bs58` to guarantee alphabet/round-trip parity.

use thiserror::Error;

/// The Bitcoin/Solana base58 alphabet.
pub const ALPHABET: &[u8; 58] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Errors produced while decoding a base58 string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Base58Error {
    /// A character outside the base58 alphabet was encountered.
    #[error("invalid base58 character {0:?} at byte offset {1}")]
    InvalidCharacter(char, usize),
    /// The decoded byte length did not match the caller's expectation.
    #[error("decoded length {got} does not match expected length {expected}")]
    WrongLength {
        /// Number of bytes actually decoded.
        got: usize,
        /// Number of bytes the caller required.
        expected: usize,
    },
}

/// Encode arbitrary bytes to a base58 [`String`].
///
/// Leading zero bytes are preserved as leading `'1'` characters, matching the
/// Solana/Bitcoin convention.
#[must_use]
pub fn encode_base58(input: &[u8]) -> String {
    bs58::encode(input).into_string()
}

/// Decode a base58 string into a [`Vec<u8>`].
///
/// # Errors
///
/// Returns [`Base58Error::InvalidCharacter`] if any character is outside the
/// alphabet.
pub fn decode_base58(input: &str) -> Result<Vec<u8>, Base58Error> {
    bs58::decode(input).into_vec().map_err(|e| match e {
        bs58::decode::Error::InvalidCharacter { character, index } => {
            Base58Error::InvalidCharacter(character, index)
        }
        // Map any other bs58 failure onto an invalid-character report at offset 0;
        // the only failure mode `into_vec` exposes for our inputs is bad chars.
        _ => Base58Error::InvalidCharacter('\u{FFFD}', 0),
    })
}

/// Decode a base58 string into a fixed-size byte array.
///
/// # Errors
///
/// Returns [`Base58Error::WrongLength`] if the decoded length differs from `N`,
/// or [`Base58Error::InvalidCharacter`] for non-alphabet input.
pub fn decode_base58_array<const N: usize>(input: &str) -> Result<[u8; N], Base58Error> {
    let v = decode_base58(input)?;
    if v.len() != N {
        return Err(Base58Error::WrongLength {
            got: v.len(),
            expected: N,
        });
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&v);
    Ok(out)
}

/// Reference base58 encoder, kept independent of [`bs58`] so the test suite can
/// cross-check the dependency. Not used on the hot path.
#[doc(hidden)]
#[must_use]
pub fn encode_base58_reference(input: &[u8]) -> String {
    // Count leading zero bytes -> leading '1's.
    let zeros = input.iter().take_while(|&&b| b == 0).count();

    // Convert base-256 to base-58 via repeated division (big-endian digits).
    let mut digits: Vec<u8> = Vec::with_capacity(input.len() * 138 / 100 + 1);
    for &byte in input {
        let mut carry = byte as u32;
        for d in digits.iter_mut() {
            carry += (*d as u32) << 8;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }

    let mut out = String::with_capacity(zeros + digits.len());
    for _ in 0..zeros {
        out.push('1');
    }
    for &d in digits.iter().rev() {
        out.push(ALPHABET[d as usize] as char);
    }
    if out.is_empty() {
        out.push('1');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_arbitrary_bytes() {
        let cases: &[&[u8]] = &[
            b"",
            b"\x00",
            b"\x00\x00\x01",
            b"hello sealevel",
            &[0xff; 32],
            &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05],
        ];
        for &c in cases {
            let s = encode_base58(c);
            let back = decode_base58(&s).expect("decode");
            assert_eq!(back.as_slice(), c, "roundtrip failed for {c:?}");
        }
    }

    #[test]
    fn reference_matches_bs58() {
        for seed in 0u8..64 {
            let buf: Vec<u8> = (0..32).map(|i| seed.wrapping_mul(i).wrapping_add(i)).collect();
            assert_eq!(
                encode_base58(&buf),
                encode_base58_reference(&buf),
                "reference encoder diverged from bs58 for seed {seed}"
            );
        }
    }

    #[test]
    fn leading_zeros_become_ones() {
        let s = encode_base58(&[0, 0, 0, 7]);
        assert!(s.starts_with("111"), "expected leading ones, got {s}");
    }

    #[test]
    fn fixed_array_roundtrip() {
        let bytes = [9u8; 32];
        let s = encode_base58(&bytes);
        let back: [u8; 32] = decode_base58_array(&s).unwrap();
        assert_eq!(back, bytes);
    }

    #[test]
    fn wrong_length_is_reported() {
        let s = encode_base58(&[1u8; 16]);
        let err = decode_base58_array::<32>(&s).unwrap_err();
        assert!(matches!(
            err,
            Base58Error::WrongLength {
                got: 16,
                expected: 32
            }
        ));
    }

    #[test]
    fn invalid_character_is_rejected() {
        // '0' is not in the base58 alphabet.
        let err = decode_base58("0OIl").unwrap_err();
        assert!(matches!(err, Base58Error::InvalidCharacter(_, _)));
    }
}

// reviewed 2026-05-25
