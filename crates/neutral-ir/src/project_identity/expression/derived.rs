// SPDX-License-Identifier: Apache-2.0

//! Explicit /3 processing and representation inputs; no ambient package-version discovery.
//!
//! [`ExpressionDerivationContext`] couples complete logical/captured identities to
//! the producer revision, base capture/project controls and the composition,
//! expression and expansion policies. Controls are framed in their frozen named
//! tuple order, so changing acceptance policy can change derivation without
//! changing complete logical meaning.
//!
//! Artifact identity adds complete/view kind, format, roots and explicit options
//! under a separate domain. It is an identity for the selected representation
//! context, not a checksum of encoded artifact bytes or an attestation of execution.
//! Caller-supplied producer facts are recorded verbatim after structural checks;
//! no host environment or Cargo metadata is consulted to fill missing values.

use super::{ARTIFACT_DOMAIN, DERIVATION_DOMAIN, semantic_profiles};
use crate::{
    composition::{profile::COMPOSITION_LIMIT_TAGS, project::CompositionPolicy},
    expression::{
        profile::{self, ExpressionLimits, SemanticProfiles},
        templates::ExpansionLimits,
    },
    project::ProjectLimits,
    project_identity::{
        ArtifactIdentityInput, CAPTURE_LIMIT_TAGS, ExpressionArtifactIdentity,
        ExpressionCapturedClosureIdentity, ExpressionDerivationIdentity, ExpressionLogicalIdentity,
        IdentityError as E, IdentityLimits, IdentityTranscript,
        derived::{artifact_body, base_derivation_body},
        framing::Writer,
        transcript_profile,
    },
};
use neutral_core::CancellationToken;

/// Complete explicitly selected producer and acceptance context for expression projects.
pub struct ExpressionDerivationContext<'a> {
    /// Exact complete /3 capture evidence.
    pub captured: ExpressionCapturedClosureIdentity,
    /// Stable producer identity supplied by the caller.
    pub producer: &'a str,
    /// Explicit producer revision, never the identity schema or ambient Cargo version.
    pub producer_version: &'a str,
    /// Capture controls in the frozen fourteen-position order.
    pub capture_limits: [u64; CAPTURE_LIMIT_TAGS.len()],
    /// Complete base project controls.
    pub project_limits: ProjectLimits,
    /// Frozen fifteen-position composition controls.
    pub composition_limits: CompositionPolicy,
    /// Exact four semantic profiles and required feature set.
    pub semantic_profiles: &'a SemanticProfiles,
    /// Frozen eleven-position expression controls.
    pub expression_limits: ExpressionLimits,
    /// Frozen thirteen-position finite expansion controls.
    pub expansion_limits: ExpansionLimits,
}

/// Frames shared base processing fields followed by the complete frozen `/3` semantic and limit tuples.
///
/// # Errors
/// Rejects unsupported tuples, empty producer facts, zero controls, bounds or cancellation.
pub fn expression_derivation_identity(
    logical: ExpressionLogicalIdentity,
    context: &ExpressionDerivationContext<'_>,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<ExpressionDerivationIdentity>, E> {
    transcript_profile(
        DERIVATION_DOMAIN,
        profile::IDENTITY_PROFILE,
        limits,
        cancellation,
        |w| {
            base_derivation_body(
                w,
                &logical.as_bytes(),
                &context.captured.as_bytes(),
                context.producer,
                context.producer_version,
                context.capture_limits,
                context.project_limits,
            )?;
            controls(
                w,
                "composition-limits",
                COMPOSITION_LIMIT_TAGS,
                context.composition_limits.values(),
            )?;
            semantic_profiles(w, context.semantic_profiles)?;
            controls(
                w,
                "expression-limits",
                [
                    "nodes",
                    "syntax-depth",
                    "evaluation-depth",
                    "bindings",
                    "dependency-edges",
                    "work",
                    "retained-bytes",
                    "output-bytes",
                    "coefficient-digits",
                    "absolute-scale",
                    "diagnostics",
                ],
                context.expression_limits.values(),
            )?;
            controls(
                w,
                "expansion-limits",
                [
                    "definitions",
                    "applications",
                    "parameters",
                    "axes",
                    "axis-items",
                    "product-tuples",
                    "aggregate-tuples",
                    "expanded-bindings",
                    "expanded-nodes",
                    "depth",
                    "work",
                    "retained-bytes",
                    "output-bytes",
                ],
                context.expansion_limits.values(),
            )
        },
        ExpressionDerivationIdentity,
    )
}

/// Frames one exact positive control tuple without inventing fallback values.
fn controls<const N: usize>(
    w: &mut Writer<'_>,
    tag: &str,
    tags: [&str; N],
    values: [u64; N],
) -> Result<(), E> {
    // Array arity ties labels to positions; positivity is checked without substituting defaults.
    w.frame(tag, |w| {
        for (tag, value) in tags.into_iter().zip(values) {
            if value == 0 {
                return Err(E::InvalidInput);
            }
            w.number(tag, value)?;
        }
        Ok(())
    })
}

/// Frames the unchanged artifact grammar under the distinct /3 identity domain.
///
/// # Errors
/// Rejects malformed format/options, duplicate roots, complete-artifact roots, bounds or cancellation.
/// Public root existence and closure remain independently validated reader authority.
pub fn expression_artifact_identity(
    derivation: ExpressionDerivationIdentity,
    input: &ArtifactIdentityInput<'_>,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<ExpressionArtifactIdentity>, E> {
    transcript_profile(
        ARTIFACT_DOMAIN,
        profile::IDENTITY_PROFILE,
        limits,
        cancellation,
        |w| artifact_body(w, &derivation.as_bytes(), input),
        ExpressionArtifactIdentity,
    )
}
