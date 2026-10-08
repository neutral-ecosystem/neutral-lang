// SPDX-License-Identifier: Apache-2.0

//! Explicit producer/processing inputs and post-compilation artifact partitions.

use super::{
    ARTIFACT_DOMAIN, CAPTURE_LIMIT_TAGS, CapturedClosureIdentity, CompositionArtifactIdentity,
    CompositionCapturedClosureIdentity, CompositionDerivationIdentity, CompositionLogicalIdentity,
    DERIVATION_DOMAIN, IdentityError, IdentityLimits, IdentityTranscript, LogicalProjectIdentity,
    ProjectArtifactIdentity, ProjectDerivationIdentity, framing::Writer, framing::ordered,
    logical::symbol, transcript, transcript_profile,
};
use crate::{
    ModuleSymbolIdentity,
    composition::{profile, project::CompositionPolicy},
    project::ProjectLimits,
};
use neutral_core::CancellationToken;
use neutral_core::allocation::RetainCapacity;

/// Complete explicit derivation inputs, never ambient compiler/host state.
pub struct DerivationContext<'a> {
    /// Exact captured closure that produced the logical input.
    pub captured: CapturedClosureIdentity,
    /// Stable producer implementation identity.
    pub producer: &'a str,
    /// Explicit producer revision; not the identity schema version.
    pub producer_version: &'a str,
    /// Fourteen acceptance limits in `CAPTURE_LIMIT_TAGS` order.
    pub capture_limits: [u64; CAPTURE_LIMIT_TAGS.len()],
    /// Complete project producer bounds, separate from logical meaning.
    pub project_limits: ProjectLimits,
}

/// Explicit successor processing inputs; composition bounds never enter logical meaning.
pub struct CompositionDerivationContext<'a> {
    /// Exact successor capture that produced the complete project.
    pub captured: CompositionCapturedClosureIdentity,
    /// Stable producer identity, not ambient executable discovery.
    pub producer: &'a str,
    /// Explicit producer revision, not the identity profile selector.
    pub producer_version: &'a str,
    /// Capture controls in the frozen fourteen-position order.
    pub capture_limits: [u64; CAPTURE_LIMIT_TAGS.len()],
    /// Explicit complete project producer bounds.
    pub project_limits: ProjectLimits,
    /// Independent composition controls in the frozen fifteen-position order.
    pub composition_limits: CompositionPolicy,
}

/// Distinct complete publication and public selection artifact domains.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactKind {
    /// Complete project, including private content and companions.
    Project,
    /// Post-compilation public dependency closure.
    View,
}

/// Explicit artifact selection/format context; no roots enter capture or logical meaning.
pub struct ArtifactIdentityInput<'a> {
    /// Complete project or selected public view.
    pub kind: ArtifactKind,
    /// Exact nonempty ASCII format identifier.
    pub format: &'a str,
    /// View roots, normalized before framing; empty for complete project artifacts.
    pub roots: &'a [ModuleSymbolIdentity],
    /// Sorted unique explicit format options, never implicit environment defaults.
    pub options: &'a [(&'a str, &'a str)],
}

/// Derives processing identity without adding source or producer data to logical identity.
///
/// # Errors
/// Rejects invalid/zero context, byte/node bounds, or cancellation.
pub fn derivation_identity(
    logical: LogicalProjectIdentity,
    context: &DerivationContext<'_>,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<ProjectDerivationIdentity>, IdentityError> {
    transcript(
        DERIVATION_DOMAIN,
        limits,
        cancellation,
        |writer| {
            derivation_body(
                writer,
                &logical.as_bytes(),
                &context.captured.as_bytes(),
                context.producer,
                context.producer_version,
                context.capture_limits,
                context.project_limits,
            )
        },
        ProjectDerivationIdentity,
    )
}

/// Frames the immutable /1 body shared by both explicitly selected identity profiles.
fn derivation_body(
    writer: &mut Writer<'_>,
    logical: &[u8; 32],
    captured: &[u8; 32],
    producer: &str,
    producer_version: &str,
    capture_limits: [u64; CAPTURE_LIMIT_TAGS.len()],
    project_limits: ProjectLimits,
) -> Result<(), IdentityError> {
    if producer.is_empty() || producer_version.is_empty() {
        return Err(IdentityError::InvalidInput);
    }
    writer.leaf("logical", logical)?;
    writer.leaf("captured", captured)?;
    writer.leaf("producer", producer.as_bytes())?;
    writer.leaf("producer-version", producer_version.as_bytes())?;
    writer.frame("capture-limits", |writer| {
        for (tag, limit) in CAPTURE_LIMIT_TAGS.iter().zip(capture_limits) {
            if limit == 0 {
                return Err(IdentityError::InvalidInput);
            }
            writer.number(tag, limit)?;
        }
        Ok(())
    })?;
    let value = project_limits;
    writer.frame("project-limits", |writer| {
        for (tag, limit) in [
            ("modules", value.modules),
            ("declarations", value.declarations),
            ("import-edges", value.import_edges),
            ("nodes", value.nodes),
            ("text-bytes", value.text_bytes),
            ("artifact-bytes", value.artifact_bytes),
        ] {
            if limit == 0 {
                return Err(IdentityError::InvalidInput);
            }
            writer.number(tag, limit)?;
        }
        Ok(())
    })
}

