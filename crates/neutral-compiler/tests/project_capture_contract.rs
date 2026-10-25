// SPDX-License-Identifier: Apache-2.0

//! Public project-capture integration contract.

use neutral_compiler::{
    CapturedExpressionProject, CapturedExpressionProjectRequest, CapturedProjectRequest,
    CapturedProjectRequestBuilder, CapturedProjectSource, CapturedProjectVocabulary,
    CapturedSourceInput, CapturedVocabularyInput, ProjectCaptureControls,
    ProjectCaptureLimitValues, ProjectCaptureLimits, ProjectHostError, capture_expression_project,
};
use neutral_core::StructuralLimits;
use neutral_core::{CancellationToken, VocabularyContentDigest, profile::LanguageProfile};
use neutral_ir::{
    composition::profile::{VOCABULARY_COMPOSITION_FEATURE, VOCABULARY_SCHEMA_VERSION},
    expression::{
        profile::{ExpressionLimits, SemanticProfiles},
        templates::ExpansionLimits,
    },
};
use neutral_vocabulary::{
    VocabularyLimits, VocabularyLock,
    composition::{CompositionLimits, ENCODING_VERSION},
};

/// Supplies explicit current semantic policies alongside a host's current byte envelope.
fn capture_request(
    envelope: CapturedProjectRequest,
) -> Result<CapturedExpressionProject, neutral_compiler::ExpressionCaptureFailure> {
    capture_expression_project(CapturedExpressionProjectRequest::new(
        envelope,
        SemanticProfiles::try_frozen().unwrap(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(4096, 16).unwrap(),
        )),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
}

/// Host correlation and input ordering cannot change the complete captured input facts.
fn same_inputs(left: &CapturedExpressionProject, right: &CapturedExpressionProject) -> bool {
    left.sources() == right.sources()
        && left.vocabularies() == right.vocabularies()
        && left.semantic_profiles() == right.semantic_profiles()
}

/// One Editor-owned location mapped to logical captured facts before capture.
struct EditorSource<'a> {
    /// Editor-only tab or workspace location, deliberately not forwarded.
    location: &'a str,
    /// Inert logical source identity.
    source_id: &'a str,
    /// Exact logical module identity.
    module_id: &'a str,
    /// Exact already-acquired bytes.
    bytes: &'a [u8],
}

/// Returns complete explicit limits for the public request schema.
fn limits(source_units: u64) -> ProjectCaptureLimits {
    ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 1024,
        source_units,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 2,
        vocabulary_bytes_per_unit: 1024,
        total_vocabulary_bytes: 2048,
        imports_per_module: 8,
        import_edges: 16,
        scc_units: source_units,
        declarations: 64,
        diagnostics: 16,
        output_bytes: 16_384,
    })
}

/// Creates the shared builder used by every host probe.
fn builder(source_units: u64) -> CapturedProjectRequestBuilder {
    CapturedProjectRequestBuilder::new(
        LanguageProfile::V1_0,
        ProjectCaptureControls::new(limits(source_units), CancellationToken::new()),
    )
}

/// Constructs one exact data-only vocabulary input.
fn vocabulary(identity: &str) -> CapturedVocabularyInput {
    let bytes = format!(
        r#"{{
        "format": "neutral-vocabulary-bundle",
        "encoding_version": "{ENCODING_VERSION}",
        "schema_version": "{VOCABULARY_SCHEMA_VERSION}",
        "identity": "{identity}", "version": "1.0.0",
        "required_features": ["{VOCABULARY_COMPOSITION_FEATURE}"],
        "dependencies": [], "types": []
    }}"#
    )
    .into_bytes();
    let lock = VocabularyLock::new(
        identity,
        "1.0.0",
        ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&bytes),
        vec![VOCABULARY_COMPOSITION_FEATURE.to_owned()],
    )
    .expect("public test lock must be valid");
    CapturedVocabularyInput::new(bytes, lock)
}

