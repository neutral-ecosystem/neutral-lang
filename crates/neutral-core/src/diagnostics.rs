// SPDX-License-Identifier: Apache-2.0

//! Safe diagnostic values and deterministic canonical ordering.

use crate::{CoreError, SourceLocation};
use std::cmp::Ordering as CompareOrdering;

/// A stable, nonempty diagnostic code.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    /// Creates a stable code when `value` is nonempty ASCII text.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidDiagnosticCode`] for empty or non-ASCII input.
    pub fn new(value: impl Into<String>) -> Result<Self, CoreError> {
        let value = value.into();
        if value.is_empty() || !value.is_ascii() {
            return Err(CoreError::InvalidDiagnosticCode);
        }
        Ok(Self(value))
    }

    /// Returns the stable code text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The layer that owns a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticLayer {
    /// Capture or host-input validation.
    Capture,
    /// Source decoding, lexing, layout, or parsing.
    Syntax,
    /// Semantic validation.
    Semantics,
    /// Typed identity-reference target validation.
    Reference,
    /// Captured vocabulary contract and payload validation.
    Vocabulary,
    /// Reader-facing consumer or probe observation.
    Consumer,
    /// Resource limits or cancellation.
    Resource,
    /// Internal invariant failure.
    Internal,
}

/// The severity assigned to a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticSeverity {
    /// A non-authoritative informational observation.
    Note,
    /// A source or input failure that prevents authoritative output.
    Error,
}

/// A deterministically ordered, safe diagnostic projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// The stable diagnostic identifier.
    code: DiagnosticCode,
    /// The owning compiler or validation layer.
    layer: DiagnosticLayer,
    /// The user-facing severity classification.
    severity: DiagnosticSeverity,
    /// The primary exact source location.
    primary: SourceLocation,
    /// Deterministically ordered source locations related to the primary failure.
    related: Vec<SourceLocation>,
    /// Deterministic safe values used to render the diagnostic.
    parameters: Vec<String>,
    /// Whether detail was bounded by a configured limit.
    truncated: bool,
}

impl Ord for Diagnostic {
    /// Orders diagnostics by source identity, byte range, code, then safe parameters.
    ///
    /// Remaining fields break ties to keep ordering consistent with equality.
    /// Sorting never depends on emission timing or localized rendered messages.
    fn cmp(&self, other: &Self) -> CompareOrdering {
        self.primary
            .cmp(&other.primary)
            .then_with(|| self.code.cmp(&other.code))
            .then_with(|| self.related.cmp(&other.related))
            .then_with(|| self.parameters.cmp(&other.parameters))
            .then_with(|| self.layer.cmp(&other.layer))
            .then_with(|| self.severity.cmp(&other.severity))
            .then_with(|| self.truncated.cmp(&other.truncated))
    }
}

impl PartialOrd for Diagnostic {
    /// Delegates partial comparison to the total canonical diagnostic ordering.
    fn partial_cmp(&self, other: &Self) -> Option<CompareOrdering> {
        Some(self.cmp(other))
    }
}

impl Diagnostic {
    /// Creates a diagnostic with safe, deterministic parameters.
    ///
    /// Safety is the caller's responsibility: parameters are stored as supplied,
    /// not escaped or redacted here. Rendering and input-derived text checks must
    /// happen at the owning diagnostic boundary.
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        layer: DiagnosticLayer,
        severity: DiagnosticSeverity,
        primary: SourceLocation,
        parameters: Vec<String>,
        truncated: bool,
    ) -> Self {
        Self {
            code,
            layer,
            severity,
            primary,
            related: Vec::new(),
            parameters,
            truncated,
        }
    }

    /// Attaches deterministic source locations related to the primary failure.
    #[must_use]
    pub fn with_related(mut self, mut related: Vec<SourceLocation>) -> Self {
        related.sort();
        related.dedup();
        self.related = related;
        self
    }

    /// Returns deterministic source locations related to the primary failure.
    #[must_use]
    pub fn related(&self) -> &[SourceLocation] {
        &self.related
    }

    /// Returns the stable diagnostic code.
    #[must_use]
    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    /// Returns the owning layer.
    #[must_use]
    pub const fn layer(&self) -> DiagnosticLayer {
        self.layer
    }

    /// Returns the severity.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Returns the primary source location.
    #[must_use]
    pub const fn primary(&self) -> SourceLocation {
        self.primary
    }

    /// Returns only safe diagnostic parameters.
    #[must_use]
    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    /// Returns whether parameters or related detail were truncated by limits.
    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }
}

#[cfg(test)]
#[path = "../tests/unit/diagnostics.rs"]
mod tests;
