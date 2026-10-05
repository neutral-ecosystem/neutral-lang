// SPDX-License-Identifier: Apache-2.0

//! Real source programs through compilation, wire artifacts, readers, and executable inspection.

use neutral_compiler::{
    CAPTURE_REQUEST_VERSION, CapturedProject, CapturedProjectRequest, CapturedSourceInput,
    CapturedVocabularyInput, ProjectCacheLimits, ProjectCaptureControls, ProjectCompilationCache,
    ProjectCompileFailure, capture_project, compile_project, module_graph_diagnostics as graph,
    project_lowering_diagnostics as lowering, project_semantics_diagnostics as semantics,
};
use neutral_core::{CancellationToken, VocabularyContentDigest, profile::LanguageProfile};
use neutral_encoding::{
    DecodeLimits,
    project::{decode_project, encode_project},
};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    project::{PROJECT_RESULT_SCHEMA, ProjectIr, ProjectValue},
};
use neutral_probe::project::{inspect_project_encoded, render_project_summary_json};
use neutral_reader::{
    IdentityLimits, MAX_TRANSCRIPT_BYTES, MAX_TRANSCRIPT_NODES, ValidatedProject,
};
use neutral_vocabulary::{
    PROJECT_VOCABULARY_ENCODING_VERSION, PROJECT_VOCABULARY_SCHEMA_VERSION, VocabularyLock,
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

mod system;

/// Registers exact file bytes with an explicit module rather than rewriting a header.
macro_rules! source {
    ($kind:literal, $name:literal, $module:literal) => {
        (
            $module,
            include_str!(concat!("fixtures/", $kind, "/", $name, ".neu")),
        )
    };
}

/// One accepted program with independently specified declaration/value expectations.
struct Case {
    /// Stable regression name, unrelated to a development stage.
    name: &'static str,
    /// Complete supplied module closure.
    units: &'static [(&'static str, &'static str)],
    /// Explicit vocabulary bundle requirement.
    vocabulary: bool,
    /// Complete declaration count, including private and disconnected content.
    declarations: usize,
}

/// File-backed accepted corpus; every member is exercised by APIs and the executable.
const CASES: &[Case] = &[
    Case {
        name: "scalars",
        units: &[source!("positive", "scalars", "api")],
        vocabulary: false,
        declarations: 5,
    },
    Case {
        name: "unicode",
        units: &[source!("positive", "unicode", "api")],
        vocabulary: false,
        declarations: 1,
    },
    Case {
        name: "nullable-lists",
        units: &[source!("positive", "nullable-lists", "api")],
        vocabulary: false,
        declarations: 4,
    },
    Case {
        name: "records",
        units: &[source!("positive", "records", "api")],
        vocabulary: false,
        declarations: 3,
    },
    Case {
        name: "records-no-trailing-comma",
        units: &[source!("positive", "records-no-trailing-comma", "api")],
        vocabulary: false,
        declarations: 3,
    },
    Case {
        name: "single-line-fields",
        units: &[source!("positive", "single-line-fields", "api")],
        vocabulary: false,
        declarations: 3,
    },
    Case {
        name: "single-line-fields-comma",
        units: &[source!("positive", "single-line-fields-comma", "api")],
        vocabulary: false,
        declarations: 3,
    },
    Case {
        name: "forward-reuse",
        units: &[source!("positive", "forward-reuse", "api")],
        vocabulary: false,
        declarations: 3,
    },
    Case {
        name: "reference-cycle",
        units: &[source!("positive", "reference-cycle", "api")],
        vocabulary: false,
        declarations: 3,
    },
    Case {
        name: "locations",
        units: &[source!("positive", "locations", "api")],
        vocabulary: false,
        declarations: 2,
    },
    Case {
        name: "vocabulary",
        units: &[source!("positive", "vocabulary", "api")],
        vocabulary: true,
        declarations: 3,
    },
    Case {
        name: "imports",
        units: &[
            source!("positive", "consumer", "app::consumer"),
            source!("positive", "orphan", "app::orphan"),
            source!("positive", "shared", "app::shared"),
        ],
        vocabulary: false,
        declarations: 8,
    },
    Case {
        name: "import-cycle",
        units: &[
            source!("positive", "cycle-left", "cycle::left"),
            source!("positive", "cycle-right", "cycle::right"),
        ],
        vocabulary: false,
        declarations: 2,
    },
];

/// Returns the closed vocabulary schema with exact lock bytes and a public typed field.
fn vocabulary(identity: &str) -> CapturedVocabularyInput {
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "format": "neutral-vocabulary-bundle",
        "encoding_version": PROJECT_VOCABULARY_ENCODING_VERSION,
        "schema_version": PROJECT_VOCABULARY_SCHEMA_VERSION,
        "identity": identity, "version": "1.0.0", "required_features": [],
        "types": [{ "name": "Visible", "public": true,
                    "fields": [{"name": "label", "type": "string"}]}],
    }))
    .unwrap();
    let lock = VocabularyLock::new(
        identity,
        "1.0.0",
        PROJECT_VOCABULARY_ENCODING_VERSION,
        PROJECT_VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&bytes),
        Vec::new(),
    )
    .unwrap();
    CapturedVocabularyInput::new(bytes, lock)
}

