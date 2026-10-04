// SPDX-License-Identifier: Apache-2.0

//! Independent complete-project reader unit tests.

use neutral_core::{CancellationToken, SourceContentDigest, profile::V1_SOURCE_PROFILE};
use neutral_ir::{LogicalModuleIdentity, project::*, project_interface::ProjectInterface};
use neutral_reader::{ProjectReadError, ValidatedProject};
use std::sync::Arc;

/// Constructs data through public IR contracts without any compiler dependency.
fn artifact() -> ProjectIr {
    let bytes = b"neu \"1.0\"\nmodule empty\n";
    ProjectIr {
        schema: PROJECT_IR_SCHEMA.to_owned(),
        modules: vec![ProjectModule {
            identity: LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "empty"),
            imports: Vec::new(),
        }],
        declarations: Vec::new(),
        vocabulary_records: Vec::new(),
        public_interface: ProjectInterface::with_computed_fingerprint(Vec::new(), Vec::new())
            .unwrap(),
        sources: vec![ProjectSource {
            module: "empty".to_owned(),
            source_id: "unit:empty".to_owned(),
            digest: SourceContentDigest::from_bytes(bytes),
            byte_len: bytes.len() as u64,
        }],
        source_maps: Vec::new(),
        provenance: Vec::new(),
        limits: ProjectLimits {
            modules: 1,
            declarations: 1,
            import_edges: 1,
            nodes: 32,
            text_bytes: 128,
        },
        resources: ProjectResourceFacts {
            source_units: 1,
            source_bytes: bytes.len() as u64,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: 0,
            import_edges: 0,
            value_nodes: 0,
        },
        vocabulary_sources: Vec::new(),
    }
}

/// An independent consumer validates an empty complete module and derives a public view.
#[test]
fn integration_project_reader_without_compiler() {
    let ir = artifact();
    let reader =
        ValidatedProject::from_ir(Arc::new(ir.clone()), ir.limits, &CancellationToken::new())
            .unwrap();
    let view = reader
        .derive_view(
            &ViewRequest {
                schema: PROJECT_VIEW_SCHEMA.to_owned(),
                roots: Vec::new(),
            },
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(view.schema(), PROJECT_VIEW_SCHEMA);
    assert_eq!(view.exports(), []);
    assert_eq!(reader.complete_ir().as_ref(), &ir);
}

/// Unknown schemas are rejected before any untrusted content is traversed.
#[test]
fn unit_project_schema_is_required() {
    assert_ne!(PROJECT_IR_SCHEMA, PROJECT_VIEW_SCHEMA);
    assert_eq!(
        PROJECT_MAX_DEPTH,
        neutral_ir::project_interface::MAX_PROJECT_INTERFACE_TYPE_DEPTH
    );
    let mut ir = artifact();
    ir.schema = PROJECT_VIEW_SCHEMA.to_owned();
    assert_eq!(
        ValidatedProject::from_ir(Arc::new(ir.clone()), ir.limits, &CancellationToken::new())
            .unwrap_err(),
        ProjectReadError::Schema
    );
}

/// Producer bounds cannot be relaxed by a more permissive caller.
#[test]
fn security_project_reader_intersects_producer_bounds() {
    let mut ir = artifact();
    let caller = ir.limits;
    ir.limits.modules = 0;
    assert_eq!(
        ValidatedProject::from_ir(Arc::new(ir), caller, &CancellationToken::new()).unwrap_err(),
        ProjectReadError::Limit
    );
    let mut oversized = artifact();
    oversized.modules[0].identity = LogicalModuleIdentity::new(
        V1_SOURCE_PROFILE,
        "a".repeat(usize::try_from(oversized.limits.text_bytes).unwrap() + 1),
    );
    assert_eq!(
        ValidatedProject::from_ir(
            Arc::new(oversized.clone()),
            oversized.limits,
            &CancellationToken::new()
        )
        .unwrap_err(),
        ProjectReadError::Limit
    );
}

/// A forged export cannot trigger deep cloning before its shared schema bound is checked.
#[test]
fn security_project_reader_bounds_export_types_before_cloning() {
    let mut ir = artifact();
    let mut ty = neutral_ir::project_interface::ProjectPublicType::Num;
    for _ in 0..=PROJECT_MAX_DEPTH {
        ty = neutral_ir::project_interface::ProjectPublicType::Ref(Box::new(ty));
    }
    ir.public_interface = ProjectInterface::from_parts(
        vec![neutral_ir::project_interface::ProjectPublicExport::new(
            neutral_ir::ModuleSymbolIdentity::new(ir.modules[0].identity.clone(), "forged"),
            neutral_ir::project_interface::ProjectPublicSignature::Binding(ty),
        )],
        Vec::new(),
        ir.public_interface.fingerprint(),
    );
    assert_eq!(
        ValidatedProject::from_ir(Arc::new(ir.clone()), ir.limits, &CancellationToken::new())
            .unwrap_err(),
        ProjectReadError::Limit
    );
}
