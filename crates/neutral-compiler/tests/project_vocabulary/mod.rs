// SPDX-License-Identifier: Apache-2.0

//! Exact project vocabulary set tests.

use super::*;
use crate::{
    CAPTURE_REQUEST_VERSION, CapturedProjectRequest, CapturedSourceInput, CapturedVocabularyInput,
    ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    analyze_project_semantics, build_module_graph, capture_project,
};
use neutral_core::{CancellationToken, VocabularyContentDigest, profile::LanguageProfile};
use neutral_ir::project_interface::{
    ProjectLocationValue, ProjectPublicSignature, ProjectPublicType,
};
use neutral_vocabulary::{
    PROJECT_VOCABULARY_ENCODING_VERSION, PROJECT_VOCABULARY_SCHEMA_VERSION, VocabularyLock,
};

/// Builds a closed v1 bundle and its exact lock.
fn vocabulary(identity: &str, types: &str) -> CapturedVocabularyInput {
    let bytes = format!(
        "{{\"format\":\"neutral-vocabulary-bundle\",\"encoding_version\":\"{PROJECT_VOCABULARY_ENCODING_VERSION}\",\"schema_version\":\"{PROJECT_VOCABULARY_SCHEMA_VERSION}\",\"identity\":\"{identity}\",\"version\":\"1.0.0\",\"required_features\":[],\"types\":[{types}]}}"
    ).into_bytes();
    let lock = VocabularyLock::new(
        identity,
        "1.0.0",
        PROJECT_VOCABULARY_ENCODING_VERSION,
        PROJECT_VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&bytes),
        Vec::new(),
    )
    .expect("valid lock");
    CapturedVocabularyInput::new(bytes, lock)
}

/// Captures one complete source and bundle set without host lookup.
fn captured(source: &str, vocabularies: Vec<CapturedVocabularyInput>) -> CapturedProject {
    let values = ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 4096,
        source_units: 1,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 4,
        vocabulary_bytes_per_unit: 4096,
        total_vocabulary_bytes: 16_384,
        imports_per_module: 4,
        import_edges: 4,
        scc_units: 1,
        declarations: 16,
        diagnostics: 8,
        output_bytes: 16_384,
    };
    capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![CapturedSourceInput::new(
            "source:test",
            "api",
            source.as_bytes().to_vec(),
        )],
        vocabularies,
        ProjectCaptureControls::new(ProjectCaptureLimits::new(values), CancellationToken::new()),
    ))
    .expect("exact project capture")
}

#[test]
/// Resolves multiple aliases without admitting them to canonical identities.
fn multiple_aliases_keep_canonical_vocabulary_identities() {
    let first = vocabulary(
        "Alpha",
        "{\"name\":\"Public\",\"public\":true,\"fields\":[]}",
    );
    let second = vocabulary(
        "Beta",
        "{\"name\":\"Public\",\"public\":true,\"fields\":[]}",
    );
    let captured = captured(
        "neu \"1.0\"\nmodule api\nuse Beta as second\nuse Alpha as first\n",
        vec![second, first],
    );
    let set = validate_project_vocabularies(&captured).expect("exact bundle set");
    assert_eq!(
        set.vocabularies()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["Alpha", "Beta"]
    );
    assert_eq!(
        set.resolve("api", "first").expect("alias").identity(),
        "Alpha"
    );
    assert_eq!(
        set.resolve("api", "second").expect("alias").identity(),
        "Beta"
    );
    assert!(set.resolve("api", "unknown").is_none());
}

#[test]
/// Rejects public fields that disclose a private nominal type.
fn rejects_private_type_in_public_vocabulary() {
    let types = "{\"name\":\"Hidden\",\"public\":false,\"fields\":[]},{\"name\":\"Public\",\"public\":true,\"fields\":[{\"name\":\"secret\",\"type\":\"Hidden\"}]}";
    let captured = captured(
        "neu \"1.0\"\nmodule api\nuse Alpha as alpha\n",
        vec![vocabulary("Alpha", types)],
    );
    assert_eq!(
        validate_project_vocabularies(&captured),
        Err(ProjectVocabularyValidationError::InvalidBundle(
            VocabularyError::PrivateTypeExposed
        )),
    );
}