/// Captures exact source files with stable logical source IDs and shared explicit test bounds.
fn capture(units: &[(&str, &str)], with_vocabulary: bool) -> CapturedProject {
    let limits = crate::project_capture::fixture_limits(&crate::project_capture::parse_fixture(
        include_str!("../project_ir/complete.toml"),
    ));
    capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        units
            .iter()
            .map(|(module, text)| {
                CapturedSourceInput::new(
                    format!("source:{module}"),
                    *module,
                    text.as_bytes().to_vec(),
                )
            })
            .collect(),
        if with_vocabulary {
            vec![vocabulary("Alpha"), vocabulary("Beta")]
        } else {
            Vec::new()
        },
        ProjectCaptureControls::new(limits, CancellationToken::new()),
    ))
    .expect("exact file closure must capture")
}

/// Makes expected exact numbers independently of project lowering.
fn number(value: &str) -> ProjectValue {
    ProjectValue::Number(ExactNumber::from_source(value, 128, 128).unwrap())
}

/// Creates an independently expected reference identity, not a compiler-derived target.
fn symbol(module: &str, name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(LanguageProfile::V1_0.source_version(), module),
        name,
    )
}

/// Returns literal expected materialized values for each accepted program.
fn expected_values(name: &str) -> Vec<(&str, &str, ProjectValue)> {
    match name {
        "single-line-fields" | "single-line-fields-comma" => expected_single_line_fields(),
        "scalars" => vec![
            ("api", "fraction", number("-125")),
            ("api", "exponent", number("42")),
            ("api", "enabled", ProjectValue::Bool(true)),
            ("api", "disabled", ProjectValue::Bool(false)),
            ("api", "label", ProjectValue::String("neutral".to_owned())),
        ],
        "unicode" => vec![(
            "api",
            "label",
            ProjectValue::String("café\n世界\t\"quoted\"".to_owned()),
        )],
        "nullable-lists" => vec![
            ("api", "missing", ProjectValue::Null),
            ("api", "present", ProjectValue::String("yes".to_owned())),
            (
                "api",
                "matrix",
                ProjectValue::List(vec![
                    ProjectValue::List(vec![number("1"), number("2")]),
                    ProjectValue::List(Vec::new()),
                    ProjectValue::List(vec![number("3")]),
                ]),
            ),
            ("api", "empty", ProjectValue::List(Vec::new())),
        ],
        "records" | "records-no-trailing-comma" => vec![(
            "api",
            "service",
            ProjectValue::Record(vec![
                (
                    "child".to_owned(),
                    ProjectValue::Record(vec![("count".to_owned(), number("42"))]),
                ),
                ("note".to_owned(), ProjectValue::Null),
                (
                    "ports".to_owned(),
                    ProjectValue::List(vec![number("80"), number("443")]),
                ),
            ]),
        )],
        "forward-reuse" => vec![("api", "final_value", number("42"))],
        "reference-cycle" => vec![
            ("api", "first", expected_node("second")),
            ("api", "second", expected_node("first")),
        ],
        "locations" => vec![
            (
                "api",
                "endpoint",
                ProjectValue::Url("HTTPS://Example.invalid/A%2fb".to_owned()),
            ),
            ("api", "file", ProjectValue::Path("../A/../b".to_owned())),
        ],
        "vocabulary" => [
            ("original", "typed"),
            ("copied", "typed"),
            ("other", "other"),
        ]
        .into_iter()
        .map(|(binding, label)| {
            (
                "api",
                binding,
                ProjectValue::Record(vec![(
                    "label".to_owned(),
                    ProjectValue::String(label.to_owned()),
                )]),
            )
        })
        .collect(),
        "imports" => vec![
            ("app::shared", "api", expected_service()),
            ("app::shared", "answer", number("42")),
            ("app::consumer", "copied", expected_service()),
            ("app::consumer", "answer", number("42")),
            (
                "app::consumer",
                "pointer",
                ProjectValue::Reference(symbol("app::shared", "api")),
            ),
        ],
        "import-cycle" => vec![
            ("cycle::left", "seed", number("42")),
            ("cycle::right", "answer", number("42")),
        ],
        _ => panic!("unregistered value oracle: {name}"),
    }
}