/// Frames successor processing identity, appending only the frozen composition policy tuple.
///
/// # Errors
/// Rejects empty producer facts, zero controls, exhausted bounds or cancellation.
pub fn composition_derivation_identity(
    logical: CompositionLogicalIdentity,
    context: &CompositionDerivationContext<'_>,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<CompositionDerivationIdentity>, IdentityError> {
    transcript_profile(
        profile::DERIVATION_DOMAIN,
        profile::IDENTITY_PROFILE,
        limits,
        cancellation,
        |writer| {
            derivation_body(
                writer,
                &logical.as_bytes(),
                &context.captured.as_bytes(),
                context.producer,
                context.producer_version,
                context.capture_limits,
                context.project_limits,
            )?;
            writer.frame("composition-limits", |writer| {
                for (tag, limit) in profile::COMPOSITION_LIMIT_TAGS
                    .iter()
                    .zip(context.composition_limits.values())
                {
                    if limit == 0 {
                        return Err(IdentityError::InvalidInput);
                    }
                    writer.number(tag, limit)?;
                }
                Ok(())
            })
        },
        CompositionDerivationIdentity,
    )
}

/// Derives artifact identity by kind, exact format, normalized selection, and options.
///
/// # Errors
/// Rejects duplicate roots/options, roots on a complete artifact, invalid format, bounds, or cancellation.
pub fn artifact_identity(
    derivation: ProjectDerivationIdentity,
    input: &ArtifactIdentityInput<'_>,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<ProjectArtifactIdentity>, IdentityError> {
    transcript(
        ARTIFACT_DOMAIN,
        limits,
        cancellation,
        |writer| artifact_body(writer, &derivation.as_bytes(), input),
        ProjectArtifactIdentity,
    )
}

/// Derives a successor artifact without accepting a /1 derivation or changing upstream identities.
///
/// # Errors
/// Rejects invalid format, duplicate roots/options, complete-artifact roots, bounds or cancellation.
pub fn composition_artifact_identity(
    derivation: CompositionDerivationIdentity,
    input: &ArtifactIdentityInput<'_>,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<CompositionArtifactIdentity>, IdentityError> {
    transcript_profile(
        profile::ARTIFACT_DOMAIN,
        profile::IDENTITY_PROFILE,
        limits,
        cancellation,
        |writer| artifact_body(writer, &derivation.as_bytes(), input),
        CompositionArtifactIdentity,
    )
}

/// Shares frozen artifact framing and fallible normalized-root retention between explicit profiles.
fn artifact_body(
    writer: &mut Writer<'_>,
    derivation: &[u8; 32],
    input: &ArtifactIdentityInput<'_>,
) -> Result<(), IdentityError> {
    writer.text(input.format)?;
    writer.items(input.roots.len())?;
    writer.items(input.options.len())?;
    for root in input.roots {
        writer.text(root.module().language_behavior_version())?;
        writer.text(root.module().module_name())?;
        writer.text(root.declaration_name())?;
    }
    for (name, _) in input.options {
        writer.text(name)?;
    }
    if input.format.is_empty()
        || !input.format.is_ascii()
        || input.format.bytes().any(|byte| byte.is_ascii_control())
        || (input.kind == ArtifactKind::Project && !input.roots.is_empty())
    {
        return Err(IdentityError::InvalidInput);
    }
    let mut roots = Vec::new();
    roots
        .try_retain(input.roots.len())
        .map_err(|_| IdentityError::Limit)?;
    roots.extend(input.roots);
    roots.sort_unstable();
    writer.check()?;
    ordered(&roots)?;
    ordered(input.options.iter().map(|(name, _)| name))?;
    writer.leaf("derivation", derivation)?;
    writer.leaf(
        "kind",
        match input.kind {
            ArtifactKind::Project => b"project",
            ArtifactKind::View => b"view",
        },
    )?;
    writer.leaf("format", input.format.as_bytes())?;
    writer.frame("roots", |writer| {
        for root in roots {
            writer.frame("root", |writer| symbol(writer, root))?;
        }
        Ok(())
    })?;
    writer.frame("options", |writer| {
        for (name, value) in input.options {
            if name.is_empty() {
                return Err(IdentityError::InvalidInput);
            }
            writer.frame("option", |writer| {
                writer.leaf("name", name.as_bytes())?;
                writer.leaf("value", value.as_bytes())
            })?;
        }
        Ok(())
    })
}
