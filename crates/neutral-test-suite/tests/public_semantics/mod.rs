// SPDX-License-Identifier: Apache-2.0

//! Executable captured public-semantics fixtures and their reviewed oracles.

use crate::project_capture::{Table, parse_fixture, request_fixture, required_string};
use neutral_compiler::{
    ProjectDependencyKind, ProjectSemanticModel, ProjectVocabularyValidationError,
    analyze_project_semantics, build_module_graph, capture_project, validate_project_vocabularies,
};
use neutral_core::{CancellationToken, SemanticDigest, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    project_interface::{
        ProjectInterface, ProjectLocationValue, ProjectPublicEdge, ProjectPublicEdgeKind,
        ProjectPublicExport, ProjectPublicField, ProjectPublicSignature, ProjectPublicType,
        ProjectPublicVocabulary,
    },
};
use neutral_probe::summarize_project_interface;
use neutral_reader::{ProjectInterfaceError, ValidatedProjectInterface};
use neutral_vocabulary::VocabularyError;
use std::{sync::Arc, thread};

/// Exact crate-owned copies of the reviewed portable public-semantics fixtures.
const CASES: &[&str] = &[
    include_str!("fixtures/negative/visibility-leak.toml"),
    include_str!("fixtures/negative/private-ref-target.toml"),
    include_str!("fixtures/negative/type-incompatible.toml"),
    include_str!("fixtures/positive/reuse-provenance.toml"),
    include_str!("fixtures/negative/cross-scc-value-cycle.toml"),
];

/// Reviewed Stage 5 requests whose exact hashes are pinned by the manifest.
const STAGE5_CASES: &[(&str, &str)] = &[
    (
        "fixtures/positive/vocabulary-multiple-alias.toml",
        include_str!("fixtures/positive/vocabulary-multiple-alias.toml"),
    ),
    (
        "fixtures/positive/location-values.toml",
        include_str!("fixtures/positive/location-values.toml"),
    ),
    (
        "fixtures/negative/vocabulary-private-type.toml",
        include_str!("fixtures/negative/vocabulary-private-type.toml"),
    ),
    (
        "fixtures/negative/vocabulary-executable-payload.toml",
        include_str!("fixtures/negative/vocabulary-executable-payload.toml"),
    ),
];

#[test]
/// Executes every pinned Stage 5 request against its reviewed outcome.
fn conformance_stage5_vocabulary_and_location_cases_match_oracles() {
    let oracle = parse_fixture(include_str!("oracles/vocabulary-and-locations.toml"));
    let cases = oracle.arrays.get("case").expect("Stage 5 oracle cases");
    assert_eq!(cases.len(), STAGE5_CASES.len());
    for (path, text) in STAGE5_CASES {
        let fixture = parse_fixture(text);
        let id = required_string(&fixture.root, "case_id");
        let expected = cases
            .iter()
            .find(|case| required_string(case, "id") == id)
            .expect("registered Stage 5 oracle");
        assert_eq!(required_string(expected, "fixture"), *path, "{id}");
        let captured = capture_project(request_fixture(text)).expect("Stage 5 capture");
        if id == "V1-VOC-001-EXECUTABLE-PAYLOAD" {
            assert_eq!(
                required_string(expected, "expected_outcome"),
                "reject-vocabulary"
            );
            assert_eq!(
                required_string(expected, "error"),
                "ExecutableShapeForbidden"
            );
            assert_eq!(
                validate_project_vocabularies(&captured),
                Err(ProjectVocabularyValidationError::InvalidBundle(
                    VocabularyError::ExecutableShapeForbidden
                )),
                "{id}"
            );
            continue;
        }
        let graph =
            build_module_graph(&captured, &CancellationToken::new()).expect("Stage 5 graph");
        let model = analyze_project_semantics(&captured, &graph, &CancellationToken::new());
        if id == "V1-VOC-003-PRIVATE-TYPE" {
            assert_eq!(
                required_string(expected, "expected_outcome"),
                "reject-semantics"
            );
            assert_eq!(
                model.expect_err("private type must fail").diagnostics()[0].code(),
                required_string(expected, "diagnostic_code"),
                "{id}"
            );
            continue;
        }
        let model = model.expect("positive Stage 5 case");
        assert_eq!(
            required_string(expected, "expected_outcome"),
            "accept-resolution"
        );
        let view = ValidatedProjectInterface::from_interface(Arc::clone(model.public_interface()))
            .expect("canonical reader interface");
        match id.as_str() {
            "V1-VOC-001-MULTIPLE-ALIASES" => {
                let identities = view
                    .vocabularies()
                    .iter()
                    .map(ProjectPublicVocabulary::identity)
                    .collect::<Vec<_>>();
                assert_eq!(
                    format!("{identities:?}"),
                    expected["ordered_vocabulary_identities"]
                );
                let types = view
                    .vocabularies()
                    .iter()
                    .flat_map(|vocabulary| {
                        vocabulary.public_types().iter().map(move |name| {
                            format!("{}@{}::{name}", vocabulary.identity(), vocabulary.version())
                        })
                    })
                    .collect::<Vec<_>>();
                assert_eq!(format!("{types:?}"), expected["public_types"]);
            }
            "V1-LOC-001-DISTINCT-VALUES" => {
                assert_eq!(
                    model.locations()[0].1,
                    ProjectLocationValue::Url(required_string(expected, "url_text"))
                );
                assert_eq!(
                    model.locations()[1].1,
                    ProjectLocationValue::Path(required_string(expected, "path_text"))
                );
            }
            _ => panic!("unexpected Stage 5 case {id}"),
        }
    }
}

