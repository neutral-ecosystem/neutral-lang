// SPDX-License-Identifier: Apache-2.0

//! Compiler-independent validation of public interface snapshots.

use neutral_core::{SemanticDigest, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    project_interface::{
        ProjectInterface, ProjectPublicEdge, ProjectPublicEdgeKind, ProjectPublicExport,
        ProjectPublicSignature, ProjectPublicType, ProjectPublicVocabulary,
    },
};
use neutral_reader::{ProjectInterfaceError, ValidatedProjectInterface};
use std::sync::Arc;

/// Builds one exact v1 module-symbol identity for a reader fixture.
fn identity(module: &str, name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new(V1_SOURCE_PROFILE, module), name)
}

/// Builds one valid public-only interface without a compiler dependency.
fn valid_interface() -> ProjectInterface {
    ProjectInterface::with_computed_fingerprint(
        vec![
            ProjectPublicExport::new(
                identity("api::consumer", "copied"),
                ProjectPublicSignature::Binding(ProjectPublicType::Num),
            ),
            ProjectPublicExport::new(
                identity("api::source", "answer"),
                ProjectPublicSignature::Binding(ProjectPublicType::Num),
            ),
        ],
        vec![ProjectPublicEdge::new(
            identity("api::consumer", "copied"),
            identity("api::source", "answer"),
            ProjectPublicEdgeKind::Value,
        )],
    )
    .expect("reader fixture transcript")
}

/// Builds a compiler-independent catalogue and one public vocabulary signature.
fn vocabulary_interface() -> ProjectInterface {
    ProjectInterface::with_vocabularies(
        vec![ProjectPublicVocabulary::new(
            "Alpha",
            "1.0.0",
            vec!["Visible".to_owned()],
        )],
        vec![ProjectPublicExport::new(
            identity("api", "item"),
            ProjectPublicSignature::Binding(ProjectPublicType::VocabularyNominal {
                identity: "Alpha".to_owned(),
                version: "1.0.0".to_owned(),
                name: "Visible".to_owned(),
            }),
        )],
        Vec::new(),
    )
    .expect("canonical vocabulary transcript")
}

#[test]
/// Reader exposes the locked catalogue without any source alias or metadata.
fn exposes_canonical_vocabulary_facts() {
    let view = ValidatedProjectInterface::from_interface(Arc::new(vocabulary_interface()))
        .expect("valid catalogue");
    assert_eq!(view.vocabularies().len(), 1);
    assert_eq!(view.vocabularies()[0].identity(), "Alpha");
    assert_eq!(view.vocabularies()[0].version(), "1.0.0");
    assert_eq!(view.vocabularies()[0].public_types(), ["Visible"]);
}

#[test]
/// Reader rejects missing locks, release mismatches, and inaccessible types.
fn rejects_unlocked_or_private_vocabulary_signatures() {
    let valid = vocabulary_interface();
    let missing = ProjectInterface::with_computed_fingerprint(valid.exports().to_vec(), Vec::new())
        .expect("framed missing catalogue");
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(missing)).expect_err("missing lock"),
        ProjectInterfaceError::InvalidSignature
    );
    for vocabulary in [
        ProjectPublicVocabulary::new("Alpha", "2.0.0", vec!["Visible".to_owned()]),
        ProjectPublicVocabulary::new("Alpha", "1.0.0", vec!["Other".to_owned()]),
    ] {
        let invalid = ProjectInterface::with_vocabularies(
            vec![vocabulary],
            valid.exports().to_vec(),
            Vec::new(),
        )
        .expect("framed invalid catalogue");
        assert_eq!(
            ValidatedProjectInterface::from_interface(Arc::new(invalid))
                .expect_err("unmatched signature"),
            ProjectInterfaceError::InvalidSignature
        );
    }
}

#[test]
/// Reader rejects noncanonical catalogue order even with a valid digest.
fn rejects_duplicate_or_unordered_vocabulary_catalogue() {
    let valid = vocabulary_interface();
    for vocabularies in [
        vec![
            ProjectPublicVocabulary::new("Alpha", "1.0.0", vec!["Visible".to_owned()]),
            ProjectPublicVocabulary::new("Alpha", "1.0.0", vec!["Visible".to_owned()]),
        ],
        vec![
            ProjectPublicVocabulary::new("Beta", "1.0.0", Vec::new()),
            ProjectPublicVocabulary::new("Alpha", "1.0.0", vec!["Visible".to_owned()]),
        ],
        vec![ProjectPublicVocabulary::new(
            "Alpha",
            "1.0.0",
            vec!["Visible".to_owned(), "Visible".to_owned()],
        )],
    ] {
        let invalid =
            ProjectInterface::with_vocabularies(vocabularies, valid.exports().to_vec(), Vec::new())
                .expect("framed invalid order");
        assert_eq!(
            ValidatedProjectInterface::from_interface(Arc::new(invalid))
                .expect_err("noncanonical catalogue"),
            ProjectInterfaceError::InvalidVocabulary
        );
    }
}