/// Both defaults materialize independently; overriding one field retains the other default.
fn expected_single_line_fields() -> Vec<(&'static str, &'static str, ProjectValue)> {
    [("defaults", "42"), ("overridden", "99")]
        .into_iter()
        .map(|(name, count)| {
            (
                "api",
                name,
                ProjectValue::Record(vec![
                    ("count".to_owned(), number(count)),
                    ("r".to_owned(), number("3")),
                ]),
            )
        })
        .collect()
}

/// Expected cyclic node embeds only a typed target identity, never the target value.
fn expected_node(target: &str) -> ProjectValue {
    ProjectValue::Record(vec![(
        "next".to_owned(),
        ProjectValue::Reference(symbol("api", target)),
    )])
}

/// Literal public service result includes materialized private data and closed defaults.
fn expected_service() -> ProjectValue {
    ProjectValue::Record(vec![
        (
            "label".to_owned(),
            ProjectValue::String("private-value".to_owned()),
        ),
        (
            "ports".to_owned(),
            ProjectValue::List(vec![number("80"), number("443")]),
        ),
    ])
}

/// Supplies hard identity work ceilings, independent of package versioning.
fn identity_limits() -> IdentityLimits {
    IdentityLimits {
        bytes: MAX_TRANSCRIPT_BYTES,
        nodes: MAX_TRANSCRIPT_NODES,
    }
}

/// Independently validates compiler output before crossing the encoding boundary.
fn encoded(ir: &Arc<ProjectIr>, token: &CancellationToken) -> Vec<u8> {
    let reader = ValidatedProject::from_ir(Arc::clone(ir), ir.limits, token).unwrap();
    encode_project(&reader, token).unwrap()
}

/// Exercises exact source-to-public-result semantics, not merely encoder/decoder agreement.
fn positive(case: &Case) -> Vec<u8> {
    let token = CancellationToken::new();
    let captured = capture(case.units, case.vocabulary);
    let ir = compile_project(&captured, &token).expect(case.name);
    assert_eq!(ir.modules.len(), case.units.len(), "{}", case.name);
    assert_eq!(ir.declarations.len(), case.declarations, "{}", case.name);
    assert_eq!(ir.source_maps.len(), case.declarations);
    for entry in &ir.source_maps {
        let source = captured
            .sources()
            .iter()
            .find(|source| source.digest() == entry.location.source())
            .unwrap();
        let span = entry.location.span();
        assert!(span.end() <= u64::try_from(source.bytes().len()).unwrap());
        assert!(span.start() < span.end());
        let bytes = &source.bytes()
            [usize::try_from(span.start()).unwrap()..usize::try_from(span.end()).unwrap()];
        assert!(
            std::str::from_utf8(bytes)
                .unwrap()
                .contains(entry.declaration.declaration_name())
        );
    }
    let bytes = encoded(&ir, &token);
    let decoded = decode_project(&bytes, DecodeLimits::hard(), ir.limits, &token).unwrap();
    assert_eq!(decoded.complete_ir().as_ref(), ir.as_ref());
    let summary =
        inspect_project_encoded(&bytes, DecodeLimits::hard(), ir.limits, None, &token).unwrap();
    let expected = expected_values(case.name);
    assert_eq!(summary.view.values().len(), expected.len());
    for (module, name, value) in expected {
        let identity = symbol(module, name);
        let actual = summary
            .view
            .values()
            .iter()
            .find(|(id, _)| *id == identity)
            .unwrap();
        assert_eq!(actual.1, value, "{}::{name}", case.name);
    }
    let json: serde_json::Value =
        serde_json::from_str(&render_project_summary_json(&summary)).unwrap();
    assert_eq!(json["modules"], case.units.len().to_string());
    assert_eq!(json["declarations"], case.declarations.to_string());
    assert!(!render_project_summary_json(&summary).contains("source:"));
    assert!(!render_project_summary_json(&summary).contains("disconnected-private"));
    let identity = decoded.logical_identity(identity_limits(), &token).unwrap();
    for roots in [Some(&[][..]), None] {
        let view = inspect_project_encoded(&bytes, DecodeLimits::hard(), ir.limits, roots, &token)
            .unwrap();
        assert_eq!(view.logical_identity, identity.identity());
    }
    assert!(
        decode_project(
            &bytes[..bytes.len() - 1],
            DecodeLimits::hard(),
            ir.limits,
            &token
        )
        .is_err()
    );
    let mut reversed = case.units.to_vec();
    reversed.reverse();
    let reordered = compile_project(&capture(&reversed, case.vocabulary), &token).unwrap();
    assert_eq!(encoded(&reordered, &token), bytes);
    let mut cache = ProjectCompilationCache::new(ProjectCacheLimits {
        source_units: captured.limits().values().source_units,
        source_bytes: captured.limits().values().total_source_bytes,
    })
    .unwrap();
    assert_eq!(cache.compile(&captured, &token).unwrap().0, ir);
    let (warm, stats) = cache.compile(&captured, &token).unwrap();
    assert_eq!(warm, ir);
    assert_eq!(stats.parsed_units, 0);
    assert_eq!(stats.reused_units, u64::try_from(case.units.len()).unwrap());
    bytes
}

