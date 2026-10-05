// SPDX-License-Identifier: Apache-2.0

//! Explicit producer/processing inputs and post-compilation artifact partitions.

use super::{
    ARTIFACT_DOMAIN, CAPTURE_LIMIT_TAGS, CapturedClosureIdentity, DERIVATION_DOMAIN, IdentityError,
    IdentityLimits, IdentityTranscript, LogicalProjectIdentity, ProjectArtifactIdentity,
    ProjectDerivationIdentity, framing::ordered, logical::symbol, transcript,
};
use crate::{ModuleSymbolIdentity, project::ProjectLimits};
use neutral_core::CancellationToken;

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
            if context.producer.is_empty() || context.producer_version.is_empty() {
                return Err(IdentityError::InvalidInput);
            }
            writer.leaf("logical", &logical.as_bytes())?;
            writer.leaf("captured", &context.captured.as_bytes())?;
            writer.leaf("producer", context.producer.as_bytes())?;
            writer.leaf("producer-version", context.producer_version.as_bytes())?;
            writer.frame("capture-limits", |writer| {
                for (tag, limit) in CAPTURE_LIMIT_TAGS.iter().zip(context.capture_limits) {
                    if limit == 0 {
                        return Err(IdentityError::InvalidInput);
                    }
                    writer.number(tag, limit)?;
                }
                Ok(())
            })?;
            let value = context.project_limits;
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
        },
        ProjectDerivationIdentity,
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
        |writer| {
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
                .try_reserve(input.roots.len())
                .map_err(|_| IdentityError::Limit)?;
            roots.extend(input.roots);
            roots.sort_unstable();
            writer.check()?;
            ordered(&roots)?;
            ordered(input.options.iter().map(|(name, _)| name))?;
            writer.leaf("derivation", &derivation.as_bytes())?;
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
        },
        ProjectArtifactIdentity,
    )
}
