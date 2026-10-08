// SPDX-License-Identifier: Apache-2.0

//! Standalone successor inspection through independent decoding and public closure only.

use crate::inspection_schema::{Field, FieldValue, render_fields_json};
use neutral_core::CancellationToken;
use neutral_core::allocation::{RetainCapacity, TryClone, text};
use neutral_encoding::{DecodeError, DecodeLimits, composition::decode_composition_project};
use neutral_reader::{
    IdentityError, IdentityLimits, MAX_TRANSCRIPT_BYTES, MAX_TRANSCRIPT_NODES,
    ModuleSymbolIdentity, ProjectLimits,
    composition::{
        CompositionLogicalIdentity, CompositionReadError, CompositionView, CompositionViewRequest,
        ProjectCompositionLimits, canonical_composition_project, profile,
    },
};
use std::fmt::Write as _;

/// Atomic inspection failures without private root names or partial projections.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionProbeError {
    /// Hostile wire or complete reader validation failed.
    Decode(DecodeError),
    /// Selection is invalid, private, unknown, duplicate or cancelled.
    View(CompositionReadError),
    /// Complete identity exceeded its independent framing bounds.
    Identity(IdentityError),
}

/// Safe complete counts and public interpretive data, never complete private IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionProbeSummary {
    /// Complete module count, independent of selection.
    pub modules: u64,
    /// Complete declaration count without private names.
    pub declarations: u64,
    /// Typed complete logical identity, never a selected-view fingerprint.
    pub logical_identity: CompositionLogicalIdentity,
    /// Independently checked complete interface digest.
    pub interface_fingerprint: String,
    /// Dependency-closed public-only contracts, values, reference paths and safe origins.
    pub view: CompositionView,
}

/// Independently inspects successor bytes; omitted roots select all public exports, explicit empty selects none.
///
/// # Errors
/// Rejects hostile bytes, invalid roots, bounds and cancellation without exposing partial success.
pub fn inspect_composition_encoded(
    bytes: &[u8],
    wire: DecodeLimits,
    project: ProjectLimits,
    composition: ProjectCompositionLimits,
    roots: Option<&[String]>,
    cancel: &CancellationToken,
) -> Result<CompositionProbeSummary, CompositionProbeError> {
    let project = decode_composition_project(bytes, wire, project, composition, cancel)
        .map_err(CompositionProbeError::Decode)?;
    let ir = project.complete_ir();
    let public_count = ir.declarations.iter().filter(|d| d.public).count();
    if roots.is_some_and(|roots| {
        roots.len() > public_count
            || roots
                .iter()
                .try_fold(0_u64, |n, r| n.checked_add(r.len() as u64))
                .is_none_or(|n| n > composition.work)
    }) {
        return Err(probe_limit());
    }
    let mut selected = Vec::new();
    selected
        .try_retain_exact(roots.map_or(public_count, <[String]>::len))
        .map_err(|_| probe_limit())?;
    let selected = match roots {
        None => {
            for declaration in ir.declarations.iter().filter(|d| d.public) {
                if cancel.is_cancelled() {
                    return Err(CompositionProbeError::View(CompositionReadError::Cancelled));
                }
                selected.push(
                    declaration
                        .identity
                        .try_clone()
                        .map_err(|_| probe_limit())?,
                );
            }
            selected
        }
        Some(roots) => {
            for root in roots {
                if cancel.is_cancelled() {
                    return Err(CompositionProbeError::View(CompositionReadError::Cancelled));
                }
                let (module, name) = root
                    .rsplit_once("::")
                    .ok_or(CompositionProbeError::View(CompositionReadError::Semantic))?;
                let index = ir
                    .declarations
                    .binary_search_by(|d| {
                        (
                            d.identity.module().module_name(),
                            d.identity.declaration_name(),
                        )
                            .cmp(&(module, name))
                    })
                    .map_err(|_| CompositionProbeError::View(CompositionReadError::Semantic))?;
                let owner = &ir.declarations[index];
                if !owner.public {
                    return Err(CompositionProbeError::View(CompositionReadError::Semantic));
                }
                selected.push(owner.identity.try_clone().map_err(|_| probe_limit())?);
            }
            selected
        }
    };
    let view = project
        .derive_view(
            &CompositionViewRequest {
                schema: text(profile::PROJECT_VIEW_SCHEMA).map_err(|_| probe_limit())?,
                roots: selected,
            },
            cancel,
        )
        .map_err(CompositionProbeError::View)?;
    let logical_identity = canonical_composition_project(
        ir,
        IdentityLimits {
            bytes: MAX_TRANSCRIPT_BYTES,
            nodes: MAX_TRANSCRIPT_NODES,
        },
        cancel,
    )
    .map_err(CompositionProbeError::Identity)?
    .identity();
    let mut fingerprint = String::new();
    fingerprint
        .try_retain_exact(ir.interface_digest.as_bytes().len() * 2)
        .map_err(|_| probe_limit())?;
    write!(fingerprint, "{}", ir.interface_digest).map_err(|_| probe_limit())?;
    if cancel.is_cancelled() {
        return Err(CompositionProbeError::View(CompositionReadError::Cancelled));
    }
    Ok(CompositionProbeSummary {
        modules: ir.resources.source_units,
        declarations: ir.resources.declarations,
        logical_identity,
        interface_fingerprint: fingerprint,
        view,
    })
}

