// SPDX-License-Identifier: Apache-2.0

//! Reader-only /3 inspection; no compiler, source reparse, evaluator or acquisition path.

use neutral_core::CancellationToken;
use neutral_encoding::{DecodeError, DecodeLimits, expression::decode_expression_project};
use neutral_reader::{
    IdentityError, IdentityLimits, SemanticLimits,
    consumer::EntryRequest,
    expression::{
        ExpressionEntryError, ExpressionInterfaceIdentity, ExpressionLogicalIdentity,
        ExpressionResourceFacts, ExpressionView, ExpressionViewLimits,
        canonical_expression_project, expression_interface, profile::SemanticProfiles,
    },
};
use std::fmt::Write as _;

/// Renders the executable's compiler-free `/3` public catalogue.
///
/// This intentionally lists actual public owners instead of inventing an entry contract.
/// # Errors
/// Rejects malformed input, invalid roots, cancellation, allocation failure or oversized output.
#[allow(clippy::too_many_lines)]
pub fn render_standalone_expression(
    bytes: &[u8],
    roots: Option<&[String]>,
    cancel: &CancellationToken,
) -> Result<String, ExpressionProbeError> {
    use neutral_core::StructuralLimits;
    use neutral_core::allocation::RetainCapacity;
    use neutral_reader::{
        IdentityLimits, SemanticLimits,
        composition::{CompositionScalarLimits, ProjectCompositionLimits},
        expression::ExpansionLimits,
        expression::{canonical_expression_project, expression_interface, profile},
    };

    let scalar = StructuralLimits::new(
        neutral_encoding::constants::MAXIMUM_ARTIFACT_BYTES as u64,
        1,
    )
    .map_err(|_| ExpressionProbeError::Entry(ExpressionEntryError::Limit))?;
    let semantic = SemanticLimits {
        project: neutral_encoding::hard_project_limits(),
        composition: ProjectCompositionLimits::from_vocabulary(
            CompositionScalarLimits::from_structural(scalar),
        ),
        expression: profile::ExpressionLimits::hard(),
        expansion: ExpansionLimits::default(),
    };
    let project = decode_expression_project(bytes, DecodeLimits::hard(), semantic, cancel)
        .map_err(ExpressionProbeError::Decode)?;
    if cancel.is_cancelled() {
        return Err(ExpressionProbeError::Entry(ExpressionEntryError::Cancelled));
    }
    let ir = project.ir();
    let mut public = Vec::new();
    public
        .try_retain_exact(ir.declarations.len() + ir.public_templates.len())
        .map_err(|_| ExpressionProbeError::Entry(ExpressionEntryError::Limit))?;
    public.extend(
        ir.declarations
            .iter()
            .filter(|d| d.public)
            .map(|d| &d.identity),
    );
    public.extend(ir.public_templates.iter().map(|t| &t.identity));
    public.sort();
    if let Some(roots) = roots {
        for (index, root) in roots.iter().enumerate() {
            let Some((module, declaration)) = root.split_once("::") else {
                return Err(ExpressionProbeError::Entry(
                    ExpressionEntryError::InvalidInput,
                ));
            };
            if root.matches("::").count() != 1
                || !public.iter().any(|owner| {
                    owner.module().module_name() == module
                        && owner.declaration_name() == declaration
                })
                || roots[..index].contains(root)
            {
                return Err(ExpressionProbeError::Entry(
                    ExpressionEntryError::InvalidInput,
                ));
            }
        }
    }
    let identity_limits = IdentityLimits {
        bytes: project.model().limits().project.artifact_bytes,
        nodes: project.model().limits().project.nodes,
    };
    let logical = canonical_expression_project(ir, identity_limits, cancel)
        .map_err(ExpressionProbeError::Identity)?;
    let interface = expression_interface(ir, identity_limits, cancel)
        .map_err(ExpressionProbeError::Identity)?;
    let profiles = &ir.semantic_profiles;
    let mut out = StandaloneOutput::new(cancel);
    let values = ir.expression_resources.values();
    out.write_str("{\n").map_err(|_| out.error())?;
    out.field("encoding", format_args!("{}", profile::ENCODING))
        .map_err(|_| out.error())?;
    out.field(
        "identity_profile",
        format_args!("{}", profile::IDENTITY_PROFILE),
    )
    .map_err(|_| out.error())?;
    out.field("schema", format_args!("{}", ir.schema))
        .map_err(|_| out.error())?;
    out.field("modules", format_args!("{}", ir.modules.len()))
        .map_err(|_| out.error())?;
    out.field("declarations", format_args!("{}", ir.declarations.len()))
        .map_err(|_| out.error())?;
    out.list("resources", &values, true, |value, out| {
        out.string(format_args!("{value}"))
    })
    .map_err(|_| out.error())?;
    out.field("logical_identity", format_args!("{}", logical.identity()))
        .map_err(|_| out.error())?;
    out.field(
        "interface_identity",
        format_args!("{}", interface.identity()),
    )
    .map_err(|_| out.error())?;
    out.field(
        "expression_profile",
        format_args!("{}", profiles.expression),
    )
    .map_err(|_| out.error())?;
    out.field("evaluator_profile", format_args!("{}", profiles.evaluator))
        .map_err(|_| out.error())?;
    out.field("expansion_profile", format_args!("{}", profiles.expansion))
        .map_err(|_| out.error())?;
    out.field("decimal_profile", format_args!("{}", profiles.decimal))
        .map_err(|_| out.error())?;
    out.list(
        "features",
        &profiles.required_features,
        true,
        |value, out| out.string(format_args!("{value}")),
    )
    .map_err(|_| out.error())?;
    if let Some(roots) = roots {
        out.list("public_roots", roots, false, |root, out| {
            out.string(format_args!("{root}"))
        })
        .map_err(|_| out.error())?;
    } else {
        out.list("public_roots", &public, false, |identity, out| {
            out.string(format_args!(
                "{}::{}",
                identity.module().module_name(),
                identity.declaration_name()
            ))
        })
        .map_err(|_| out.error())?;
    }
    out.write_str("\n}\n").map_err(|_| out.error())?;
    if cancel.is_cancelled() {
        return Err(ExpressionProbeError::Entry(ExpressionEntryError::Cancelled));
    }
    Ok(out.text)
}