/// Models Editor request construction without adding Editor state to core.
fn editor_request(
    sources: &[EditorSource<'_>],
) -> Result<CapturedExpressionProject, ProjectHostError> {
    let mut request = builder(u64::try_from(sources.len()).expect("test source count must fit"))
        .with_project_key("editor-session");
    for source in sources {
        assert_ne!(source.location, "");
        request.add_source(CapturedSourceInput::new(
            source.source_id,
            source.module_id,
            source.bytes.to_vec(),
        ))?;
    }
    Ok(capture_request(request.build()).expect("Editor request must capture"))
}

#[test]
fn editor_and_cli_style_hosts_construct_the_same_closed_request_meaning() {
    let sources = [EditorSource {
        location: "editor://workspace/tab-1",
        source_id: "source:shared-host",
        module_id: "host::shared",
        bytes: b"neu \"1.0\"\nmodule host::shared\n",
    }];
    let editor = editor_request(&sources).expect("Editor mapping must be coherent");

    let mut cli = builder(1).with_project_key("cli-command");
    cli.add_source(CapturedSourceInput::new(
        sources[0].source_id,
        sources[0].module_id,
        sources[0].bytes.to_vec(),
    ))
    .expect("CLI mapping must be coherent");
    let cli = capture_request(cli.build()).expect("CLI-style request must capture");

    assert!(same_inputs(&editor, &cli));
    assert_eq!(editor.resource_facts().source_units(), 1);
    assert_eq!(
        editor.resource_facts().total_source_bytes(),
        u64::try_from(sources[0].bytes.len()).expect("test length must fit")
    );
}

#[test]
fn host_builder_coalesces_equivalent_mappings_and_rejects_conflicts() {
    let mut request = builder(1);
    let source = CapturedSourceInput::new(
        "source:mapped",
        "host::mapped",
        b"neu \"1.0\"\nmodule host::mapped\n".to_vec(),
    );
    request
        .add_source(source.clone())
        .expect("first host mapping must be accepted");
    request
        .add_source(source)
        .expect("equivalent host mapping must coalesce");
    assert_eq!(
        capture_request(request.build())
            .expect("coalesced request must capture")
            .sources()
            .len(),
        1
    );

    let mut conflicting = builder(1);
    conflicting
        .add_source(CapturedSourceInput::new(
            "source:mapped",
            "host::mapped",
            b"neu \"1.0\"\nmodule host::mapped\n".to_vec(),
        ))
        .expect("first mapping must be accepted");
    let error = conflicting
        .add_source(CapturedSourceInput::new(
            "source:mapped",
            "host::different",
            b"neu \"1.0\"\nmodule host::different\n".to_vec(),
        ))
        .expect_err("conflicting mapping must fail before capture");
    assert_eq!(error.code(), "NEU-HOST-001");
}

#[test]
fn property_shuffled_source_and_lock_order_has_equivalent_capture_meaning() {
    let first = CapturedSourceInput::new(
        "source:zeta",
        "shuffle::zeta",
        b"neu \"1.0\"\nmodule shuffle::zeta\nuse ZebraDomain as zebra\n".to_vec(),
    );
    let second = CapturedSourceInput::new(
        "source:alpha",
        "shuffle::alpha",
        b"neu \"1.0\"\nmodule shuffle::alpha\nuse AlphaDomain as alpha\n".to_vec(),
    );
    let mut forward = builder(2);
    forward.add_source(first.clone()).expect("source must map");
    forward.add_source(second.clone()).expect("source must map");
    forward.add_vocabulary(vocabulary("ZebraDomain"));
    forward.add_vocabulary(vocabulary("AlphaDomain"));

    let mut reversed = builder(2);
    reversed.add_source(second).expect("source must map");
    reversed.add_source(first).expect("source must map");
    reversed.add_vocabulary(vocabulary("AlphaDomain"));
    reversed.add_vocabulary(vocabulary("ZebraDomain"));

    let forward = capture_request(forward.build()).expect("forward request must capture");
    let reversed = capture_request(reversed.build()).expect("reversed request must capture");
    assert!(same_inputs(&forward, &reversed));
    assert_eq!(
        forward
            .sources()
            .iter()
            .map(CapturedProjectSource::module_id)
            .collect::<Vec<_>>(),
        ["shuffle::alpha", "shuffle::zeta"]
    );
    assert_eq!(
        forward
            .vocabularies()
            .iter()
            .map(CapturedProjectVocabulary::identity)
            .collect::<Vec<_>>(),
        ["AlphaDomain", "ZebraDomain"]
    );
}

#[test]
fn captured_project_replays_without_inventing_a_closure_digest() {
    let sources = [EditorSource {
        location: "editor://replay/source",
        source_id: "source:replay",
        module_id: "capture::replay",
        bytes: b"neu \"1.0\"\nmodule capture::replay\n",
    }];
    let captured = editor_request(&sources).expect("initial capture must succeed");
    let replayed =
        capture_expression_project(captured.replay_request(CancellationToken::new()).unwrap())
            .expect("replay must succeed");
    assert!(same_inputs(&captured, &replayed));
}