/// Registers separately reported pipeline tests for every accepted file project.
macro_rules! accepted {
    ($($function:ident => $name:literal),* $(,)?) => {$(
        /// Checks file-backed expected values, complete companions, wire and reader boundaries.
        #[test]
        fn $function() {
            positive(CASES.iter().find(|case| case.name == $name).unwrap());
        }
    )*};
}

accepted! {
    scalars => "scalars", unicode => "unicode", nullable_lists => "nullable-lists",
    records => "records", forward_reuse => "forward-reuse", reference_cycle => "reference-cycle",
    records_no_trailing_comma => "records-no-trailing-comma",
    single_line_fields => "single-line-fields",
    single_line_fields_with_comma => "single-line-fields-comma",
    locations => "locations", vocabulary_aliases => "vocabulary", cross_module => "imports",
    import_scc => "import-cycle",
}

/// Exact rejection phase, preventing a different failure from satisfying the oracle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    /// Import parsing and complete graph validation.
    Graph,
    /// Source parsing, name/type visibility and dependency resolution.
    Semantics,
    /// Contextual values/defaults after semantic analysis.
    Lowering,
}

/// Checks fail-closed publication and an original-byte diagnostic over exact input files.
fn rejected(units: &[(&str, &str)], phase: Phase, expected: &str) {
    let captured = capture(units, false);
    let failure = compile_project(&captured, &CancellationToken::new()).unwrap_err();
    assert_eq!(failure.schema(), PROJECT_RESULT_SCHEMA);
    let (actual_phase, code, location) = match failure {
        ProjectCompileFailure::Graph(failure) => {
            let first = &failure.diagnostics()[0];
            (Phase::Graph, first.code(), first.source_location())
        }
        ProjectCompileFailure::Semantics(failure) => {
            let first = &failure.diagnostics()[0];
            (
                Phase::Semantics,
                first.code(),
                Some(first.source_location()),
            )
        }
        ProjectCompileFailure::Lowering { code, location } => (Phase::Lowering, code, location),
    };
    assert_eq!((actual_phase, code), (phase, expected));
    let Some(location) = location else {
        // The published lowering failure contract explicitly makes location optional.
        assert_eq!(actual_phase, Phase::Lowering);
        return;
    };
    let source = captured
        .sources()
        .iter()
        .find(|source| source.digest() == location.source())
        .unwrap();
    assert!(location.span().end() <= u64::try_from(source.bytes().len()).unwrap());
}

/// Gives each negative source an independently asserted phase and stable diagnostic.
macro_rules! rejected_cases {
    ($($function:ident => ($file:literal, $phase:ident, $code:expr)),* $(,)?) => {$(
        /// Rejects the registered source at its expected phase without producing partial IR.
        #[test]
        fn $function() {
            rejected(&[source!("negative", $file, "api")], Phase::$phase, $code);
        }
    )*};
}