/// Bounded formatter with private output until successful completion.
struct StandaloneOutput<'a> {
    /// Unpublished serialized catalogue.
    text: String,
    /// Caller cancellation checked for every fragment.
    cancel: &'a CancellationToken,
    /// Error returned when the formatter cannot proceed.
    error: ExpressionProbeError,
}
impl<'a> StandaloneOutput<'a> {
    /// Starts an empty bounded output.
    fn new(cancel: &'a CancellationToken) -> Self {
        Self {
            text: String::new(),
            cancel,
            error: ExpressionProbeError::Entry(ExpressionEntryError::Limit),
        }
    }
    /// Preserves cancellation as the specific failure when formatting stops.
    fn error(&self) -> ExpressionProbeError {
        if self.cancel.is_cancelled() {
            ExpressionProbeError::Entry(ExpressionEntryError::Cancelled)
        } else {
            self.error
        }
    }
    /// Writes JSON escaped text.
    fn string(&mut self, value: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.write_str("\"")?;
        JsonString(self).write_fmt(value)?;
        self.write_str("\"")
    }
    /// Writes a scalar object member.
    fn field(&mut self, key: &str, value: std::fmt::Arguments<'_>) -> std::fmt::Result {
        write!(self, "  \"{key}\": ")?;
        self.string(value)?;
        self.write_str(",\n")
    }
    /// Writes a borrowed list without allocating intermediate strings.
    fn list<T>(
        &mut self,
        key: &str,
        values: &[T],
        comma: bool,
        render: impl Fn(&T, &mut Self) -> std::fmt::Result,
    ) -> std::fmt::Result {
        write!(self, "  \"{key}\": [")?;
        if !values.is_empty() {
            self.write_str("\n")?;
            for (i, value) in values.iter().enumerate() {
                self.write_str("    ")?;
                render(value, self)?;
                if i + 1 != values.len() {
                    self.write_str(",")?;
                }
                self.write_str("\n")?;
            }
            self.write_str("  ")?;
        }
        self.write_str(if comma { "],\n" } else { "]\n" })
    }
}
impl std::fmt::Write for StandaloneOutput<'_> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        use neutral_core::allocation::RetainCapacity;
        if self.cancel.is_cancelled() {
            return Err(std::fmt::Error);
        }
        if self.text.len().saturating_add(value.len())
            > neutral_encoding::constants::MAXIMUM_ARTIFACT_BYTES
        {
            return Err(std::fmt::Error);
        }
        if self.text.capacity() - self.text.len() < value.len() {
            self.text
                .try_retain(value.len())
                .map_err(|_| std::fmt::Error)?;
        }
        self.text.push_str(value);
        Ok(())
    }
}
/// Escapes JSON string fragments directly into the bounded output.
struct JsonString<'a, 'b>(&'a mut StandaloneOutput<'b>);
impl std::fmt::Write for JsonString<'_, '_> {
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

/// Exact independently bounded public entry selection after complete wire validation.
pub struct ExpressionEntryProbeRequest<'a> {
    /// Complete exact semantic profile/features, never inferred from source or package version.
    pub semantic_profiles: &'a SemanticProfiles,
    /// Exact nominal public entry contract and cardinality policy.
    pub entry: EntryRequest<'a>,
    /// Independent public selection/projection controls.
    pub limits: ExpressionViewLimits,
}
/// Atomic inspection failures without private artifact values, names or source identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionProbeError {
    /// Complete hostile-wire or independent reader validation failure.
    Decode(DecodeError),
    /// Exact public contract/profile/selection or projection failure.
    Entry(ExpressionEntryError),
    /// Independent typed identity framing failure.
    Identity(IdentityError),
}
/// Safe complete counts and independently derived typed identities with a redacted public view.
#[derive(Debug)]
pub struct ExpressionProbeSummary {
    /// Complete module count, never private module names.
    pub modules: u64,
    /// Complete declaration count, independent of selected roots.
    pub declarations: u64,
    /// Independently recomputed complete retained successor facts.
    pub resources: ExpressionResourceFacts,
    /// Complete typed logical meaning, never rewritten by roots or consumer bindings.
    pub logical_identity: ExpressionLogicalIdentity,
    /// Independently checked typed complete public interpretive digest.
    pub interface_identity: ExpressionInterfaceIdentity,
    /// Selected public interpretation closure without raw complete-project access.
    pub view: ExpressionView,
}

