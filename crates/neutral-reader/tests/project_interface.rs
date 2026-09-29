// SPDX-License-Identifier: Apache-2.0

//! Compiler-independent validation of public interface snapshots.

use neutral_core::{SemanticDigest, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    project_interface::{
        ProjectInterface, ProjectPublicEdge, ProjectPublicEdgeKind, ProjectPublicExport,
        ProjectPublicSignature, ProjectPublicType,
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
