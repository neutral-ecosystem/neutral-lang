// SPDX-License-Identifier: Apache-2.0

//! Reader-only inspection support for Neutral artifacts.
//!
//! This library owns generic traversal and consumer-facing observations over
//! validated reader views. It must remain independent of the compiler, private
//! frontend models, test support, and host acquisition behavior.

use neutral_core::{
    CancellationToken, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity,
};
use neutral_encoding::{DecodeError, DecodeLimits, decode};
use neutral_reader::{ElementId, ValidatedDocument};

/// Stable host-output prefixes shared by the probe library and binary.
pub mod output {
    /// Error output category prefix.
    pub const ERROR: &str = "[error]";
    /// Informational output category prefix.
    pub const INFO: &str = "[info]";
}

/// Stable consumer-owned diagnostic identifiers.
pub mod diagnostics {
    /// Source-linked probe observation diagnostic.
    pub const OBSERVATION: &str = "NEU-PROBE-001";
}

/// Deterministic generic summary of one validated Neutral document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeSummary {
    /// Logical module name.
    module: String,
    /// Logical, source-map, derivation, resource, and identity metadata.
    metadata: Vec<String>,
    /// Exact captured vocabulary identity summary, when present.
    vocabulary: Option<String>,
    /// Vocabulary-owned type summaries in reader order.
    vocabulary_types: Vec<String>,
    /// Nominal record schema summaries in reader order.
    record_types: Vec<String>,
    /// Typed declaration summaries in reader order.
    declarations: Vec<String>,
    /// Original-byte source mappings in element order.
    source_mappings: Vec<String>,
    /// Root value-origin and normalization summaries in compiler order.
    value_provenance: Vec<String>,
    /// Explicit/default field-provenance summaries in compiler order.
    field_provenance: Vec<String>,
    /// Ordinary immutable-value reuse edges in compiler order.
    reuse_provenance: Vec<String>,
    /// Typed identity-reference edges in compiler order.
    reference_provenance: Vec<String>,
    /// Safe diagnostic-code summaries.
    diagnostics: Vec<String>,
}

impl ProbeSummary {
    /// Returns the logical module name.
    #[must_use]
    pub fn module(&self) -> &str {
        &self.module
    }

    /// Returns logical, source-map, derivation, resource, and identity metadata.
    #[must_use]
    pub fn metadata(&self) -> &[String] {
        &self.metadata
    }

    /// Returns the exact captured vocabulary identity summary, when present.
    #[must_use]
    pub fn vocabulary(&self) -> Option<&str> {
        self.vocabulary.as_deref()
    }

    /// Returns generic vocabulary-owned schema summaries.
    #[must_use]
    pub fn vocabulary_types(&self) -> &[String] {
        &self.vocabulary_types
    }

    /// Returns deterministic typed declaration summaries.
    #[must_use]
    pub fn declarations(&self) -> &[String] {
        &self.declarations
    }

    /// Returns original-byte source mappings in element order.
    #[must_use]
    pub fn source_mappings(&self) -> &[String] {
        &self.source_mappings
    }

    /// Returns root value-origin and normalization summaries.
    #[must_use]
    pub fn value_provenance(&self) -> &[String] {
        &self.value_provenance
    }

    /// Returns deterministic nominal record schema summaries.
    #[must_use]
    pub fn record_types(&self) -> &[String] {
        &self.record_types
    }

    /// Returns deterministic explicit/default record-field provenance.
    #[must_use]
    pub fn field_provenance(&self) -> &[String] {
        &self.field_provenance
    }

    /// Returns deterministic ordinary immutable-value reuse edges.
    #[must_use]
    pub fn reuse_provenance(&self) -> &[String] {
        &self.reuse_provenance
    }

    /// Returns deterministic typed identity-reference edges traversed by ID.
    #[must_use]
    pub fn reference_provenance(&self) -> &[String] {
        &self.reference_provenance
    }