#[test]
/// Rejects executable payload members before publishing a contract.
fn rejects_executable_project_bundle_member() {
    let types = "{\"name\":\"Public\",\"public\":true,\"fields\":[],\"script\":\"run\"}";
    let captured = captured(
        "neu \"1.0\"\nmodule api\nuse Alpha as alpha\n",
        vec![vocabulary("Alpha", types)],
    );
    assert_eq!(
        validate_project_vocabularies(&captured),
        Err(ProjectVocabularyValidationError::InvalidBundle(
            VocabularyError::ExecutableShapeForbidden
        )),
    );
}

#[test]
/// Authoring metadata cannot be smuggled into a semantic bundle.
fn rejects_inline_authoring_metadata() {
    let types = "{\"name\":\"Public\",\"public\":true,\"fields\":[],\"metadata\":{}}";
    let captured = captured(
        "neu \"1.0\"\nmodule api\nuse Alpha as alpha\n",
        vec![vocabulary("Alpha", types)],
    );
    assert_eq!(
        validate_project_vocabularies(&captured),
        Err(ProjectVocabularyValidationError::InvalidBundle(
            VocabularyError::UnknownMember
        )),
    );
}

#[test]
/// Public source types retain canonical lock identity, not local alias spelling.
fn public_vocabulary_type_uses_canonical_identity() {
    let types = "{\"name\":\"Hidden\",\"public\":false,\"fields\":[]},{\"name\":\"Visible\",\"public\":true,\"fields\":[]}";
    let captured = captured(
        "neu \"1.0\"\nmodule api\nuse Alpha as local\npublic local::Visible item = {}\n",
        vec![vocabulary("Alpha", types)],
    );
    let graph = build_module_graph(&captured, &CancellationToken::new()).expect("valid graph");
    let model = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("public vocabulary type resolves");
    let exports = model.public_interface().exports();
    let vocabularies = model.public_interface().vocabularies();
    assert_eq!(vocabularies.len(), 1);
    assert_eq!(vocabularies[0].identity(), "Alpha");
    assert_eq!(vocabularies[0].version(), "1.0.0");
    assert_eq!(vocabularies[0].public_types(), ["Visible"]);
    assert_eq!(exports.len(), 1);
    assert_eq!(
        exports[0].signature(),
        &ProjectPublicSignature::Binding(ProjectPublicType::VocabularyNominal {
            identity: "Alpha".to_owned(),
            version: "1.0.0".to_owned(),
            name: "Visible".to_owned(),
        }),
    );
}

#[test]
/// Private vocabulary types cannot appear even in private source declarations.
fn source_cannot_name_private_vocabulary_type() {
    let types = "{\"name\":\"Hidden\",\"public\":false,\"fields\":[]}";
    let captured = captured(
        "neu \"1.0\"\nmodule api\nuse Alpha as local\nlocal::Hidden item = {}\n",
        vec![vocabulary("Alpha", types)],
    );
    let graph = build_module_graph(&captured, &CancellationToken::new()).expect("valid graph");
    let error = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect_err("private type cannot surface");
    assert_eq!(
        error.diagnostics()[0].code(),
        crate::project_semantics_diagnostics::PRIVATE_VOCABULARY_TYPE
    );
}