rejected_cases! {
    unknown_name => ("unknown-name", Semantics, semantics::INACCESSIBLE_NAME),
    private_type => ("private-type", Semantics, semantics::PRIVATE_PUBLIC_TYPE),
    private_reference => ("private-reference", Semantics, semantics::PRIVATE_REFERENCE),
    indirect_private_reference => ("indirect-private-reference", Semantics, semantics::PRIVATE_REFERENCE),
    type_mismatch => ("type-mismatch", Lowering, lowering::INVALID_VALUE),
    nominal_mismatch => ("nominal-mismatch", Semantics, semantics::TYPE_MISMATCH),
    duplicate_field => ("duplicate-field", Lowering, lowering::INVALID_VALUE),
    unknown_field => ("unknown-field", Lowering, lowering::INVALID_VALUE),
    missing_field => ("missing-field", Lowering, lowering::INVALID_VALUE),
    list_element => ("list-element", Lowering, lowering::INVALID_VALUE),
    trailing_token => ("trailing-token", Semantics, semantics::INVALID_SOURCE),
    duplicate_public => ("duplicate-public", Semantics, semantics::INVALID_PUBLIC),
    unterminated_string => ("unterminated-string", Graph, graph::INVALID_SYNTAX),
    value_cycle => ("value-cycle", Semantics, semantics::SEMANTIC_CYCLE),
    missing_import => ("missing-import", Graph, graph::MISSING_IMPORT),
    self_import => ("self-import", Graph, graph::SELF_IMPORT),
    wildcard_import => ("wildcard-import", Graph, graph::FORBIDDEN_IMPORT),
    url_import => ("url-import", Graph, graph::INVALID_SYNTAX),
    nonnullable_null => ("nonnullable-null", Lowering, lowering::INVALID_VALUE),
    reference_type => ("reference-type", Semantics, semantics::TYPE_MISMATCH),
    default_type => ("default-type", Lowering, lowering::INVALID_VALUE),
    relative_import => ("relative-import", Graph, graph::FORBIDDEN_IMPORT),
    forbidden_function => ("function", Semantics, semantics::INVALID_SOURCE),
    record_value_cycle => ("record-value-cycle", Semantics, semantics::SEMANTIC_CYCLE),
    missing_field_separator => ("missing-field-separator", Semantics, semantics::INVALID_SOURCE),
    duplicate_final_field => ("duplicate-final-field", Semantics, semantics::INVALID_SOURCE),
}

/// Optional final punctuation changes captured bytes, not defaults, signatures or logical identity.
#[test]
fn optional_final_comma_preserves_complete_meaning() {
    let token = CancellationToken::new();
    let compile = |name| {
        let case = CASES.iter().find(|case| case.name == name).unwrap();
        compile_project(&capture(case.units, false), &token).unwrap()
    };
    let with_comma = compile("records");
    let without_comma = compile("records-no-trailing-comma");
    assert!(with_comma.logical_eq(&without_comma));
    assert_eq!(
        with_comma.public_interface.fingerprint(),
        without_comma.public_interface.fingerprint()
    );
    assert_ne!(with_comma.sources, without_comma.sources);
    let inspect = |ir: &Arc<ProjectIr>| {
        inspect_project_encoded(
            &encoded(ir, &token),
            DecodeLimits::hard(),
            ir.limits,
            None,
            &token,
        )
        .unwrap()
    };
    assert_eq!(
        inspect(&with_comma).logical_identity,
        inspect(&without_comma).logical_identity
    );
}

/// Two fields on one line retain identical defaults and identities with either closing style.
#[test]
fn single_line_field_comma_styles_are_equivalent() {
    let token = CancellationToken::new();
    let compile = |name| {
        let case = CASES.iter().find(|case| case.name == name).unwrap();
        compile_project(&capture(case.units, false), &token).unwrap()
    };
    let plain = compile("single-line-fields");
    let comma = compile("single-line-fields-comma");
    assert!(plain.logical_eq(&comma));
    assert_eq!(
        plain.public_interface.fingerprint(),
        comma.public_interface.fingerprint()
    );
    let inspect = |ir: &Arc<ProjectIr>| {
        inspect_project_encoded(
            &encoded(ir, &token),
            DecodeLimits::hard(),
            ir.limits,
            None,
            &token,
        )
        .unwrap()
    };
    assert_eq!(
        inspect(&plain).logical_identity,
        inspect(&comma).logical_identity
    );
}