#[test]
/// Every registered public-semantics fixture matches its reviewed outcome.
fn conformance_semantic_cases_match_oracles() {
    let oracle = parse_fixture(include_str!("oracles/semantics.toml"));
    let cases = oracle
        .arrays
        .get("case")
        .expect("reviewed public-semantics cases");
    assert_eq!(cases.len(), CASES.len());
    for text in CASES {
        let fixture = parse_fixture(text);
        let id = required_string(&fixture.root, "case_id");
        let expected = cases
            .iter()
            .find(|case| required_string(case, "id") == id)
            .expect("fixture must have oracle");
        let captured =
            capture_project(request_fixture(text)).expect("public-semantics fixture must capture");
        let graph = build_module_graph(&captured, &CancellationToken::new())
            .expect("public-semantics graph must be valid");
        let result = analyze_project_semantics(&captured, &graph, &CancellationToken::new());
        match required_string(expected, "expected_outcome").as_str() {
            "reject-semantics" => {
                let code = result
                    .expect_err("negative fixture must fail")
                    .diagnostics()[0]
                    .code();
                assert_eq!(code, required_string(expected, "diagnostic_code"), "{id}");
            }
            "accept-resolution" => {
                let model = result.expect("positive fixture must pass");
                assert_positive_case(&model, expected, &id);
            }
            other => panic!("unknown reviewed semantic outcome: {other}"),
        }
    }
}

