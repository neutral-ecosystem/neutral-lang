// SPDX-License-Identifier: Apache-2.0

//! Successor probe entrypoints reject incomplete inputs at their independent boundaries.

use neutral_core::{CancellationToken, StructuralLimits, allocation::Shared};
use neutral_encoding::{DecodeErrorClass, DecodeLimits, composition::encode_composition_project};
use neutral_ir::project_interface::ProjectPublicType;
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
    let composition = limits();
    let ir = neutral_test_support::empty_composition_ir(
        neutral_encoding::project::hard_project_limits(),
        composition.policy(),
    );
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

/// The actual executable recognizes composition magic and renders the independently decoded summary.
#[test]
fn system_probe_routes_composition_artifacts() {
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
    let directory = neutral_test_support::TestDirectory::new().unwrap();
    let path = directory.path().join("composition.nir");
    std::fs::write(&path, bytes).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_neutral-probe"))
        .arg("--json")
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, []);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        neutral_probe::composition::render_composition_summary_json(&summary, &cancel).unwrap()
    );
}

/// Composition routing must reject corrupt bytes without publishing a partial summary.
#[test]
fn system_probe_rejects_corrupt_composition_artifacts() {
    let bytes = encode_composition_project(&empty_project(), &CancellationToken::new()).unwrap();
    let directory = neutral_test_support::TestDirectory::new().unwrap();
    let path = directory.path().join("corrupt.nir");
    std::fs::write(&path, &bytes[..bytes.len() - 1]).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_neutral-probe"))
        .arg("--json")
        .arg(path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.stdout, []);
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains(DecodeErrorClass::MalformedCbor.code())
    );
}