/// Declaration ordering and exact number spelling alter source facts but not logical identity.
#[test]
fn declaration_order_and_number_spelling_are_nonsemantic() {
    let token = CancellationToken::new();
    let original = compile_project(
        &capture(&[source!("positive", "scalars", "api")], false),
        &token,
    )
    .unwrap();
    let changed = compile_project(
        &capture(&[source!("positive", "scalars-reordered", "api")], false),
        &token,
    )
    .unwrap();
    assert!(original.logical_eq(&changed));
    assert_ne!(original.sources, changed.sources);
    let read = |ir: &Arc<ProjectIr>| {
        inspect_project_encoded(
            &encoded(ir, &token),
            DecodeLimits::hard(),
            ir.limits,
            None,
            &token,
        )
        .unwrap()
    };
    assert_eq!(
        read(&original).logical_identity,
        read(&changed).logical_identity
    );
}

/// A changed real source reparses only that unit and cannot leave stale cross-module values.
#[test]
fn incremental_file_change_matches_clean_public_results() {
    let token = CancellationToken::new();
    let case = CASES.iter().find(|case| case.name == "imports").unwrap();
    let before = capture(case.units, false);
    let after = capture(
        &[
            source!("positive", "consumer", "app::consumer"),
            source!("positive", "orphan", "app::orphan"),
            source!("positive", "shared-updated", "app::shared"),
        ],
        false,
    );
    let mut cache = ProjectCompilationCache::new(ProjectCacheLimits {
        source_units: before.limits().values().source_units,
        source_bytes: before.limits().values().total_source_bytes,
    })
    .unwrap();
    let (old, _) = cache.compile(&before, &token).unwrap();
    let (updated, stats) = cache.compile(&after, &token).unwrap();
    assert_eq!(stats.parsed_units, 1);
    assert_eq!(stats.reused_units, 2);
    assert_eq!(updated, compile_project(&after, &token).unwrap());
    assert!(!updated.logical_eq(&old));
    let bytes = encoded(&updated, &token);
    let summary =
        inspect_project_encoded(&bytes, DecodeLimits::hard(), updated.limits, None, &token)
            .unwrap();
    let answer = summary
        .view
        .values()
        .iter()
        .find(|(id, _)| *id == symbol("app::consumer", "answer"))
        .unwrap();
    assert_eq!(answer.1, number("43"));
    let copied = summary
        .view
        .values()
        .iter()
        .find(|(id, _)| *id == symbol("app::consumer", "copied"))
        .unwrap();
    assert_eq!(
        copied.1,
        ProjectValue::Record(vec![
            (
                "label".to_owned(),
                ProjectValue::String("private-updated".to_owned())
            ),
            (
                "ports".to_owned(),
                ProjectValue::List(vec![number("80"), number("443")])
            ),
        ])
    );
    assert_eq!(cache.compile(&before, &token).unwrap().0, old);
}

/// Imported private values remain inaccessible even when their module is captured.
#[test]
fn private_import() {
    rejected(
        &[
            source!("negative", "private-import", "app::consumer"),
            source!("positive", "shared", "app::shared"),
        ],
        Phase::Semantics,
        semantics::INACCESSIBLE_NAME,
    );
}

/// Import SCCs are legal, but ordinary value cycles across them still fail.
#[test]
fn cross_module_value_cycle() {
    rejected(
        &[
            source!("negative", "cycle-broken", "cycle::left"),
            source!("positive", "cycle-right", "cycle::right"),
        ],
        Phase::Semantics,
        semantics::SEMANTIC_CYCLE,
    );
}