#[test]
/// URL and path retain exact decoded text in distinct IR variants.
fn locations_are_inert_distinct_values() {
    let captured = captured(
        "neu \"1.0\"\nmodule api\npublic url endpoint = \"HTTPS://Example.invalid/A%2fb\"\npublic path file = \"../A/../b\"\n",
        Vec::new(),
    );
    let graph = build_module_graph(&captured, &CancellationToken::new()).expect("valid graph");
    let model = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("inert values resolve");
    assert_eq!(model.locations().len(), 2);
    assert_eq!(
        model.locations()[0].1,
        ProjectLocationValue::Url("HTTPS://Example.invalid/A%2fb".to_owned())
    );
    assert_eq!(
        model.locations()[1].1,
        ProjectLocationValue::Path("../A/../b".to_owned())
    );
    let exports = model.public_interface().exports();
    assert!(
        exports
            .iter()
            .any(|entry| entry.signature()
                == &ProjectPublicSignature::Binding(ProjectPublicType::Url))
    );
    assert!(exports.iter().any(
        |entry| entry.signature() == &ProjectPublicSignature::Binding(ProjectPublicType::Path)
    ));
    assert_ne!(
        ProjectLocationValue::Url("same".to_owned()),
        ProjectLocationValue::Path("same".to_owned()),
    );
}

#[test]
/// Malformed host locations remain exact data and require no host resource.
fn security_locations_do_not_require_url_or_path_resolution() {
    let captured = captured(
        "neu \"1.0\"\nmodule api\npublic url endpoint = \"not a URL %zz\"\npublic path file = \"/neutral/absent/../opaque\"\n",
        Vec::new(),
    );
    let graph = build_module_graph(&captured, &CancellationToken::new()).expect("valid graph");
    let model = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("locations are inert scalars");
    assert_eq!(
        model.locations()[0].1,
        ProjectLocationValue::Url("not a URL %zz".to_owned())
    );
    assert_eq!(
        model.locations()[1].1,
        ProjectLocationValue::Path("/neutral/absent/../opaque".to_owned())
    );
}

#[test]
/// Alias spelling cannot perturb the complete canonical public interface.
fn alias_renaming_preserves_public_fingerprint() {
    let types = "{\"name\":\"Visible\",\"public\":true,\"fields\":[]}";
    let resolve = |alias: &str| {
        let body = format!(
            "neu \"1.0\"\nmodule api\nuse Alpha as {alias}\npublic {alias}::Visible item = {{}}\n"
        );
        let captured = captured(&body, vec![vocabulary("Alpha", types)]);
        let graph = build_module_graph(&captured, &CancellationToken::new()).expect("valid graph");
        analyze_project_semantics(&captured, &graph, &CancellationToken::new())
            .expect("valid semantic model")
            .public_interface()
            .clone()
    };
    assert_eq!(resolve("first"), resolve("second"));
}

#[test]
/// Comment text cannot invent an extra required vocabulary identity.
fn commented_requirement_does_not_change_canonical_set() {
    let captured = captured(
        "neu \"1.0\"\nmodule api\n/*\nuse Decoy as decoy\n*/\nuse Alpha as alpha\n",
        vec![vocabulary("Alpha", "")],
    );
    let set = validate_project_vocabularies(&captured).expect("comments are nonsemantic");
    assert_eq!(set.vocabularies().len(), 1);
    assert!(set.resolve("api", "decoy").is_none());
}

#[test]
/// A location declaration must use a decoded quoted literal, not a number.
fn rejects_non_text_location_initializer() {
    let captured = captured("neu \"1.0\"\nmodule api\nurl endpoint = 42\n", Vec::new());
    let graph = build_module_graph(&captured, &CancellationToken::new()).expect("valid graph");
    let error = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect_err("location initializer must be text");
    assert_eq!(
        error.diagnostics()[0].code(),
        crate::project_semantics_diagnostics::INVALID_SOURCE
    );
}

#[test]
/// An embedded nominal cycle is not permitted in closed project data.
fn rejects_embedded_vocabulary_recursion() {
    let types =
        "{\"name\":\"Loop\",\"public\":true,\"fields\":[{\"name\":\"next\",\"type\":\"Loop\"}]}";
    let captured = captured(
        "neu \"1.0\"\nmodule api\nuse Alpha as alpha\n",
        vec![vocabulary("Alpha", types)],
    );
    assert_eq!(
        validate_project_vocabularies(&captured),
        Err(ProjectVocabularyValidationError::InvalidBundle(
            VocabularyError::InvalidTypeRecursion
        )),
    );
}
