// SPDX-License-Identifier: Apache-2.0

//! Typed exact-byte and semantic digests with frozen transcript framing.

use crate::CoreError;
use sha2::{Digest, Sha256};
use std::fmt;

/// Frozen prefix for v0 SHA-256 digest text.
const SHA256_TEXT_PREFIX: &str = "sha256:";
/// Exact character length of one prefixed SHA-256 digest.
const SHA256_TEXT_LENGTH: usize = SHA256_TEXT_PREFIX.len() + 64;
/// Frozen outer domain-separation envelope for every Neutral Hash Transcript v1.
pub const HASH_TRANSCRIPT_ENVELOPE: &str = "neutral-nht-v1";

/// A typed SHA-256 digest of exact captured source bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceContentDigest([u8; 32]);

impl SourceContentDigest {
    /// Reconstructs a typed digest from an externally validated 32-byte value.
    #[must_use]
    pub const fn from_raw_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Computes the digest over exactly `bytes`, without normalization.
    ///
    /// Whitespace, BOMs, and line-ending changes therefore change source identity
    /// even when a later compiler considers the program logically equivalent.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let bytes: [u8; 32] = Sha256::digest(bytes).into();
        Self(bytes)
    }

    /// Returns the raw SHA-256 bytes for integrity comparison or storage.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// A typed SHA-256 digest of one exact externally encoded artifact section.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EncodedSectionDigest([u8; 32]);

impl EncodedSectionDigest {
    /// Computes the digest over exactly one encoded section's bytes.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let bytes: [u8; 32] = Sha256::digest(bytes).into();
        Self(bytes)
    }

    /// Returns the raw SHA-256 bytes for the external integrity envelope.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// A typed SHA-256 digest of exact captured vocabulary-bundle bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VocabularyContentDigest([u8; 32]);

impl VocabularyContentDigest {
    /// Reconstructs a typed digest from an externally validated 32-byte value.
    #[must_use]
    pub const fn from_raw_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Computes the digest over exactly `bytes`, without JSON normalization.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let bytes: [u8; 32] = Sha256::digest(bytes).into();
        Self(bytes)
    }

    /// Parses the frozen lowercase `sha256:` textual representation.
    ///
    /// # Errors
    ///
    /// Returns [`DigestTextError::InvalidSha256Text`] for any prefix, length,
    /// case, or hexadecimal deviation.
    pub fn parse_text(value: &str) -> Result<Self, DigestTextError> {
        if value.len() != SHA256_TEXT_LENGTH || !value.starts_with(SHA256_TEXT_PREFIX) {
            return Err(DigestTextError::InvalidSha256Text);
        }
        let mut bytes = [0_u8; 32];
        let hexadecimal = &value.as_bytes()[SHA256_TEXT_PREFIX.len()..];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = index * 2;
            let high = lowercase_hex_value(hexadecimal[offset])
                .ok_or(DigestTextError::InvalidSha256Text)?;
            let low = lowercase_hex_value(hexadecimal[offset + 1])
                .ok_or(DigestTextError::InvalidSha256Text)?;
            *byte = (high << 4) | low;
        }
        Ok(Self(bytes))
    }

    /// Returns the raw SHA-256 bytes for typed integrity comparison.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Compares digest bytes without data-dependent early exit.
    #[must_use]
    pub fn securely_matches(self, other: Self) -> bool {
        self.0
            .iter()
            .zip(other.0)
            .fold(0_u8, |difference, (left, right)| {
                difference | (left ^ right)
            })
            == 0
    }
}

impl fmt::Display for VocabularyContentDigest {
    /// Formats the digest using the frozen lowercase textual representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(SHA256_TEXT_PREFIX)?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// A malformed frozen digest textual representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestTextError {
    /// Text was not exactly `sha256:` followed by 64 lowercase hexadecimal digits.
    InvalidSha256Text,
}

/// Converts one lowercase ASCII hexadecimal digit to its numeric value.
fn lowercase_hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

/// A typed SHA-256 digest produced by a Neutral Hash Transcript v1 domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticDigest([u8; 32]);

impl SemanticDigest {
    /// Hashes a caller-constructed complete NHT transcript without framing it again.
    /// The caller owns transcript validation, domain separation, and resource bounds.
    ///
    /// Use this when an encoder has already written the envelope and domain.
    /// Passing that transcript to `from_nht` instead would add a second envelope
    /// and produce a different digest. Hashing alone does not validate meaning.
    #[must_use]
    pub fn from_transcript(transcript: &[u8]) -> Self {
        Self(Sha256::digest(transcript).into())
    }
    /// Reconstructs a typed semantic digest from validated transcript bytes.
    #[must_use]
    pub const fn from_raw_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Hashes `payload` using the frozen NHT-v1 framing and supplied ASCII domain.
    ///
    /// # Errors
    ///
    /// Returns an error when a tag is non-ASCII or a framed length cannot be
    /// represented by the frozen transcript integer widths.
    pub fn from_nht(domain: &str, payload: &[u8]) -> Result<Self, CoreError> {
        let domain = nht_frame(domain, payload)?;
        let transcript = nht_frame(HASH_TRANSCRIPT_ENVELOPE, &domain)?;
        let bytes: [u8; 32] = Sha256::digest(transcript).into();
        Ok(Self(bytes))
    }

    /// Returns the raw SHA-256 bytes for integrity comparison or storage.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for SemanticDigest {
    /// Formats the semantic digest using lower hexadecimal text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Frames one NHT-v1 tagged payload using unsigned big-endian lengths.
///
/// The layout is `u16(tag length) || ASCII tag || u64(payload length) || payload`.
/// Explicit lengths distinguish tuples that collide under plain concatenation.
/// This low-level helper checks representable lengths, not application budgets;
/// large or hostile inputs need a bounded encoder before reaching this boundary.
///
/// # Errors
///
/// Returns an error for non-ASCII tags or lengths outside the frozen widths.
pub fn nht_frame(tag: &str, payload: &[u8]) -> Result<Vec<u8>, CoreError> {
    if !tag.is_ascii() {
        return Err(CoreError::InvalidTranscriptTag);
    }
    let tag_length = u16::try_from(tag.len()).map_err(|_| CoreError::TranscriptLengthExceeded)?;
    let payload_length =
        u64::try_from(payload.len()).map_err(|_| CoreError::TranscriptLengthExceeded)?;
    let capacity = 2_usize
        .checked_add(tag.len())
        .and_then(|length| length.checked_add(8))
        .and_then(|length| length.checked_add(payload.len()))
        .ok_or(CoreError::TranscriptLengthExceeded)?;
    let mut frame = Vec::with_capacity(capacity);
    frame.extend_from_slice(&tag_length.to_be_bytes());
    frame.extend_from_slice(tag.as_bytes());
    frame.extend_from_slice(&payload_length.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

impl fmt::Display for SourceContentDigest {
    /// Formats the digest using the frozen `sha256:` lower-hex text form.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{SHA256_TEXT_PREFIX}")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}
