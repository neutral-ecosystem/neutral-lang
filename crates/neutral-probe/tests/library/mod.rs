// SPDX-License-Identifier: Apache-2.0

//! Crate-local tests for complete probe-library projections.

use super::*;
use neutral_core::{ByteSpan, SourceContentDigest, StructuralLimits};
use neutral_ir::{
    AcceptancePartition, CompilationArtifacts, Declaration, DeclarationFingerprint,
    DerivationManifest, ExactNumber, LANGUAGE_BEHAVIOR_VERSION, LogicalDocument,
    LogicalModuleIdentity, LogicalValue, ModuleSymbolIdentity, Normalization, ProvenanceRecord,
    ResolvedType, ResourceFacts, SourceMap, SourceMapEntry, ValueOrigin,
};
use neutral_reader::ValidatedDocument;
use std::sync::Arc;

/// Exact source represented by the direct public-IR fixture.
const SOURCE: &[u8] = b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n";
/// Module-header end in the direct public-IR fixture.
const MODULE_END: u64 = 24;
/// Declaration start in the direct public-IR fixture.
const DECLARATION_START: u64 = 26;

/// Builds one validated reader without compiler linkage.
fn reader_fixture() -> ValidatedDocument {
    let element_id = ElementId::new(1);
    let module = LogicalModuleIdentity::new(LANGUAGE_BEHAVIOR_VERSION, "sample");
    let value = LogicalValue::Number(
        ExactNumber::from_unsigned_integer("42").expect("fixture number should normalize"),
    );
    let fingerprint = DeclarationFingerprint::for_binding(&ResolvedType::Num, &value)
        .expect("fixture fingerprint should fit");
    let declaration = Declaration::new(
        element_id,
        ModuleSymbolIdentity::new(module.clone(), "answer"),
        fingerprint,
        "answer",
        ResolvedType::Num,
        value,
    );
    let source_length = u64::try_from(SOURCE.len()).expect("source length should fit");
    let declaration_span =
        ByteSpan::new(DECLARATION_START, source_length).expect("span should be valid");
    let source_digest = SourceContentDigest::from_bytes(SOURCE);
    let source_map = SourceMap::new(
        source_digest,
        source_length,
        ByteSpan::new(0, MODULE_END).expect("module span should be valid"),
        vec![SourceMapEntry::new(
            element_id,
            declaration_span,
            declaration_span,
            declaration_span,
            declaration_span,
        )],
    );
    let limits = StructuralLimits::new(4_096, 16).expect("fixture limits should be valid");
    let artifacts = CompilationArtifacts::new(
        LogicalDocument::new(module, vec![declaration]),
        source_map,
        vec![ProvenanceRecord::new(
            element_id,
            ValueOrigin::ExplicitSource,
            Normalization::ExactNumberCanonicalization,
        )],
        DerivationManifest::new(
            LANGUAGE_BEHAVIOR_VERSION,
            source_digest,
            AcceptancePartition::from_limits(limits),
            ResourceFacts::new(source_length, 1, 0, 0),
        ),
    );
    ValidatedDocument::from_compiler_output(Arc::new(artifacts))
        .expect("direct fixture should satisfy reader invariants")
}

#[test]
/// Verifies summary accessors, rendering, and source-linked diagnostics together.
fn probe_summary_exposes_every_projection() {
    let document = reader_fixture();
    let summary = summarize(&document);
    assert_eq!(summary.module(), "sample");
    assert!(!summary.metadata().is_empty());
    assert_eq!(summary.vocabulary(), None);
    assert!(summary.vocabulary_types().is_empty());
    assert_eq!(summary.record_types(), Vec::<String>::new());
    assert_eq!(summary.declarations().len(), 1);
    assert_eq!(summary.source_mappings().len(), 1);
    assert_eq!(summary.value_provenance().len(), 1);
    assert!(summary.field_provenance().is_empty());
    assert!(summary.reuse_provenance().is_empty());
    assert!(summary.reference_provenance().is_empty());
    assert!(summary.diagnostics().is_empty());
    assert!(!render_summary(&summary).is_empty());

    let diagnostic = source_linked_diagnostic(&document, ElementId::new(1))
        .expect("known element should map to source");
    assert_eq!(diagnostic.code().as_str(), diagnostics::OBSERVATION);
    assert_eq!(
        source_linked_diagnostic(&document, ElementId::new(99)),
        Err(ProbeError::UnknownElement)
    );
}

#[test]
/// Verifies rendering includes each optional summary category in its documented order.
fn renderer_preserves_all_summary_categories() {
    let summary = ProbeSummary {
        module: "sample".into(),
        metadata: vec!["meta".into()],
        vocabulary: Some("Fixture".into()),
        vocabulary_types: vec!["Entry".into()],
        record_types: vec!["Record".into()],
        declarations: vec!["answer".into()],
        source_mappings: vec!["span".into()],
        value_provenance: vec!["value".into()],
        field_provenance: vec!["field".into()],
        reuse_provenance: vec!["reuse".into()],
        reference_provenance: vec!["reference".into()],
        diagnostics: vec!["observation".into()],
    };
    assert_eq!(
        render_summary(&summary),
        [
            "module sample",
            "metadata meta",
            "vocabulary Fixture",
            "record Record",
            "vocabulary-type Entry",
            "declaration answer",
            "source-map span",
            "value-provenance value",
            "field-provenance field",
            "reuse-provenance reuse",
            "reference-provenance reference",
            "diagnostic observation"
        ]
    );
}