#[test]
/// Altering a canonical revision without updating its digest is detected.
fn rejects_stale_vocabulary_fingerprint() {
    let valid = vocabulary_interface();
    let stale = ProjectInterface::from_parts_with_vocabularies(
        vec![
            ProjectPublicVocabulary::new("Alpha", "1.0.0", vec!["Visible".to_owned()]),
            ProjectPublicVocabulary::new("Beta", "2.0.0", Vec::new()),
        ],
        valid.exports().to_vec(),
        valid.edges().to_vec(),
        valid.fingerprint(),
    );
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(stale)).expect_err("forged revision"),
        ProjectInterfaceError::InvalidFingerprint
    );
}

#[test]
/// Reader traverses canonical public exports and cross-module reuse.
fn accepts_public_only_interface() {
    let view = ValidatedProjectInterface::from_interface(Arc::new(valid_interface()))
        .expect("valid interface");
    assert_eq!(view.exports().len(), 2);
    assert_eq!(view.cross_module_values().count(), 1);
    assert_eq!(view.cross_module_references().count(), 0);
    assert!(
        view.export_by_identity(&identity("api::source", "answer"))
            .is_some()
    );
}

#[test]
/// A stale digest never becomes a validated reader view.
fn rejects_stale_public_fingerprint() {
    let valid = valid_interface();
    let stale = ProjectInterface::from_parts(
        valid.exports().to_vec(),
        valid.edges().to_vec(),
        SemanticDigest::from_raw_bytes([0; 32]),
    );
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(stale)).expect_err("stale"),
        ProjectInterfaceError::InvalidFingerprint
    );
}

#[test]
/// Unsorted public exports are rejected before binary-search access.
fn rejects_unsorted_exports() {
    let valid = valid_interface();
    let mut exports = valid.exports().to_vec();
    exports.reverse();
    let unordered = ProjectInterface::with_computed_fingerprint(exports, valid.edges().to_vec())
        .expect("framed unordered interface");
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(unordered)).expect_err("unordered"),
        ProjectInterfaceError::InvalidExport
    );
}

#[test]
/// An edge to an unexported target is rejected without source access.
fn rejects_non_public_edge_target() {
    let valid = valid_interface();
    let private = identity("api::source", "hidden");
    let dangling = ProjectInterface::with_computed_fingerprint(
        valid.exports().to_vec(),
        vec![ProjectPublicEdge::new(
            identity("api::consumer", "copied"),
            private,
            ProjectPublicEdgeKind::Value,
        )],
    )
    .expect("framed dangling interface");
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(dangling)).expect_err("dangling"),
        ProjectInterfaceError::InvalidEdge
    );
}

#[test]
/// A public nominal signature requires exactly its matching type edge.
fn rejects_missing_and_spurious_public_type_edges() {
    let record = ProjectPublicExport::new(
        identity("api::source", "Service"),
        ProjectPublicSignature::Record(Vec::new()),
    );
    let binding = ProjectPublicExport::new(
        identity("api::source", "service"),
        ProjectPublicSignature::Binding(ProjectPublicType::Nominal(record.identity().clone())),
    );
    let exports = vec![record.clone(), binding.clone()];
    let missing = ProjectInterface::with_computed_fingerprint(exports.clone(), Vec::new())
        .expect("framed missing edge");
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(missing))
            .expect_err("missing type edge"),
        ProjectInterfaceError::InvalidEdge
    );
    let wrong_kind = ProjectInterface::with_computed_fingerprint(
        exports.clone(),
        vec![ProjectPublicEdge::new(
            binding.identity().clone(),
            record.identity().clone(),
            ProjectPublicEdgeKind::ReferenceType,
        )],
    )
    .expect("framed wrong-kind edge");
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(wrong_kind))
            .expect_err("wrong type edge"),
        ProjectInterfaceError::InvalidEdge
    );
    let valid = ProjectInterface::with_computed_fingerprint(
        exports,
        vec![ProjectPublicEdge::new(
            binding.identity().clone(),
            record.identity().clone(),
            ProjectPublicEdgeKind::Type,
        )],
    )
    .expect("framed valid edge");
    assert!(ValidatedProjectInterface::from_interface(Arc::new(valid)).is_ok());
}
