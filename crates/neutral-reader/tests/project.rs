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
            artifact_bytes: 4096,
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

/// All identity failures retain the same public result envelope.
#[test]
fn project_identity_failures_expose_the_shared_schema() {
    use neutral_reader::ProjectIdentityReadError as E;
    for error in [
        E::CaptureMismatch,
        E::View(ProjectReadError::Cancelled),
        E::Identity(neutral_ir::project_identity::IdentityError::Cancelled),
    ] {
        assert_eq!(error.schema(), neutral_reader::PROJECT_RESULT_SCHEMA);
    }
}

/// Repeated or reversed imports fail even when all targets are known distinct modules.
#[test]
fn project_modules_require_strict_import_order() {
    let mut ir = artifact();
    for name in ["left", "right"] {
        ir.modules.push(ProjectModule {
            identity: LogicalModuleIdentity::new(V1_SOURCE_PROFILE, name),
            imports: vec![],
        });
        let mut source = ir.sources[0].clone();
        source.module = name.into();
        source.source_id = format!("unit:{name}");
        ir.sources.push(source);
    }
    ir.limits.modules = 3;
    ir.limits.import_edges = 2;
    ir.resources.source_units = 3;
    ir.resources.source_bytes = ir.sources.iter().map(|source| source.byte_len).sum();
    ir.resources.import_edges = 2;
    ir.modules[0].imports = vec!["left".into(), "right".into()];
    assert!(
        ValidatedProject::from_ir(Arc::new(ir.clone()), ir.limits, &CancellationToken::new())
            .is_ok()
    );
    for imports in [
        vec!["right".into(), "left".into()],
        vec!["left".into(), "left".into()],
    ] {
        ir.modules[0].imports = imports;
        assert_eq!(
            ValidatedProject::from_ir(Arc::new(ir.clone()), ir.limits, &CancellationToken::new())
                .unwrap_err(),
            ProjectReadError::Module
        );
    }
}

/// Complete vocabulary schemas preserve all canonical field names and reject duplicate order.
#[test]
fn project_vocabulary_fields_require_strict_order() {
    use neutral_ir::project_interface::{ProjectPublicType as T, ProjectPublicVocabulary};
    let mut ir = artifact();
    ir.public_interface = ProjectInterface::with_vocabularies(
        vec![ProjectPublicVocabulary::new(
            "Fixture",
            "1.0.0",
            vec!["Record".into()],
        )],
        vec![],
        vec![],
    )
    .unwrap();
    ir.vocabulary_records.push(ProjectVocabularyRecord {
        identity: "Fixture".into(),
        version: "1.0.0".into(),
        name: "Record".into(),
        public: true,
        fields: vec![("first".into(), T::Num), ("second".into(), T::Bool)],
    });
    ir.vocabulary_sources.push(ProjectVocabularySource {
        identity: "Fixture".into(),
        version: "1.0.0".into(),
        digest: neutral_core::VocabularyContentDigest::from_bytes(b"fixture"),
        byte_len: 7,
    });
    ir.resources.vocabulary_units = 1;
    ir.resources.vocabulary_bytes = 7;
    ValidatedProject::from_ir(Arc::new(ir.clone()), ir.limits, &CancellationToken::new()).unwrap();
    ir.vocabulary_records[0].fields.reverse();
    assert_eq!(
        ValidatedProject::from_ir(Arc::new(ir), artifact().limits, &CancellationToken::new())
            .unwrap_err(),
        ProjectReadError::Declaration
    );
}
