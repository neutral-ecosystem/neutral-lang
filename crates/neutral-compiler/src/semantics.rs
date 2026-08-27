// SPDX-License-Identifier: Apache-2.0

//! Private minimal semantic validation and public IR lowering.

use crate::diagnostics;
use crate::frontend::{ParsedType, ParsedValue};
use crate::language::names;
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

/// A private semantic failure before authoritative IR construction.
pub(super) struct SemanticError {
    /// Stable semantic diagnostic code.
    code: &'static str,
    /// Exact original-byte primary span.
    span: ByteSpan,
    /// Stable failure class.
    class: ResultClass,
    /// Public bounded failure detail.
    detail: CompilationFailureDetail,
    /// Diagnostic ownership layer.
    layer: DiagnosticLayer,
}

impl SemanticError {
    /// Creates one semantic type, name, or value failure.
    fn semantic(code: &'static str, span: ByteSpan) -> Self {
        Self {
            code,
            span,
            class: ResultClass::Semantics,
            detail: CompilationFailureDetail::SemanticRejected,
            layer: DiagnosticLayer::Semantics,
        }
    }

    /// Creates one deterministic captured resource-limit failure.
    fn resource(code: &'static str, span: ByteSpan) -> Self {
        Self {
            code,
            span,
            class: ResultClass::Resource,
            detail: CompilationFailureDetail::ResourceLimitExceeded,
            layer: DiagnosticLayer::Resource,
        }
    }

    /// Converts a private semantic failure into a bounded public failure.
    pub(super) fn into_failure(self, source: SourceContentDigest) -> CompilationFailure {
        let code = DiagnosticCode::new(self.code).expect("semantic diagnostic code must be ASCII");
        CompilationFailure {
            class: self.class,
            detail: self.detail,
            diagnostics: vec![Diagnostic::new(
                code,
                self.layer,
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
    let _retained_private_trivia_count = unit.trivia_count();
    if names::is_protected_name(&unit.module.name) {
        return Err(SemanticError::semantic(
            diagnostics::PROTECTED_NAME,
            unit.module.name_span,
        ));
    }
    validate_snake_name(&unit.module.name, unit.module.name_span)?;
    if names::is_protected_name(&unit.binding.name) {
        return Err(SemanticError::semantic(
            diagnostics::PROTECTED_NAME,
            unit.binding.name_span,
        ));
    }
    validate_snake_name(&unit.binding.name, unit.binding.name_span)?;

    let (resolved_type, value, normalization, decoded_string_bytes) =
        lower_scalar(&unit.binding, limits)?;
    let module_identity =
        LogicalModuleIdentity::new(LANGUAGE_BEHAVIOR_VERSION, unit.module.name.clone());
    let symbol_identity =
        ModuleSymbolIdentity::new(module_identity.clone(), unit.binding.name.clone());
    let fingerprint = DeclarationFingerprint::for_binding(resolved_type, &value).map_err(|_| {
        SemanticError::semantic(diagnostics::INVALID_NUMBER, unit.binding.value_span)
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
        normalization,
    )];
    let resource_facts = ResourceFacts::new(source_byte_length, 1, 0, decoded_string_bytes);
    let derivation = DerivationManifest::new(
        LANGUAGE_BEHAVIOR_VERSION,
        source_digest,
        limits.source_bytes(),
        limits.diagnostics(),
        limits.string_bytes(),
        resource_facts,
    );
    Ok(CompilationArtifacts::new(
        logical_document,
        source_map,
        provenance,
        derivation,
    ))
}

/// Type-checks and lowers one active explicit scalar literal.
fn lower_scalar(
    binding: &crate::frontend::ParsedBinding,
    limits: StructuralLimits,
) -> Result<(ResolvedType, LogicalValue, Normalization, u64), SemanticError> {
    match (&binding.declared_type, &binding.value) {
        (ParsedType::Num, ParsedValue::Number(spelling)) => {
            let number = ExactNumber::from_unsigned_integer(spelling).map_err(|_| {
                SemanticError::semantic(diagnostics::INVALID_NUMBER, binding.value_span)
            })?;
            Ok((
                ResolvedType::Num,
                LogicalValue::Number(number),
                Normalization::ExactNumberCanonicalization,
                0,
            ))
        }
        (ParsedType::String, ParsedValue::String(value)) => {
            let decoded_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
            if decoded_bytes > limits.string_bytes() {
                return Err(SemanticError::resource(
                    diagnostics::STRING_LIMIT_EXCEEDED,
                    binding.value_span,
                ));
            }
            Ok((
                ResolvedType::String,
                LogicalValue::String(value.clone()),
                Normalization::StringEscapeDecoding,
                decoded_bytes,
            ))
        }
        (ParsedType::Bool, ParsedValue::Boolean(value)) => Ok((
            ResolvedType::Bool,
            LogicalValue::Boolean(*value),
            Normalization::BooleanIdentity,
            0,
        )),
        _ => Err(SemanticError::semantic(
            diagnostics::TYPE_MISMATCH,
            binding.value_span,
        )),
    }
}

/// Validates the frozen ASCII `snake_name` category without locale behavior.
fn validate_snake_name(value: &str, span: ByteSpan) -> Result<(), SemanticError> {
    if classify_ascii_name(value) == AsciiNameCategory::Snake {
        Ok(())
    } else {
        Err(SemanticError::semantic(diagnostics::INVALID_NAME, span))
    }
}

/// Complete frozen ASCII identifier categories, independent of locale behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AsciiNameCategory {
    /// `[a-z][a-z0-9]*("_"[a-z][a-z0-9]*)*`.
    Snake,
    /// `[A-Z][A-Za-z0-9]*`.
    Upper,
    /// Any other spelling.
    Invalid,
}

/// Classifies one spelling into the frozen ASCII identifier categories.
fn classify_ascii_name(value: &str) -> AsciiNameCategory {
    let snake = !value.is_empty()
        && value.split('_').all(|segment| {
            let mut bytes = segment.bytes();
            bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
                && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
    if snake {
        return AsciiNameCategory::Snake;
    }
    let mut bytes = value.bytes();
    let upper = bytes.next().is_some_and(|byte| byte.is_ascii_uppercase())
        && bytes.all(|byte| byte.is_ascii_alphanumeric());
    if upper {
        AsciiNameCategory::Upper
    } else {
        AsciiNameCategory::Invalid
    }
}

#[cfg(test)]
/// Unit tests for minimal semantic validation and lowering.
mod tests {
    use super::{AsciiNameCategory, classify_ascii_name, validate_snake_name};
    use crate::language::names;
    use neutral_core::ByteSpan;

    #[test]
    /// Verifies exact ASCII snake names and rejects malformed categories.
    fn unit_semantics_validates_minimal_names() {
        let span = ByteSpan::new(0, 1).expect("test span should be valid");
        assert!(validate_snake_name("answer_two2", span).is_ok());
        assert!(validate_snake_name("Answer", span).is_err());
        assert!(validate_snake_name("answer__two", span).is_err());
        assert_eq!(classify_ascii_name("Record2"), AsciiNameCategory::Upper);
        assert_eq!(
            classify_ascii_name("Record_Name"),
            AsciiNameCategory::Invalid
        );
    }

    #[test]
    /// Verifies the frozen core namespace cannot be redeclared by a binding.
    fn unit_semantics_protects_core_names() {
        assert!(names::is_protected_name(names::NUM));
        assert!(names::is_protected_name(names::MODULE));
        assert!(!names::is_protected_name("answer"));
    }
}
