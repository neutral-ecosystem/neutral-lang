// SPDX-License-Identifier: Apache-2.0

//! Standalone successor inspection through independent decoding and public closure only.

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

/// Borrowed owner formatting avoids an intermediate allocated root name.
struct Symbol<'a>(&'a ModuleSymbolIdentity);
impl std::fmt::Display for Symbol<'_> {
    /// Writes the exact qualified public owner through the caller's fallible writer.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}::{}",
            self.0.module().module_name(),
            self.0.declaration_name()
        )
    }
}

/// Private output is published only after all formatting and cancellation checks succeed.
struct Output<'a> {
    /// Unpublished JSON bytes, retained only through checked growth.
    text: String,
    /// Caller-owned cancellation policy checked at every formatted fragment.
    cancel: &'a CancellationToken,
    /// Nonallocating classification retained when the formatter returns its unit error.
    error: CompositionProbeError,
}
impl std::fmt::Write for Output<'_> {
    /// Reserves each growth fallibly and checks cancellation even when capacity is already available.
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        if self.cancel.is_cancelled() {
            self.error = CompositionProbeError::View(CompositionReadError::Cancelled);
            return Err(std::fmt::Error);
        }
        if self.text.capacity() - self.text.len() < value.len() {
            self.text
                .try_retain(value.len())
                .map_err(|_| std::fmt::Error)?;
        }
        if self.cancel.is_cancelled() {
            self.error = CompositionProbeError::View(CompositionReadError::Cancelled);
            return Err(std::fmt::Error);
        }
        self.text.push_str(value);
        Ok(())
    }
}

/// Escapes formatted fragments directly into output, never allocating a presentation string.
struct JsonString<'a, 'b>(&'a mut Output<'b>);
impl std::fmt::Write for JsonString<'_, '_> {
    /// Preserves Unicode while escaping quotes, backslashes and every JSON control character.
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        let mut start = 0;
        for (index, c) in value.char_indices() {
            if c == '"' || c == '\\' || c <= '\u{001f}' {
                self.0.write_str(&value[start..index])?;
                match c {
                    '"' => self.0.write_str("\\\"")?,
                    '\\' => self.0.write_str("\\\\")?,
                    '\n' => self.0.write_str("\\n")?,
                    '\r' => self.0.write_str("\\r")?,
                    '\t' => self.0.write_str("\\t")?,
                    control => write!(self.0, "\\u{:04x}", u32::from(control))?,
                }
                start = index + c.len_utf8();
            }
        }
        self.0.write_str(&value[start..])
    }
}

impl Output<'_> {
    /// Writes an escaped JSON string from borrowed formatting arguments.
    fn string(&mut self, value: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.write_str("\"")?;
        JsonString(self).write_fmt(value)?;
        self.write_str("\"")
    }

    /// Writes a fixed schema member with no temporary number or digest string.
    fn field(&mut self, key: &str, value: std::fmt::Arguments<'_>) -> std::fmt::Result {
        write!(self, "  \"{key}\": ")?;
        self.string(value)?;
        self.write_str(",\n")
    }

    /// Writes a borrowed collection using the existing ordered, two-space inspection layout.
    fn list<T>(
        &mut self,
        key: &str,
        values: &[T],
        render: impl Fn(&T, &mut Self) -> std::fmt::Result,
    ) -> std::fmt::Result {
        write!(self, "  \"{key}\": [")?;
        if !values.is_empty() {
            self.write_str("\n")?;
            for (index, value) in values.iter().enumerate() {
                self.write_str("    ")?;
                render(value, self)?;
                if index + 1 != values.len() {
                    self.write_str(",")?;
                }
                self.write_str("\n")?;
            }
            self.write_str("  ")?;
        }
        self.write_str("]")
    }
}

/// Renders public contract/default/restriction/reference facts without intermediate owned projections.
///
/// # Errors
/// Returns retention failure or cancellation without publishing partial JSON.
pub fn render_composition_summary_json(
    summary: &CompositionProbeSummary,
    cancel: &CancellationToken,
) -> Result<String, CompositionProbeError> {
    let mut output = Output {
        text: String::new(),
        cancel,
        error: probe_limit(),
    };
    render_projection(summary, &mut output).map_err(|_| output.error)?;
    if cancel.is_cancelled() {
        return Err(CompositionProbeError::View(CompositionReadError::Cancelled));
    }
    Ok(output.text)
}

/// Formats the redacted reader view using borrowed facts and the shared inspection schema version.
fn render_projection(summary: &CompositionProbeSummary, out: &mut Output<'_>) -> std::fmt::Result {
    write!(
        out,
        "{{\n  \"schema_version\": {},\n",
        crate::inspection_schema::SCHEMA_VERSION
    )?;
    out.field("schema", format_args!("{}", profile::PROJECT_IR_SCHEMA))?;
    out.field("view_schema", format_args!("{}", summary.view.schema()))?;
    out.field(
        "identity_profile",
        format_args!("{}", profile::IDENTITY_PROFILE),
    )?;
    out.field(
        "logical_identity",
        format_args!("{}", summary.logical_identity),
    )?;
    out.field(
        "interface_fingerprint",
        format_args!("{}", summary.interface_fingerprint),
    )?;
    out.field("modules", format_args!("{}", summary.modules))?;
    out.field("declarations", format_args!("{}", summary.declarations))?;
    out.list("roots", summary.view.roots(), |owner, out| {
        out.string(format_args!("{}", Symbol(owner)))
    })?;
    out.write_str(",\n")?;
    out.list(
        "contracts_and_values",
        summary.view.declarations(),
        |d, out| {
            out.string(format_args!(
                "{}: {:?} = {:?}",
                Symbol(&d.identity),
                d.signature,
                d.value
            ))
        },
    )?;
    out.write_str(",\n")?;
    out.list("vocabularies", summary.view.vocabularies(), |b, out| {
        out.string(format_args!(
            "{}@{}: {:?}; dependencies: {:?}",
            b.identity.identity(),
            b.identity.version(),
            b.definitions,
            b.dependencies
        ))
    })?;
    out.write_str(",\n")?;
    out.list(
        "references",
        summary.view.references(),
        |(owner, r), out| {
            out.string(format_args!(
                "{} {:?} -> {}",
                Symbol(owner),
                r.path,
                Symbol(&r.target)
            ))
        },
    )?;
    out.write_str(",\n")?;
    out.list("origins", summary.view.origins(), |o, out| {
        out.string(format_args!(
            "{} {:?}: {:?}; {:?}",
            Symbol(&o.binding),
            o.path,
            o.kind,
            o.attribution
        ))
    })?;
    out.write_str("\n}\n")
}