/// Checks one reviewed positive resolution through both semantic and reader views.
fn assert_positive_case(model: &ProjectSemanticModel, expected: &Table, id: &str) {
    let view = ValidatedProjectInterface::from_interface(Arc::clone(model.public_interface()))
        .expect("reviewed public interface must validate");
    let summary = summarize_project_interface(&view);
    assert_eq!(
        format!("{:?}", summary.exports()),
        expected["public_exports"],
        "{id}"
    );
    assert_eq!(
        summary.fingerprint(),
        required_string(expected, "public_signature_fingerprint"),
        "{id}"
    );
    let order = model
        .value_order()
        .iter()
        .map(|identity| {
            format!(
                "{}::{}",
                identity.module().module_name(),
                identity.declaration_name()
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        format!("{order:?}"),
        expected["dependency_first_values"],
        "{id}"
    );
    for (kind, declaration, key) in [
        (ProjectDependencyKind::Value, "copied", "value_edge"),
        (
            ProjectDependencyKind::Reference,
            "pointer",
            "reference_edge",
        ),
    ] {
        let edge = model
            .dependencies()
            .iter()
            .find(|edge| edge.kind() == kind && edge.from().declaration_name() == declaration)
            .expect("reviewed semantic edge");
        assert_eq!(
            format!(
                "{}::{} -> {}::{}",
                edge.from().module().module_name(),
                edge.from().declaration_name(),
                edge.to().module().module_name(),
                edge.to().declaration_name()
            ),
            required_string(expected, key),
            "{id}"
        );
    }
}

/// Resolves one portable public-semantics fixture into the reader view.
fn positive_view() -> ValidatedProjectInterface {
    let text = include_str!("fixtures/positive/reuse-provenance.toml");
    let captured = capture_project(request_fixture(text)).expect("positive fixture captures");
    let graph = build_module_graph(&captured, &CancellationToken::new()).expect("positive graph");
    let model = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("positive resolution");
    ValidatedProjectInterface::from_interface(Arc::clone(model.public_interface()))
        .expect("public interface validates independently")
}

#[test]
/// Reader-only probe enumerates public values and refs but no private source facts.
fn reader_and_probe_redact_private_implementation() {
    let view = positive_view();
    let summary = summarize_project_interface(&view);
    assert_eq!(
        summary.exports(),
        [
            "service::consumer::copied",
            "service::consumer::pointer",
            "service::source::answer"
        ]
    );
    assert_eq!(
        summary.cross_module_values(),
        ["service::consumer::copied -> service::source::answer"]
    );
    assert_eq!(
        summary.cross_module_references(),
        ["service::consumer::pointer -> service::source::answer"]
    );
    let visible = format!(
        "{:?}{:?}{:?}",
        summary.exports(),
        summary.cross_module_values(),
        summary.cross_module_references()
    );
    assert!(!visible.contains("seed"));
    assert!(!visible.contains("source:source"));
    assert!(!visible.contains("42"));
}

/// Builds a single-module public interface from an exact source body.
fn interface_for(body: &str) -> Arc<ProjectInterface> {
    let request = format!("neu \"1.0\"\nmodule api\n{body}");
    let source =
        neutral_compiler::CapturedSourceInput::new("source:api", "api", request.into_bytes());
    let limits = crate::project_capture::fixture_limits(&parse_fixture(CASES[3]));
    let captured = capture_project(neutral_compiler::CapturedProjectRequest::new(
        neutral_compiler::CAPTURE_REQUEST_VERSION,
        neutral_core::profile::LanguageProfile::V1_0,
        vec![source],
        Vec::new(),
        neutral_compiler::ProjectCaptureControls::new(limits, CancellationToken::new()),
    ))
    .expect("test project captures");
    let graph = build_module_graph(&captured, &CancellationToken::new()).expect("test graph");
    Arc::clone(
        analyze_project_semantics(&captured, &graph, &CancellationToken::new())
            .expect("test resolution")
            .public_interface(),
    )
}

#[test]
/// Private implementation changes do not perturb the public signature digest.
fn public_api_fingerprint_excludes_private_implementation() {
    let first = interface_for("num secret = 1\npublic num api = secret\n");
    let second = interface_for("num other = 2\npublic num api = other\n");
    let first = ValidatedProjectInterface::from_interface(first).expect("first public view");
    let second = ValidatedProjectInterface::from_interface(second).expect("second public view");
    assert_eq!(first.fingerprint(), second.fingerprint());
    assert!(first.public_edges().is_empty());
    assert!(second.public_edges().is_empty());
    assert_eq!(first.exports().len(), 1);
}

#[test]
/// Changing a public type changes the domain-separated interface fingerprint.
fn public_api_fingerprint_tracks_signature_changes() {
    let number = ValidatedProjectInterface::from_interface(interface_for("public num api = 1\n"))
        .expect("number view");
    let string =
        ValidatedProjectInterface::from_interface(interface_for("public string api = \"1\"\n"))
            .expect("string view");
    assert_ne!(number.fingerprint(), string.fingerprint());
}

#[test]
/// Public record fields retain canonical names and a reference-only type edge.
fn public_record_signature_and_reference_type_are_readable() {
    let view = ValidatedProjectInterface::from_interface(interface_for(
        "public record Node { Ref<Node> next, string label, }\n",
    ))
    .expect("public recursive reference is structurally valid");
    let ProjectPublicSignature::Record(fields) = view.exports()[0].signature() else {
        panic!("public Node must be a record");
    };
    assert_eq!(
        fields
            .iter()
            .map(ProjectPublicField::name)
            .collect::<Vec<_>>(),
        ["label", "next"]
    );
    assert_eq!(view.public_edges().len(), 1);
    assert_eq!(
        view.public_edges()[0].kind(),
        ProjectPublicEdgeKind::ReferenceType
    );
}

#[test]
/// Reader rejects stale fingerprints, dangling public edges, and private type leaks.
fn reader_security_rejects_forged_interfaces() {
    let valid = interface_for("public num api = 1\n");
    let forged_digest = ProjectInterface::from_parts(
        valid.exports().to_vec(),
        valid.edges().to_vec(),
        SemanticDigest::from_raw_bytes([0; 32]),
    );
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(forged_digest))
            .expect_err("stale digest"),
        ProjectInterfaceError::InvalidFingerprint
    );
    let api = valid.exports()[0].identity().clone();
    let private = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "api"),
        "secret",
    );
    let dangling = ProjectInterface::with_computed_fingerprint(
        valid.exports().to_vec(),
        vec![ProjectPublicEdge::new(
            api.clone(),
            private.clone(),
            ProjectPublicEdgeKind::Value,
        )],
    )
    .expect("framed interface");
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(dangling)).expect_err("private target"),
        ProjectInterfaceError::InvalidEdge
    );
    let record = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "api"),
        "Visible",
    );
    let leaked = ProjectInterface::with_computed_fingerprint(
        vec![ProjectPublicExport::new(
            record,
            ProjectPublicSignature::Record(vec![ProjectPublicField::new(
                "hidden",
                ProjectPublicType::Nominal(private),
            )]),
        )],
        Vec::new(),
    )
    .expect("framed leak");
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(leaked)).expect_err("private nominal"),
        ProjectInterfaceError::InvalidSignature
    );
}