/// Classifies failed probe retention without allocating diagnostic text.
fn probe_limit() -> CompositionProbeError {
    CompositionProbeError::View(CompositionReadError::Limit)
}

/// Formats an exact public owner for independent root selection.
fn symbol_name(owner: &ModuleSymbolIdentity) -> String {
    format!(
        "{}::{}",
        owner.module().module_name(),
        owner.declaration_name()
    )
}

/// Owned consumer projection assembled before JSON rendering.
struct Projection {
    /// Normalized public root names.
    roots: Vec<String>,
    /// Public source signatures and materialized values.
    exports: Vec<String>,
    /// Complete required public vocabulary contracts and dependencies.
    vocabularies: Vec<String>,
    /// Actual public reference occurrence paths.
    references: Vec<String>,
    /// Redacted public origin facts.
    origins: Vec<String>,
}
/// Converts only a redacted reader view into presentation strings.
fn projection(summary: &CompositionProbeSummary) -> Projection {
    let roots = summary
        .view
        .roots()
        .iter()
        .map(symbol_name)
        .collect::<Vec<_>>();
    let exports = summary
        .view
        .declarations()
        .iter()
        .map(|d| {
            format!(
                "{}: {:?} = {:?}",
                symbol_name(&d.identity),
                d.signature,
                d.value
            )
        })
        .collect::<Vec<_>>();
    let vocabularies = summary
        .view
        .vocabularies()
        .iter()
        .map(|b| {
            format!(
                "{}@{}: {:?}; dependencies: {:?}",
                b.identity.identity(),
                b.identity.version(),
                b.definitions,
                b.dependencies
            )
        })
        .collect::<Vec<_>>();
    let references = summary
        .view
        .references()
        .iter()
        .map(|(owner, r)| {
            format!(
                "{} {:?} -> {}",
                symbol_name(owner),
                r.path,
                symbol_name(&r.target)
            )
        })
        .collect::<Vec<_>>();
    let origins = summary
        .view
        .origins()
        .iter()
        .map(|o| {
            format!(
                "{} {:?}: {:?}; {:?}",
                symbol_name(&o.binding),
                o.path,
                o.kind,
                o.attribution
            )
        })
        .collect::<Vec<_>>();
    Projection {
        roots,
        exports,
        vocabularies,
        references,
        origins,
    }
}

/// Renders the safe consumer projection with complete contract/default/restriction/reference facts.
#[must_use]
pub fn render_composition_summary_json(summary: &CompositionProbeSummary) -> String {
    let modules = summary.modules.to_string();
    let declarations = summary.declarations.to_string();
    let logical = summary.logical_identity.to_string();
    let Projection {
        roots,
        exports,
        vocabularies,
        references,
        origins,
    } = projection(summary);
    render_fields_json(&[
        Field {
            json_key: "schema",
            text_prefix: "schema",
            value: FieldValue::Text(profile::PROJECT_IR_SCHEMA),
        },
        Field {
            json_key: "view_schema",
            text_prefix: "view",
            value: FieldValue::Text(summary.view.schema()),
        },
        Field {
            json_key: "identity_profile",
            text_prefix: "identity",
            value: FieldValue::Text(profile::IDENTITY_PROFILE),
        },
        Field {
            json_key: "logical_identity",
            text_prefix: "logical",
            value: FieldValue::Text(&logical),
        },
        Field {
            json_key: "interface_fingerprint",
            text_prefix: "interface",
            value: FieldValue::Text(&summary.interface_fingerprint),
        },
        Field {
            json_key: "modules",
            text_prefix: "modules",
            value: FieldValue::Text(&modules),
        },
        Field {
            json_key: "declarations",
            text_prefix: "declarations",
            value: FieldValue::Text(&declarations),
        },
        Field {
            json_key: "roots",
            text_prefix: "roots",
            value: FieldValue::TextList(&roots),
        },
        Field {
            json_key: "contracts_and_values",
            text_prefix: "contracts",
            value: FieldValue::TextList(&exports),
        },
        Field {
            json_key: "vocabularies",
            text_prefix: "vocabularies",
            value: FieldValue::TextList(&vocabularies),
        },
        Field {
            json_key: "references",
            text_prefix: "references",
            value: FieldValue::TextList(&references),
        },
        Field {
            json_key: "origins",
            text_prefix: "origins",
            value: FieldValue::TextList(&origins),
        },
    ])
}
