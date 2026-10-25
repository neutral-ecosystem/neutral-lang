// SPDX-License-Identifier: Apache-2.0

//! Compiler capture -> independent catalogue reader integration; not source/codec activation.

use super::{REQUESTS, bytes, limits_for, locks};
use neutral_compiler::{
    CapturedExpressionProjectRequest, CapturedProjectRequest, CapturedProjectVocabulary,
    CapturedSourceInput, CapturedVocabularyInput, ProjectCaptureControls, ProjectCaptureError,
    ProjectCaptureLimitValues, ProjectCaptureLimits, capture_expression_project,
};
use neutral_core::{CancellationToken, SourceContentDigest, profile::LanguageProfile};
use neutral_ir::{
    composition::{ClosedValue as V, CompositionBody, FieldPresence, ValueOriginKind},
    expression::{
        profile::{self, ExpressionLimits, SemanticProfiles},
        templates::ExpansionLimits,
    },
    project_interface::ProjectPublicType,
};
use neutral_reader::composition::{
    CompositionCatalogue, CompositionInspectionLimits, CompositionLookupError,
    ReferenceTypeSegment as P,
};
use serde_json::Value;
use std::fmt::Write;

/// Locates reviewed runtime inputs by literal identifier, without a portable dependency.
fn case(family: usize, name: &str) -> Value {
    let family: Value = serde_json::from_str(REQUESTS[family]).unwrap();
    family["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == name)
        .unwrap()
        .clone()
}

/// Adapts a fixture envelope to the explicit public API, retaining original source and bundle pins.
///
/// Catalogue-only fixtures have no source unit. For those tests only, an explicit
/// synthetic source declares their reviewed roots; this is not a literal source
/// compilation expectation or permission to infer roots in production.
pub(super) fn request(case: &Value) -> CapturedExpressionProjectRequest {
    let inputs = case["capture"]["vocabularies"].as_array().unwrap();
    let vocabularies = inputs
        .iter()
        .zip(locks(case))
        .map(|(input, lock)| {
            CapturedVocabularyInput::new(
                input["bundle_utf8"].as_str().unwrap().as_bytes().to_vec(),
                lock,
            )
        })
        .collect();
    let mut sources = case["capture"]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|input| {
            CapturedSourceInput::new(
                input["source_id"].as_str().unwrap(),
                input["module"].as_str().unwrap(),
                input["source_utf8"].as_str().unwrap().as_bytes().to_vec(),
            )
            .requiring_digest(SourceContentDigest::from_raw_bytes(
                bytes(&input["digest"]).try_into().unwrap(),
            ))
        })
        .collect::<Vec<_>>();
    if sources.is_empty() {
        let mut text = format!(
            "neu \"{}\"\nmodule catalogue_test\n",
            LanguageProfile::V1_0.source_version()
        );
        for (index, root) in case["roots"].as_array().unwrap().iter().enumerate() {
            writeln!(text, "use {} as root{index}", root[0].as_str().unwrap()).unwrap();
        }
        sources.push(CapturedSourceInput::new(
            "source:catalogue_test",
            "catalogue_test",
            text.into_bytes(),
        ));
    }
    CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            sources,
            vocabularies,
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(ProjectCaptureLimitValues {
                    total_source_bytes: 65_536,
                    source_bytes_per_unit: 16_384,
                    source_units: 64,
                    source_id_bytes: 256,
                    module_id_bytes: 256,
                    vocabulary_units: 64,
                    vocabulary_bytes_per_unit: 65_536,
                    total_vocabulary_bytes: 1_048_576,
                    imports_per_module: 64,
                    import_edges: 4096,
                    scc_units: 64,
                    declarations: 4096,
                    diagnostics: 128,
                    output_bytes: 1_048_576,
                }),
                CancellationToken::new(),
            ),
        ),
        SemanticProfiles::try_frozen().unwrap(),
        limits_for(case),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    )
}

/// Every registered catalogue family executes through capture; source semantics remain separate.
#[test]
fn conformance_composition_capture_registered_catalogue_cases() {
    for raw in REQUESTS {
        let data: Value = serde_json::from_str(raw).unwrap();
        for case in data["cases"].as_array().unwrap() {
            let actual = capture_expression_project(request(case));
            let expected = case["catalogue_outcome"].as_str().unwrap();
            if expected == "accepted" {
                assert!(actual.is_ok(), "{}", case["id"]);
            } else {
                // Capture rejects duplicate/conflicting locks at integrity,
                // before the standalone catalogue's duplicate-owner check.
                let expected = if case["id"] == "conflicting-revision" {
                    ProjectCaptureError::DuplicateVocabulary.code()
                } else {
                    expected
                };
                assert_eq!(actual.err().unwrap().code(), expected, "{}", case["id"]);
            }
        }
    }
}

