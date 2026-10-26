// SPDX-License-Identifier: Apache-2.0

//! Exact /3 capture evidence, separate from expression meaning and processing controls.
//!
//! The current capture transcript combines exact source/lock facts with the frozen
//! expression/evaluator/expansion/decimal tuple and required features. Source text
//! formatting or captured vocabulary-byte changes therefore affect this layer even
//! when normalized project meaning is unchanged. Roots, runtime binding generations
//! and producer acceptance controls belong to other contexts.
//!
//! The shared capture body checks canonical shape/order under the writer's bounds;
//! it cannot verify a declared digest against content absent from these borrowed
//! inputs. Successful hashing proves reproducibility of these facts, not acquisition
//! success, source validity or a trusted producer signature.

use super::{CAPTURED_DOMAIN, features, semantic_profiles};
use crate::{
    expression::profile::{self, SemanticProfiles},
    project_identity::{
        CapturedIdentityInput, ExpressionCapturedClosureIdentity, IdentityError, IdentityLimits,
        IdentityTranscript, captured::capture_body, transcript_profile,
    },
};
use neutral_core::{CancellationToken, profile::V1_SOURCE_PROFILE};

/// Frames the complete supplied source/vocabulary closure under the exact /3 semantic tuple.
///
/// # Errors
/// Rejects malformed profile/features/order, independent bounds or cancellation.
/// Capture assertions, host paths and controls are deliberately not identity inputs.
/// The supplied byte digests and lengths must originate from independently checked capture facts.
pub fn captured_expression_closure(
    input: &CapturedIdentityInput<'_>,
    profiles: &SemanticProfiles,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<ExpressionCapturedClosureIdentity>, IdentityError> {
    if input.profile != V1_SOURCE_PROFILE || !profiles.matches_frozen() {
        return Err(IdentityError::InvalidInput);
    }
    transcript_profile(
        CAPTURED_DOMAIN,
        profile::IDENTITY_PROFILE,
        limits,
        cancellation,
        |w| {
            capture_body(w, input)?;
            features(w, "features", profiles)?;
            semantic_profiles(w, profiles)
        },
        ExpressionCapturedClosureIdentity,
    )
}