/// Alias spelling changes captured evidence, not complete logical meaning or identity.
#[test]
fn alias_renaming_preserves_logical_identity() {
    let token = CancellationToken::new();
    let case = CASES.iter().find(|case| case.name == "imports").unwrap();
    let original = compile_project(&capture(case.units, false), &token).unwrap();
    let changed = compile_project(
        &capture(
            &[
                source!("positive", "consumer-renamed", "app::consumer"),
                source!("positive", "orphan", "app::orphan"),
                source!("positive", "shared", "app::shared"),
            ],
            false,
        ),
        &token,
    )
    .unwrap();
    assert!(original.logical_eq(&changed));
    assert_ne!(original.sources, changed.sources);
    let read = |ir: &Arc<ProjectIr>| {
        inspect_project_encoded(
            &encoded(ir, &token),
            DecodeLimits::hard(),
            ir.limits,
            None,
            &token,
        )
        .unwrap()
    };
    assert_eq!(
        read(&original).logical_identity,
        read(&changed).logical_identity
    );
}

/// Complete artifacts and native public results remain equal across parallel compilations.
#[test]
fn concurrent_pipeline_is_deterministic() {
    let case = CASES.iter().find(|case| case.name == "imports").unwrap();
    let expected = positive(case);
    thread::scope(|scope| {
        let workers = (0..4)
            .map(|_| scope.spawn(|| positive(case)))
            .collect::<Vec<_>>();
        for worker in workers {
            assert_eq!(worker.join().unwrap(), expected);
        }
    });
}

/// Unique native-test root suffix across concurrently scheduled tests.
static ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Owns only this test's temporary source-derived artifacts.
struct TestRoot(PathBuf);

impl TestRoot {
    /// Creates a process-unique directory and never uses a caller-provided cleanup path.
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "neutral-source-pipeline-{}-{}",
            std::process::id(),
            ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestRoot {
    /// Cleans only the exact temporary root this test created, including on assertion failure.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Builds a requested executable offline and discovers its path from Cargo's typed messages.
fn executable(package: &str) -> PathBuf {
    let output = Command::new(env!("CARGO"))
        .args([
            "build",
            "--offline",
            "--locked",
            "--manifest-path",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"),
            "--package",
            package,
            "--bin",
            package,
            "--message-format=json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|value| {
            if value["reason"] == "compiler-artifact" && value["target"]["name"] == package {
                value["executable"].as_str().map(PathBuf::from)
            } else {
                None
            }
        })
        .expect("Cargo must identify the requested binary")
}

/// Real executable JSON agrees with asserted source semantics; corrupt/private selections fail.
#[test]
fn system_source_to_standalone_probe() {
    let executable = executable("neutral-probe");
    let root = TestRoot::new();
    for case in CASES {
        let bytes = positive(case);
        let path = root.0.join(format!("{}.nir", case.name));
        fs::write(&path, &bytes).unwrap();
        let output = Command::new(&executable)
            .arg("--json")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}: {}",
            case.name,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stderr, [] as [u8; 0]);
        let limits = compile_project(
            &capture(case.units, case.vocabulary),
            &CancellationToken::new(),
        )
        .unwrap()
        .limits;
        let summary = inspect_project_encoded(
            &bytes,
            DecodeLimits::hard(),
            limits,
            None,
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            render_project_summary_json(&summary)
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        if case.name == "imports" {
            check_executable_roots(&executable, &path, &bytes, limits);
        }
        fs::write(&path, &bytes[..bytes.len() - 1]).unwrap();
        let corrupt = Command::new(&executable)
            .arg("--json")
            .arg(&path)
            .output()
            .unwrap();
        assert!(!corrupt.status.success());
        assert_eq!(corrupt.stdout, [] as [u8; 0]);
        assert!(String::from_utf8_lossy(&corrupt.stderr).starts_with("[error]"));
    }
}

/// Actual public selection retains identity/dependencies; private roots publish no partial JSON.
fn check_executable_roots(
    executable: &std::path::Path,
    artifact: &std::path::Path,
    bytes: &[u8],
    limits: neutral_ir::project::ProjectLimits,
) {
    let roots = ["app::consumer::copied".to_owned()];
    let output = Command::new(executable)
        .args(["--json", "--root", &roots[0]])
        .arg(artifact)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary = inspect_project_encoded(
        bytes,
        DecodeLimits::hard(),
        limits,
        Some(&roots),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        render_project_summary_json(&summary)
    );
    assert!(
        summary
            .view
            .exports()
            .iter()
            .any(|export| *export.identity() == symbol("app::shared", "Service"))
    );
    let private = Command::new(executable)
        .args(["--json", "--root", "app::shared::secret"])
        .arg(artifact)
        .output()
        .unwrap();
    assert!(!private.status.success());
    assert_eq!(private.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&private.stderr).starts_with("[error]"));
}