/// Shared diamond leaves are retained once and source aliases do not grant implicit dependency access.
#[test]
fn integration_composition_capture_diamond_and_independent_public_reader() {
    let case = case(0, "exact-diamond-closure");
    let captured = capture_expression_project(request(&case)).unwrap();
    let identities = captured
        .vocabularies()
        .iter()
        .map(CapturedProjectVocabulary::identity)
        .collect::<Vec<_>>();
    assert_eq!(identities, ["Leaf", "Left", "Right", "Root"]);
    assert_eq!(captured.resource_facts().vocabulary_units(), 4);
    let reader = CompositionCatalogue::from_shared(captured.catalogue());
    assert_eq!(
        reader
            .vocabularies()
            .map(neutral_ir::VocabularyIdentity::identity)
            .collect::<Vec<_>>(),
        identities
    );
    assert_eq!(
        reader
            .dependencies("Root", "1.0.0")
            .unwrap()
            .iter()
            .map(|d| d.identity.as_str())
            .collect::<Vec<_>>(),
        ["Left", "Right"]
    );
    let materialized = reader
        .materialize(
            ("Root", "1.0.0", "Item"),
            &V::Record(vec![]),
            limits_for(&case),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(
        materialized.value(),
        &V::Record(vec![("left".into(), None), ("right".into(), None)])
    );
    assert_eq!(
        materialized
            .origins()
            .iter()
            .map(|o| o.kind)
            .collect::<Vec<_>>(),
        [
            ValueOriginKind::Supplied,
            ValueOriginKind::OmittedOptional,
            ValueOriginKind::OmittedOptional
        ]
    );
}

/// Lists, nullable wrappers and unselected variant alternatives stay visible without private contracts.
#[test]
fn integration_composition_capture_reader_reference_paths_and_privacy() {
    let case = case(0, "list-nullable-reference-unselected-alternative");
    let captured = capture_expression_project(request(&case)).unwrap();
    let reader = CompositionCatalogue::from_shared(captured.catalogue());
    let policy = CompositionInspectionLimits {
        visits: 1000,
        references: 10,
        depth: 10,
    };
    let refs = reader
        .reference_types(
            ("Fixture", "1.0.0", "Request"),
            policy,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].path, [P::Field("direct")]);
    assert_eq!(
        refs[1].path,
        [P::Field("others"), P::ListElement, P::NullableInner]
    );
    let refs = reader
        .reference_types(
            ("Fixture", "1.0.0", "Outcome"),
            policy,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].path, [P::Alternative("target")]);
    for name in ["Private", "Missing"] {
        assert_eq!(
            reader.public_type("Fixture", "1.0.0", name),
            Err(CompositionLookupError::UnavailableType)
        );
    }
    let supplied = V::Variant {
        tag: "text".into(),
        payload: Box::new(V::String("complete".into())),
    };
    assert_eq!(
        reader
            .materialize(
                ("Fixture", "1.0.0", "Outcome"),
                &supplied,
                limits_for(&case),
                &CancellationToken::new()
            )
            .unwrap()
            .value(),
        &supplied
    );
}

/// Independent readers expose required/optional/defaulted contracts, finite choices and exact bounds.
#[test]
fn integration_composition_capture_reader_preserves_complete_restrictions() {
    let case = case(
        0,
        "both-origins-heterogeneous-list-default-absence-reference",
    );
    let captured = capture_expression_project(request(&case)).unwrap();
    let reader = CompositionCatalogue::from_shared(captured.catalogue());
    let CompositionBody::Record(fields) = &reader
        .public_type("ExampleDomain", "1.0.0", "Request")
        .unwrap()
        .body
    else {
        panic!("expected record");
    };
    let attempts = fields.iter().find(|f| f.name == "attempts").unwrap();
    assert_eq!(attempts.presence, FieldPresence::Defaulted);
    assert_eq!(attempts.restrictions.choices.as_ref().unwrap().len(), 2);
    assert_eq!(
        attempts.default,
        attempts.restrictions.maximum.clone().map(V::Number)
    );
    let label = fields.iter().find(|f| f.name == "label").unwrap();
    assert_eq!(label.presence, FieldPresence::Optional);
    assert_eq!(
        label.ty,
        ProjectPublicType::Nullable(Box::new(ProjectPublicType::String))
    );
    assert_eq!(label.restrictions.max_length, Some(32));
    let outcomes = fields.iter().find(|f| f.name == "outcomes").unwrap();
    assert_eq!(outcomes.presence, FieldPresence::Defaulted);
    assert_eq!(outcomes.restrictions.max_length, Some(16));
    assert!(matches!(&outcomes.default, Some(V::List(items)) if items.len() == 1));
}

/// Exact captured order and replay do not alter accepted contracts or fresh module graph facts.
#[test]
fn property_composition_capture_permutation_replay_and_concurrent_requests() {
    let mut case = case(0, "exact-diamond-closure");
    let first = capture_expression_project(request(&case)).unwrap();
    case["capture"]["vocabularies"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let reversed = capture_expression_project(request(&case)).unwrap();
    assert_eq!(first.vocabularies(), reversed.vocabularies());
    assert_eq!(first.catalogue(), reversed.catalogue());
    let replay =
        capture_expression_project(first.replay_request(CancellationToken::new()).unwrap())
            .unwrap();
    assert_eq!(first.sources(), replay.sources());
    assert_eq!(first.vocabularies(), replay.vocabularies());
    assert_eq!(first.catalogue(), replay.catalogue());
    std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| scope.spawn(|| capture_expression_project(request(&case)).unwrap()))
            .collect::<Vec<_>>();
        for handle in handles {
            assert_eq!(handle.join().unwrap().catalogue(), first.catalogue());
        }
    });
}

/// Explicit old-schema leaves retain old required-field meaning, never acquire new defaults.
#[test]
fn compatibility_composition_capture_legacy_leaf_adapter_is_explicit() {
    let case = case(3, "legacy-leaf-explicit-adapter");
    let captured = capture_expression_project(request(&case)).unwrap();
    let reader = CompositionCatalogue::from_shared(captured.catalogue());
    let CompositionBody::Record(fields) =
        &reader.public_type("Fixture", "1.0.0", "Item").unwrap().body
    else {
        panic!("expected record");
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].presence, FieldPresence::Required);
    assert_eq!(fields[0].default, None);
    assert_eq!(reader.dependencies("Fixture", "1.0.0").unwrap(), []);
}