    /// Returns safe probe diagnostic-code summaries.
    #[must_use]
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

/// Traverses only immutable public reader views to summarize a document.
#[must_use]
pub fn summarize(document: &ValidatedDocument) -> ProbeSummary {
    let artifacts = document.artifacts();
    let (vocabulary, vocabulary_types) = summarize_vocabulary(document);
    let metadata = summarize_metadata(document);
    let record_types = summarize_record_types(document);
    let declarations = summarize_declarations(document);
    let source_mappings = summarize_source_mappings(document);
    let value_provenance = artifacts
        .provenance()
        .iter()
        .map(|record| {
            format!(
                "{}:{}:{}",
                record.element_id().get(),
                record.origin().as_str(),
                record.normalization().as_str()
            )
        })
        .collect();
    let field_provenance = artifacts
        .field_provenance()
        .iter()
        .map(|record| {
            format!(
                "{}:{}:{}",
                record.element_id().get(),
                record.field_path().join("."),
                record.origin().as_str()
            )
        })
        .collect();
    let reuse_provenance = artifacts
        .reuse_provenance()
        .iter()
        .map(|record| {
            format!(
                "{}:{}:{}",
                record.element_id().get(),
                record.value_path().join("."),
                record.source_element_id().get()
            )
        })
        .collect();
    let reference_provenance = artifacts
        .reference_provenance()
        .iter()
        .map(|record| {
            format!(
                "{}:{}:{}",
                record.element_id().get(),
                record.value_path().join("."),
                record.target_element_id().get()
            )
        })
        .collect();
    ProbeSummary {
        module: document.module_name().to_owned(),
        metadata,
        vocabulary,
        vocabulary_types,
        record_types,
        declarations,
        source_mappings,
        value_provenance,
        field_provenance,
        reuse_provenance,
        reference_provenance,
        diagnostics: Vec::new(),
    }
}

/// Summarizes version, source, derivation, resource, and identity metadata.
fn summarize_metadata(document: &ValidatedDocument) -> Vec<String> {
    let artifacts = document.artifacts();
    let logical = artifacts.logical_document();
    let source_map = artifacts.source_map();
    let derivation = artifacts.derivation();
    let acceptance = derivation.acceptance();
    let resources = derivation.resource_facts();
    let mut metadata = vec![
        format!(
            "language-behavior {}",
            logical.module().language_behavior_version()
        ),
        format!(
            "logical-ir-schema {}",
            derivation.logical_ir_schema_version()
        ),
        format!("source-map-schema {}", derivation.source_map_version()),
        format!("provenance-schema {}", derivation.provenance_version()),
        format!("source-digest {}", source_map.source_digest()),
        format!("source-bytes {}", source_map.source_byte_length()),
        format!(
            "module-span {}..{}",
            source_map.module_span().start(),
            source_map.module_span().end()
        ),
        format!(
            "resource-facts source-bytes={} declarations={} diagnostics={} decoded-string-bytes={}",
            resources.source_bytes(),
            resources.declarations(),
            resources.diagnostics(),
            resources.decoded_string_bytes()
        ),
        format!(
            "acceptance source-bytes={} diagnostics={} string-bytes={} numeric-digits={} numeric-scale={} declarations={} record-fields={} nesting-depth={} list-items={} traversal-nodes={}",
            acceptance.source_byte_limit(),
            acceptance.diagnostic_limit(),
            acceptance.string_byte_limit(),
            acceptance.numeric_digit_limit(),
            acceptance.numeric_scale_limit(),
            acceptance.declaration_limit(),
            acceptance.record_field_limit(),
            acceptance.nesting_depth_limit(),
            acceptance.list_item_limit(),
            acceptance.traversal_node_limit()
        ),
        format!(
            "safe-bounded-output {}",
            derivation.diagnostics().safe_bounded_output()
        ),
    ];
    metadata.extend(document.record_types().iter().map(|record| {
        format!(
            "record-identity id={} symbol={}::{} fingerprint={}",
            record.element_id().get(),
            record.symbol_identity().module().module_name(),
            record.symbol_identity().declaration_name(),
            record.fingerprint().digest()
        )
    }));
    metadata.extend(document.declarations().iter().map(|declaration| {
        format!(
            "declaration-identity id={} symbol={}::{} fingerprint={}",
            declaration.element_id().get(),
            declaration.symbol_identity().module().module_name(),
            declaration.symbol_identity().declaration_name(),
            declaration.fingerprint().digest()
        )
    }));
    metadata
}

/// Summarizes nominal record schemas in deterministic reader order.
fn summarize_record_types(document: &ValidatedDocument) -> Vec<String> {
    document
        .record_types()
        .iter()
        .map(|record| {
            let fields = record
                .fields()
                .iter()
                .map(|field| match field.default_value() {
                    Some(default) => {
                        format!("{}: {} = {}", field.name(), field.resolved_type(), default)
                    }
                    None => format!("{}: {}", field.name(), field.resolved_type()),
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("record {} {{ {fields} }}", record.name())
        })
        .collect()
}

/// Summarizes declarations with resolved types and final logical values.
fn summarize_declarations(document: &ValidatedDocument) -> Vec<String> {
    document
        .declarations()
        .iter()
        .map(|declaration| {
            format!(
                "{}: {} = {}",
                declaration.name(),
                declaration.resolved_type(),
                declaration.value()
            )
        })
        .collect()
}

/// Summarizes every original-byte source-map entry in element order.
fn summarize_source_mappings(document: &ValidatedDocument) -> Vec<String> {
    document
        .artifacts()
        .source_map()
        .entries()
        .iter()
        .map(|entry| {
            format!(
                "{}:declaration={}..{}:type={}..{}:name={}..{}:value={}..{}",
                entry.element_id().get(),
                entry.declaration_span().start(),
                entry.declaration_span().end(),
                entry.type_span().start(),
                entry.type_span().end(),
                entry.name_span().start(),
                entry.name_span().end(),
                entry.value_span().start(),
                entry.value_span().end()
            )
        })
        .collect()
}

/// Renders a summary as deterministic, categorized consumer observations.
#[must_use]
pub fn render_summary(summary: &ProbeSummary) -> Vec<String> {
    let mut lines = vec![format!("module {}", summary.module())];
    lines.extend(
        summary
            .metadata()
            .iter()
            .map(|value| format!("metadata {value}")),
    );
    if let Some(vocabulary) = summary.vocabulary() {
        lines.push(format!("vocabulary {vocabulary}"));
    }
    lines.extend(
        summary
            .record_types()
            .iter()
            .map(|record| format!("record {record}")),
    );
    lines.extend(
        summary
            .vocabulary_types()
            .iter()
            .map(|record| format!("vocabulary-type {record}")),
    );
    lines.extend(
        summary
            .declarations()
            .iter()
            .map(|declaration| format!("declaration {declaration}")),
    );
    lines.extend(
        summary
            .source_mappings()
            .iter()
            .map(|record| format!("source-map {record}")),
    );
    lines.extend(
        summary
            .value_provenance()
            .iter()
            .map(|record| format!("value-provenance {record}")),
    );
    lines.extend(
        summary
            .field_provenance()
            .iter()
            .map(|record| format!("field-provenance {record}")),
    );
    lines.extend(
        summary
            .reuse_provenance()
            .iter()
            .map(|record| format!("reuse-provenance {record}")),
    );
    lines.extend(
        summary
            .reference_provenance()
            .iter()
            .map(|record| format!("reference-provenance {record}")),
    );
    lines.extend(
        summary
            .diagnostics()
            .iter()
            .map(|diagnostic| format!("diagnostic {diagnostic}")),
    );
    lines
}

/// Decodes hostile external bytes and returns only a validated generic summary.
///
/// # Errors
///
/// Returns the decoder's stable bounded failure when the artifact is malformed,
/// unsupported, oversized, inconsistent, or cancelled.
pub fn inspect_encoded(
    bytes: &[u8],
    limits: DecodeLimits,
    cancellation: &CancellationToken,
) -> Result<ProbeSummary, DecodeError> {
    decode(bytes, limits, cancellation).map(|document| summarize(&document))
}

/// Summarizes exact vocabulary identity and schema data without interpretation.
fn summarize_vocabulary(document: &ValidatedDocument) -> (Option<String>, Vec<String>) {
    let vocabulary = document.vocabulary().map(|contract| {
        let identity = contract.identity();
        format!(
            "{}@{} schema={} encoding={} digest={} features=[{}]",
            identity.identity(),
            identity.version(),
            identity.schema_version(),
            identity.encoding_version(),
            identity.content_digest(),
            identity.required_features().join(",")
        )
    });
    let types = document.vocabulary().map_or_else(Vec::new, |contract| {
        contract
            .types()
            .iter()
            .map(|definition| {
                let fields = definition
                    .fields()
                    .iter()
                    .map(|field| match field.default_value() {
                        Some(default) => {
                            format!("{}: {} = {}", field.name(), field.resolved_type(), default)
                        }
                        None => format!("{}: {}", field.name(), field.resolved_type()),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("record {} {{ {fields} }}", definition.identity())
            })
            .collect()
    });
    (vocabulary, types)
}

/// Creates a consumer-owned diagnostic mapped through the public source map.
///
/// # Errors
///
/// Returns an error when `element_id` does not exist in the validated document.
pub fn source_linked_diagnostic(
    document: &ValidatedDocument,
    element_id: ElementId,
) -> Result<Diagnostic, ProbeError> {
    let primary = document
        .source_location(element_id)
        .ok_or(ProbeError::UnknownElement)?;
    let code =
        DiagnosticCode::new(diagnostics::OBSERVATION).map_err(|_| ProbeError::InternalInvariant)?;
    Ok(Diagnostic::new(
        code,
        DiagnosticLayer::Consumer,
        DiagnosticSeverity::Note,
        primary,
        Vec::new(),
        false,
    ))
}

/// A bounded generic-probe traversal failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbeError {
    /// The requested graph-local element was absent.
    UnknownElement,
    /// A frozen project-owned diagnostic contract was internally invalid.
    InternalInvariant,
}

#[cfg(test)]
#[path = "../tests/library/mod.rs"]
mod tests;
