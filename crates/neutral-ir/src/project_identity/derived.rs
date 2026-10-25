// SPDX-License-Identifier: Apache-2.0

//! Shared `/3` derivation/artifact framing inputs and body helpers.

use super::{CAPTURE_LIMIT_TAGS, IdentityError, framing::Writer};
use crate::{ModuleSymbolIdentity, project::ProjectLimits};
use neutral_core::allocation::RetainCapacity;

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

/// Frames shared producer and project controls for the current expression derivation.
pub(super) fn base_derivation_body(
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

/// Frames exact artifact kind, format, normalized roots and explicit options.
pub(super) fn artifact_body(
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
    super::framing::ordered(&roots)?;
    super::framing::ordered(input.options.iter().map(|(name, _)| name))?;
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
            writer.frame("root", |writer| {
                writer.leaf(
                    "profile",
                    root.module().language_behavior_version().as_bytes(),
                )?;
                writer.leaf("module", root.module().module_name().as_bytes())?;
                writer.leaf("declaration", root.declaration_name().as_bytes())
            })?;
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
