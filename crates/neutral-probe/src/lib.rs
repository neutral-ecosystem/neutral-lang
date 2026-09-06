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
    /// Exact captured vocabulary identity summary, when present.
    vocabulary: Option<String>,
    /// Vocabulary-owned type summaries in reader order.
    vocabulary_types: Vec<String>,
    /// Nominal record schema summaries in reader order.
    record_types: Vec<String>,
    /// Typed declaration summaries in reader order.
    declarations: Vec<String>,
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
    let (vocabulary, vocabulary_types) = summarize_vocabulary(document);
    let record_types = document
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
        .collect();
    let declarations = document
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
        .collect();
    let field_provenance = document
        .artifacts()
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
    let reuse_provenance = document
        .artifacts()
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
    let reference_provenance = document
        .artifacts()
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
        vocabulary,
        vocabulary_types,
        record_types,
        declarations,
        field_provenance,
        reuse_provenance,
        reference_provenance,
        diagnostics: Vec::new(),
    }
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