/// Independently decodes a complete /3 artifact and projects exact public entry contracts.
/// # Errors
/// Rejects unsupported/malformed bytes, profiles, absent contracts, invalid selections,
/// independent limits, allocation or cancellation without publishing a partial summary.
pub fn inspect_expression_encoded(
    bytes: &[u8],
    wire: DecodeLimits,
    semantic: SemanticLimits,
    request: &ExpressionEntryProbeRequest<'_>,
    cancel: &CancellationToken,
) -> Result<ExpressionProbeSummary, ExpressionProbeError> {
    if cancel.is_cancelled() {
        return Err(ExpressionProbeError::Entry(ExpressionEntryError::Cancelled));
    }
    if !request.semantic_profiles.matches_frozen() {
        return Err(ExpressionProbeError::Entry(
            ExpressionEntryError::Unsupported,
        ));
    }
    let project = decode_expression_project(bytes, wire, semantic, cancel)
        .map_err(ExpressionProbeError::Decode)?;
    let overhead = std::mem::size_of::<ExpressionProbeSummary>() as u64;
    let limits = ExpressionViewLimits {
        output_bytes: request
            .limits
            .output_bytes
            .checked_sub(overhead)
            .ok_or(ExpressionProbeError::Entry(ExpressionEntryError::Limit))?,
        work: request
            .limits
            .work
            .checked_sub(overhead + 1)
            .ok_or(ExpressionProbeError::Entry(ExpressionEntryError::Limit))?,
        ..request.limits
    };
    let identity_limits = IdentityLimits {
        bytes: project.model().limits().project.artifact_bytes,
        nodes: project.model().limits().project.nodes,
    };
    let logical_identity = canonical_expression_project(project.ir(), identity_limits, cancel)
        .map_err(ExpressionProbeError::Identity)?
        .identity();
    let interface_identity = expression_interface(project.ir(), identity_limits, cancel)
        .map_err(ExpressionProbeError::Identity)?
        .identity();
    let selected = project
        .entries(&request.entry, request.semantic_profiles, limits, cancel)
        .map_err(ExpressionProbeError::Entry)?;
    let view = selected
        .derive_view(cancel)
        .map_err(ExpressionProbeError::Entry)?;
    if cancel.is_cancelled() {
        return Err(ExpressionProbeError::Entry(ExpressionEntryError::Cancelled));
    }
    Ok(ExpressionProbeSummary {
        modules: project.ir().modules.len() as u64,
        declarations: project.ir().declarations.len() as u64,
        resources: project.model().facts(),
        logical_identity,
        interface_identity,
        view,
    })
}
