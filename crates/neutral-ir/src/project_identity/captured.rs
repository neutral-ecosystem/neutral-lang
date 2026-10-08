// SPDX-License-Identifier: Apache-2.0

//! Exact capture facts; source contents are represented by verified byte digests.

use super::{
    CAPTURED_DOMAIN, CapturedClosureIdentity, CompositionCapturedClosureIdentity, IdentityError,
    IdentityLimits, IdentityTranscript,
    framing::{Writer, ordered},
    transcript, transcript_profile,
};
use crate::composition::profile;
use neutral_core::allocation::RetainCapacity;
use neutral_core::{CancellationToken, SourceContentDigest, VocabularyContentDigest};

/// Borrowed source facts from a successfully frozen capture, never host paths.
pub struct CapturedIdentitySource<'a> {
    /// Exact module name.
    pub module: &'a str,
    /// Request-scoped logical source ID.
    pub source_id: &'a str,
    /// SHA-256 of exact source bytes.
    pub digest: SourceContentDigest,
    /// Exact source byte length.
    pub byte_len: u64,
}

/// Borrowed complete exact vocabulary lock facts.
pub struct CapturedIdentityVocabulary<'a> {
    /// Canonical vocabulary identity.
    pub identity: &'a str,
    /// Exact semantic release.
    pub version: &'a str,
    /// Exact encoding version.
    pub encoding_version: &'a str,
    /// Exact logical schema version.
    pub schema_version: &'a str,
    /// Exact captured vocabulary digest.
    pub digest: VocabularyContentDigest,
    /// Exact captured vocabulary byte length.
    pub byte_len: u64,
    /// Sorted unique feature IDs.
    pub required_features: &'a [String],
}

/// Canonically ordered complete capture facts, excluding controls and correlation keys.
pub struct CapturedIdentityInput<'a> {
    /// Exact source language profile.
    pub profile: &'a str,
    /// Complete source set in module order.
    pub sources: &'a [CapturedIdentitySource<'a>],
    /// Complete exact locks in canonical identity order.
    pub vocabularies: &'a [CapturedIdentityVocabulary<'a>],
}

/// Hashes verified capture facts without source normalization or host acquisition.
///
/// # Errors
/// Rejects malformed ordering, zero/overrun bounds, or cancellation; this does not validate source semantics.
pub fn captured_closure(
    input: &CapturedIdentityInput<'_>,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<CapturedClosureIdentity>, IdentityError> {
    transcript(
        CAPTURED_DOMAIN,
        limits,
        cancellation,
        |writer| capture_body(writer, input),
        CapturedClosureIdentity,
    )
}

/// Hashes verified successor capture facts in the frozen /2 profile and domain.
///
/// Features are required explicitly, in exact canonical order. This only projects
/// capture facts; it neither validates source semantics nor constructs logical,
/// derivation, interface or artifact identities. Old /1 framing is unchanged.
///
/// # Errors
/// Rejects unsupported profile/features, noncanonical input, independent bounds or cancellation.
pub fn captured_composition_closure(
    input: &CapturedIdentityInput<'_>,
    required_features: &[String],
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<CompositionCapturedClosureIdentity>, IdentityError> {
    if input.profile != neutral_core::profile::V1_SOURCE_PROFILE
        || !required_features
            .iter()
            .map(String::as_str)
            .eq(profile::REQUIRED_FEATURES.iter().copied())
    {
        return Err(IdentityError::InvalidInput);
    }
    transcript_profile(
        profile::CAPTURED_DOMAIN,
        profile::IDENTITY_PROFILE,
        limits,
        cancellation,
        |writer| {
            capture_body(writer, input)?;
            writer.items(required_features.len())?;
            writer.frame("features", |writer| {
                for feature in required_features {
                    writer.leaf("feature", feature.as_bytes())?;
                }
                Ok(())
            })
        },
        CompositionCapturedClosureIdentity,
    )
}

/// Frames unchanged exact capture fields once for both explicitly selected identity profiles.
fn capture_body(
    writer: &mut Writer<'_>,
    input: &CapturedIdentityInput<'_>,
) -> Result<(), IdentityError> {
    if input.profile.is_empty() || input.sources.is_empty() {
        return Err(IdentityError::InvalidInput);
    }
    writer.items(input.sources.len())?;
    writer.items(input.vocabularies.len())?;
    for source in input.sources {
        writer.text(source.module)?;
        writer.text(source.source_id)?;
        if source.source_id.chars().any(char::is_control) {
            return Err(IdentityError::InvalidInput);
        }
    }
    let mut source_ids = Vec::new();
    source_ids
        .try_retain(input.sources.len())
        .map_err(|_| IdentityError::Limit)?;
    source_ids.extend(input.sources.iter().map(|source| source.source_id));
    source_ids.sort_unstable();
    writer.check()?;
    ordered(source_ids)?;
    for vocabulary in input.vocabularies {
        writer.text(vocabulary.identity)?;
    }
    ordered(input.sources.iter().map(|source| source.module))?;
    ordered(
        input
            .vocabularies
            .iter()
            .map(|vocabulary| vocabulary.identity),
    )?;
    writer.leaf("profile", input.profile.as_bytes())?;
    writer.frame("sources", |writer| {
        for source in input.sources {
            if source.module.is_empty() || source.source_id.is_empty() {
                return Err(IdentityError::InvalidInput);
            }
            writer.frame("source", |writer| {
                writer.leaf("module", source.module.as_bytes())?;
                writer.leaf("source-id", source.source_id.as_bytes())?;
                writer.leaf("digest", &source.digest.as_bytes())?;
                writer.number("byte-length", source.byte_len)
            })?;
        }
        Ok(())
    })?;
    writer.frame("vocabularies", |writer| {
        for vocabulary in input.vocabularies {
            vocabulary_transcript(writer, vocabulary)?;
        }
        Ok(())
    })
}

/// Frames every semantic lock field, including byte identity and required features.
fn vocabulary_transcript(
    writer: &mut Writer<'_>,
    vocabulary: &CapturedIdentityVocabulary<'_>,
) -> Result<(), IdentityError> {
    writer.items(vocabulary.required_features.len())?;
    for feature in vocabulary.required_features {
        writer.text(feature)?;
    }
    ordered(vocabulary.required_features)?;
    writer.frame("vocabulary", |writer| {
        for (tag, text) in [
            ("identity", vocabulary.identity),
            ("version", vocabulary.version),
            ("encoding-version", vocabulary.encoding_version),
            ("schema-version", vocabulary.schema_version),
        ] {
            if text.is_empty() {
                return Err(IdentityError::InvalidInput);
            }
            writer.leaf(tag, text.as_bytes())?;
        }
        writer.leaf("digest", &vocabulary.digest.as_bytes())?;
        writer.number("byte-length", vocabulary.byte_len)?;
        writer.frame("features", |writer| {
            for feature in vocabulary.required_features {
                writer.leaf("feature", feature.as_bytes())?;
            }
            Ok(())
        })
    })
}
