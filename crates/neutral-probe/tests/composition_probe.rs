// SPDX-License-Identifier: Apache-2.0

//! Successor probe entrypoints reject incomplete inputs at their independent boundaries.

use neutral_core::{
    CancellationToken, SemanticDigest, SourceContentDigest, StructuralLimits, allocation::Shared,
    profile::V1_SOURCE_PROFILE,
};
use neutral_encoding::{DecodeErrorClass, DecodeLimits, composition::encode_composition_project};
use neutral_ir::{
    LogicalModuleIdentity,
    composition::project::{CompositionProjectIr, CompositionResourceFacts},
    project::{ProjectModule, ProjectResourceFacts, ProjectSource},
    project_identity::{IdentityLimits, MAX_TRANSCRIPT_BYTES, composition_interface},
    project_interface::ProjectPublicType,
};
use neutral_probe::composition::{
    CompositionEntryProbeRequest, CompositionProbeError, inspect_composition_encoded,
    inspect_composition_entries_encoded,
};
use neutral_reader::{
    ProjectLimits,
    composition::{CompositionScalarLimits, ProjectCompositionLimits, profile},
    consumer::{
        CONSUMER_PROFILE, ConsumerCompatibilityRequest, ConsumerLimits, ENTRY_REQUEST_SCHEMA,
        EntryRequest, EntrySelection,
    },
};

fn limits() -> ProjectCompositionLimits {
    ProjectCompositionLimits::from_vocabulary(CompositionScalarLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ))
}

fn empty_project() -> neutral_reader::composition::ValidatedCompositionProject {
    let source = b"neu \"1.0\"\nmodule example\n";
    let composition = limits();
    let mut ir = CompositionProjectIr {
        schema: profile::PROJECT_IR_SCHEMA.to_owned(),
        modules: vec![ProjectModule {
            identity: LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "example"),
            imports: vec![],
        }],
        declarations: vec![],
        vocabularies: vec![],
        interface_digest: SemanticDigest::from_raw_bytes([0; 32]),
        sources: vec![ProjectSource {
            module: "example".into(),
            source_id: "source:example".into(),
            digest: SourceContentDigest::from_bytes(source),
            byte_len: source.len() as u64,
        }],
        source_maps: vec![],
        provenance: vec![],
        limits: neutral_encoding::project::hard_project_limits(),
        resources: ProjectResourceFacts {
            source_units: 1,
            source_bytes: source.len() as u64,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: 0,
            import_edges: 0,
            value_nodes: 0,
        },
        vocabulary_sources: vec![],
        origins: vec![],
        composition_limits: composition.policy(),
        composition_resources: CompositionResourceFacts::from_values([0; 7]),
    };
    ir.interface_digest = composition_interface(
        &ir,
        IdentityLimits {
            bytes: MAX_TRANSCRIPT_BYTES,
            nodes: 65_536,
        },
        &CancellationToken::new(),
    )
    .unwrap()
    .identity();
    neutral_reader::composition::ValidatedCompositionProject::from_ir(
        Shared::try_new(ir).unwrap(),
        neutral_encoding::project::hard_project_limits(),
        composition,
        &CancellationToken::new(),
    )
    .unwrap()
}

#[test]
fn successor_probe_entrypoints_keep_decode_and_negotiation_errors_atomic() {
    let cancel = CancellationToken::new();
    assert!(matches!(
        inspect_composition_encoded(
            &[],
            DecodeLimits::hard(),
            neutral_encoding::project::hard_project_limits(),
            limits(),
            None,
            &cancel,
        ),
        Err(CompositionProbeError::Decode(error))
            if error.class() == DecodeErrorClass::MalformedFrame
    ));

    let compatibility = ConsumerCompatibilityRequest {
        profile: CONSUMER_PROFILE,
        artifact: profile::COMPOSITION_PROFILE,
        vocabulary_schemas: &[],
        required_features: &[],
        authoring_profile: None,
    };
    let entry = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &ProjectPublicType::Bool,
        selection: EntrySelection::Enumerate,
    };
    assert!(matches!(
        inspect_composition_entries_encoded(
            &[],
            DecodeLimits::hard(),
            ProjectLimits {
                artifact_bytes: 65_536,
                ..neutral_encoding::project::hard_project_limits()
            },
            limits(),
            &CompositionEntryProbeRequest {
                compatibility,
                entry,
                limits: ConsumerLimits::default(),
            },
            &cancel,
        ),
        Err(CompositionProbeError::Consumer(_))
    ));
}

#[test]
fn successor_probe_projects_a_complete_empty_project() {
    let cancel = CancellationToken::new();
    let project = empty_project();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    let summary = inspect_composition_encoded(
        &bytes,
        DecodeLimits::hard(),
        neutral_encoding::project::hard_project_limits(),
        limits(),
        None,
        &cancel,
    )
    .unwrap();
    assert_eq!(summary.modules, 1);
    assert_eq!(summary.declarations, 0);
    assert_eq!(summary.view.roots(), []);
    assert!(neutral_probe::composition::render_composition_summary_json(&summary, &cancel).is_ok());
}
