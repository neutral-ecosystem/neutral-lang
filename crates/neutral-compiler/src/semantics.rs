// SPDX-License-Identifier: Apache-2.0

//! Private minimal semantic validation and public IR lowering.

use crate::{CompilationFailure, CompilationFailureDetail, LANGUAGE_BEHAVIOR_VERSION};
use neutral_core::{
    ByteSpan, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity, ResultClass,
    SourceContentDigest, SourceLocation, StructuralLimits,
};
use neutral_ir::{
    CompilationArtifacts, Declaration, DeclarationFingerprint, DerivationManifest, ElementId,
    ExactNumber, LogicalDocument, LogicalModuleIdentity, LogicalValue, ModuleSymbolIdentity,
    Normalization, ProvenanceRecord, ResolvedType, ResourceFacts, SourceMap, SourceMapEntry,
    ValueOrigin,
};

/// Stable semantic diagnostic for an invalid minimal module or binding name.
const INVALID_NAME: &str = "NEU-NAME-001";
/// Stable semantic diagnostic for attempting to redeclare a protected core name.
const PROTECTED_NAME: &str = "NEU-NAME-002";
/// Stable semantic diagnostic for an invalid exact numeric value.
const INVALID_NUMBER: &str = "NEU-VAL-001";

/// A private semantic failure before authoritative IR construction.
pub(super) struct SemanticError {
    /// Stable semantic diagnostic code.
    code: &'static str,
    /// Exact original-byte primary span.
    span: ByteSpan,
}

impl SemanticError {
    /// Converts a private semantic failure into a bounded public failure.
    pub(super) fn into_failure(self, source: SourceContentDigest) -> CompilationFailure {
        let code = DiagnosticCode::new(self.code).expect("semantic diagnostic code must be ASCII");
        CompilationFailure {
            class: ResultClass::Semantics,
            detail: CompilationFailureDetail::SemanticRejected,
            diagnostics: vec![Diagnostic::new(
                code,
                DiagnosticLayer::Semantics,
                DiagnosticSeverity::Error,
                SourceLocation::new(source, self.span),
                Vec::new(),
                false,
            )],
        }
    }
}

/// Validates the private minimal model and lowers complete immutable artifacts.
pub(super) fn lower(
    unit: crate::frontend::ParsedUnit,
    source_digest: SourceContentDigest,
    source_length: usize,
    limits: StructuralLimits,
) -> Result<CompilationArtifacts, SemanticError> {
    validate_snake_name(&unit.module.name, unit.module.span)?;
    validate_snake_name(&unit.binding.name, unit.binding.name_span)?;
    if is_protected_name(&unit.binding.name) {
        return Err(SemanticError {
            code: PROTECTED_NAME,
            span: unit.binding.name_span,
        });
    }

    let number =
        ExactNumber::from_unsigned_integer(&unit.binding.number).map_err(|_| SemanticError {
            code: INVALID_NUMBER,
            span: unit.binding.value_span,
        })?;
    let value = LogicalValue::Number(number);
    let resolved_type = ResolvedType::Num;
    let module_identity =
        LogicalModuleIdentity::new(LANGUAGE_BEHAVIOR_VERSION, unit.module.name.clone());
    let symbol_identity =
        ModuleSymbolIdentity::new(module_identity.clone(), unit.binding.name.clone());
    let fingerprint =
        DeclarationFingerprint::for_binding(resolved_type, &value).map_err(|_| SemanticError {
            code: INVALID_NUMBER,
            span: unit.binding.value_span,
        })?;
    let element_id = ElementId::new(0);
    let declaration = Declaration::new(
        element_id,
        symbol_identity,
        fingerprint,
        unit.binding.name,
        resolved_type,
        value,
    );
    let logical_document = LogicalDocument::new(module_identity, vec![declaration]);
    let source_byte_length = u64::try_from(source_length).unwrap_or(u64::MAX);
    let source_map = SourceMap::new(
        source_digest,
        source_byte_length,
        unit.module.span,
        vec![SourceMapEntry::new(
            element_id,
            unit.binding.span,
            unit.binding.type_span,
            unit.binding.name_span,
            unit.binding.value_span,
        )],
    );
    let provenance = vec![ProvenanceRecord::new(
        element_id,
        ValueOrigin::ExplicitSource,
        Normalization::ExactNumberCanonicalization,
    )];
    let resource_facts = ResourceFacts::new(source_byte_length, 1, 0);
    let derivation = DerivationManifest::new(
        LANGUAGE_BEHAVIOR_VERSION,
        source_digest,
        limits.source_bytes(),
        limits.diagnostics(),
        resource_facts,
    );
    Ok(CompilationArtifacts::new(
        logical_document,
        source_map,
        provenance,
        derivation,
    ))
}

/// Validates the frozen ASCII `snake_name` category without locale behavior.
fn validate_snake_name(value: &str, span: ByteSpan) -> Result<(), SemanticError> {
    let valid = !value.is_empty()
        && value.split('_').all(|segment| {
            let mut bytes = segment.bytes();
            bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
                && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
    if valid {
        Ok(())
    } else {
        Err(SemanticError {
            code: INVALID_NAME,
            span,
        })
    }
}

/// Returns whether a binding name belongs to the frozen protected core set.
fn is_protected_name(value: &str) -> bool {
    matches!(
        value,
        "num"
            | "string"
            | "bool"
            | "List"
            | "Ref"
            | "neu"
            | "module"
            | "use"
            | "record"
            | "true"
            | "false"
            | "null"
            | "ref"
    )
}

#[cfg(test)]
/// Unit tests for minimal semantic validation and lowering.
mod tests {
    use super::{is_protected_name, validate_snake_name};
    use neutral_core::ByteSpan;

    #[test]
    /// Verifies exact ASCII snake names and rejects malformed categories.
    fn unit_semantics_validates_minimal_names() {
        let span = ByteSpan::new(0, 1).expect("test span should be valid");
        assert!(validate_snake_name("answer_two2", span).is_ok());
        assert!(validate_snake_name("Answer", span).is_err());
        assert!(validate_snake_name("answer__two", span).is_err());
    }

    #[test]
    /// Verifies the frozen core namespace cannot be redeclared by a binding.
    fn unit_semantics_protects_core_names() {
        assert!(is_protected_name("num"));
        assert!(is_protected_name("module"));
        assert!(!is_protected_name("answer"));
    }
}