#[test]
/// Parallel reader consumers produce identical public observations.
fn public_view_is_concurrently_deterministic() {
    let view = Arc::new(positive_view());
    let expected = summarize_project_interface(&view);
    let handles = (0..4)
        .map(|_| {
            let view = Arc::clone(&view);
            thread::spawn(move || summarize_project_interface(&view))
        })
        .collect::<Vec<_>>();
    for handle in handles {
        assert_eq!(handle.join().expect("reader thread"), expected);
    }
}

/// Builds a two-module interface with caller-selected alias and capture order.
fn reordered_interface(alias: &str, reverse: bool) -> ValidatedProjectInterface {
    let consumer = neutral_compiler::CapturedSourceInput::new(
        "source:consumer", "service::consumer",
        format!("neu \"1.0\"\nmodule service::consumer\nimport service::source as {alias}\npublic num copied = {alias}::answer\n").into_bytes(),
    );
    let source = neutral_compiler::CapturedSourceInput::new(
        "source:source",
        "service::source",
        b"neu \"1.0\"\nmodule service::source\npublic num answer = 42\n".to_vec(),
    );
    let inputs = if reverse {
        vec![source, consumer]
    } else {
        vec![consumer, source]
    };
    let limits = crate::project_capture::fixture_limits(&parse_fixture(CASES[3]));
    let captured = capture_project(neutral_compiler::CapturedProjectRequest::new(
        neutral_compiler::CAPTURE_REQUEST_VERSION,
        neutral_core::profile::LanguageProfile::V1_0,
        inputs,
        Vec::new(),
        neutral_compiler::ProjectCaptureControls::new(limits, CancellationToken::new()),
    ))
    .expect("reordered project captures");
    let graph = build_module_graph(&captured, &CancellationToken::new()).expect("reordered graph");
    let model = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("reordered resolution");
    ValidatedProjectInterface::from_interface(Arc::clone(model.public_interface()))
        .expect("reordered interface")
}

#[test]
/// Capture order and import-alias spelling cannot change public meaning.
fn public_interface_is_order_and_alias_invariant() {
    let first = reordered_interface("source", false);
    let second = reordered_interface("renamed", true);
    assert_eq!(first.fingerprint(), second.fingerprint());
    assert_eq!(
        summarize_project_interface(&first),
        summarize_project_interface(&second)
    );
}

#[test]
/// The reader rejects duplicate exports and unbounded type nesting before hashing.
fn reader_rejects_duplicate_exports_and_deep_types() {
    let valid = interface_for("public num api = 1\n");
    let export = valid.exports()[0].clone();
    let duplicate = ProjectInterface::with_computed_fingerprint(
        vec![export.clone(), export.clone()],
        Vec::new(),
    )
    .expect("duplicate transcript is representable");
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(duplicate))
            .expect_err("duplicate export"),
        ProjectInterfaceError::InvalidExport
    );
    let mut deep = ProjectPublicType::Num;
    for _ in 0..=neutral_ir::project_interface::MAX_PROJECT_INTERFACE_TYPE_DEPTH {
        deep = ProjectPublicType::List(Box::new(deep));
    }
    let deep_export = ProjectPublicExport::new(
        export.identity().clone(),
        ProjectPublicSignature::Binding(deep),
    );
    let untrusted =
        ProjectInterface::from_parts(vec![deep_export], Vec::new(), valid.fingerprint());
    assert_eq!(
        ValidatedProjectInterface::from_interface(Arc::new(untrusted)).expect_err("deep type"),
        ProjectInterfaceError::InvalidSignature
    );
}
