// SPDX-License-Identifier: Apache-2.0

//! Reader-only inspection support for Neutral artifacts.
//!
//! This library owns generic traversal and consumer-facing observations over
//! validated reader views. It must remain independent of the compiler, private
//! frontend models, test support, and host acquisition behavior.

use neutral_core::{Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity};
use neutral_reader::{ElementId, ValidatedDocument};

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
    /// Typed declaration summaries in reader order.
    declarations: Vec<String>,
    /// Safe diagnostic-code summaries.
    diagnostics: Vec<String>,
}

impl ProbeSummary {
    /// Returns the logical module name.
    #[must_use]
    pub fn module(&self) -> &str {
        &self.module
    }

    /// Returns deterministic typed declaration summaries.
    #[must_use]
    pub fn declarations(&self) -> &[String] {
        &self.declarations
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
    ProbeSummary {
        module: document.module_name().to_owned(),
        declarations,
        diagnostics: Vec::new(),
    }
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
