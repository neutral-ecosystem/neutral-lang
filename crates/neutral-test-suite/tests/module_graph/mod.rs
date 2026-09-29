// SPDX-License-Identifier: Apache-2.0

//! Executable module-graph corpus and adversarial validation.

use crate::project_capture::{
    Table, fixture_limits, number, parse_fixture, request_fixture, required_string, string,
};
use neutral_compiler::{
    CAPTURE_REQUEST_VERSION, CapturedProject, CapturedProjectRequest, CapturedSourceInput,
    ModuleGraph, ModuleGraphFailure, ProjectCaptureControls, ProjectCaptureError,
    ProjectCaptureLimits, build_module_graph, capture_project,
};
use neutral_core::CancellationToken;
use std::{collections::BTreeMap, thread};

/// Exact crate-owned copies of all 13 reviewed module-graph request fixtures.
const CASES: &[&str] = &[
    include_str!("fixtures/positive/valid-cycle.toml"),
    include_str!("fixtures/negative/semantic-cycle.toml"),
    include_str!("fixtures/negative/missing-import.toml"),
    include_str!("fixtures/negative/self-import.toml"),
    include_str!("fixtures/negative/duplicate-import.toml"),
    include_str!("fixtures/negative/alias-collision.toml"),
    include_str!("fixtures/negative/forbidden-import-forms.toml"),
    include_str!("fixtures/boundary/import-edges-exact.toml"),
    include_str!("fixtures/boundary/import-edges-over.toml"),
    include_str!("fixtures/boundary/imports-per-module-exact.toml"),
    include_str!("fixtures/boundary/imports-per-module-over.toml"),
    include_str!("fixtures/boundary/scc-units-exact.toml"),
    include_str!("fixtures/boundary/scc-units-over.toml"),
];

/// Parses all three pinned module-graph oracle tables by case ID.
fn oracle_cases() -> BTreeMap<String, Table> {
    let mut cases = BTreeMap::new();
    for text in [
        include_str!("oracles/graph.toml"),
        include_str!("oracles/import-errors.toml"),
        include_str!("oracles/limits.toml"),
    ] {
        let parsed = parse_fixture(text);
        assert_eq!(number(&parsed.root, "schema_version"), 1);
        for case in parsed.arrays.get("case").expect("oracle cases must exist") {
            let id = required_string(case, "id");
            assert!(
                cases.insert(id, case.clone()).is_none(),
                "oracle ID must be unique"
            );
        }
    }
    cases
}

/// Decodes one reviewed TOML array of quoted strings, preserving embedded commas.
fn string_array(value: &str) -> Vec<String> {
    let body = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .expect("oracle array must be bracketed");
    let mut values = Vec::new();
    let mut token = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for character in body.chars() {
        if escaped {
            token.push(character);
            escaped = false;
        } else if character == '\\' {
            token.push(character);
            escaped = true;
        } else if character == '"' {
            token.push(character);
            in_string = !in_string;
        } else if character == ',' && !in_string {
            if !token.trim().is_empty() {
                values.push(string(token.trim()));
            }
            token.clear();
        } else {
            token.push(character);
        }
    }
    assert!(!in_string && !escaped, "reviewed oracle strings must close");
    if !token.trim().is_empty() {
        values.push(string(token.trim()));
    }
    values
}

/// Returns a required list-valued field from one oracle case.
fn required_array(case: &Table, key: &str) -> Vec<String> {
    string_array(case.get(key).expect("oracle list must exist"))
}

/// Returns every ordered SCC as the contract's comma-joined member representation.
fn component_names(graph: &ModuleGraph) -> Vec<String> {
    graph
        .components()
        .iter()
        .map(|component| {
            component
                .modules()
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .join(",")
        })
        .collect()
}

/// Compares an accepted graph against every applicable frozen oracle field.
fn assert_graph(case: &Table, graph: &ModuleGraph, id: &str) {
    if case.contains_key("ordered_modules") {
        assert_eq!(
            graph
                .modules()
                .iter()
                .map(|module| module.module_id().to_owned())
                .collect::<Vec<_>>(),
            required_array(case, "ordered_modules"),
            "{id} modules"
        );
    }
    if case.contains_key("ordered_edges") {
        assert_eq!(
            graph
                .edges()
                .iter()
                .map(|edge| format!("{} -> {}", edge.from(), edge.target()))
                .collect::<Vec<_>>(),
            required_array(case, "ordered_edges"),
            "{id} edges"
        );
    }
    assert_eq!(
        component_names(graph),
        required_array(case, "dependency_first_sccs"),
        "{id} SCCs"
    );
    if case.contains_key("retained_disconnected_modules") {
        for module in required_array(case, "retained_disconnected_modules") {
            assert!(
                graph
                    .modules()
                    .iter()
                    .any(|member| member.module_id() == module),
                "{id} must retain {module}"
            );
        }
    }
}

/// Compares a rejected graph against all applicable frozen diagnostic fields.
fn assert_failure(case: &Table, failure: &ModuleGraphFailure, id: &str) {
    let diagnostics = failure.diagnostics();
    assert!(!diagnostics.is_empty(), "{id} must report a failure");
    if let Some(codes) = case.get("ordered_diagnostic_codes") {
        assert_eq!(
            diagnostics
                .iter()
                .map(|item| item.code().to_owned())
                .collect::<Vec<_>>(),
            string_array(codes),
            "{id} diagnostic codes"
        );
    } else {
        assert_eq!(
            diagnostics[0].code(),
            required_string(case, "diagnostic_code"),
            "{id}"
        );
    }
    if let Some(modules) = case.get("ordered_diagnostic_modules") {
        assert_eq!(
            diagnostics
                .iter()
                .map(|item| item.module_id().to_owned())
                .collect::<Vec<_>>(),
            string_array(modules),
            "{id} diagnostic modules"
        );
    }
    if case.contains_key("diagnostic_module") {
        assert_eq!(
            diagnostics[0].module_id(),
            required_string(case, "diagnostic_module")
        );
    }
    if case.contains_key("diagnostic_target") {
        assert_eq!(
            diagnostics[0].target(),
            required_string(case, "diagnostic_target")
        );
    }
    if case.contains_key("diagnostic_alias") {
        assert_eq!(
            diagnostics[0].alias(),
            required_string(case, "diagnostic_alias")
        );
    }
    assert_eq!(required_string(case, "partial_graph"), "absent");
}

/// Builds a graph or bounded failure from an already accepted fixture request.
fn graph_result(text: &str) -> Result<ModuleGraph, ModuleGraphFailure> {
    let project =
        capture_project(request_fixture(text)).expect("module-graph fixture must capture");
    build_module_graph(&project, &CancellationToken::new())
}

/// Reconstructs a request in a different source submission order.
fn reordered_request(project: &CapturedProject, rotation: usize) -> CapturedProjectRequest {
    let mut sources = project
        .sources()
        .iter()
        .map(|source| {
            CapturedSourceInput::new(
                source.source_id(),
                source.module_id(),
                source.bytes().to_vec(),
            )
        })
        .collect::<Vec<_>>();
    if !sources.is_empty() {
        let length = sources.len();
        sources.rotate_left(rotation % length);
    }
    sources.reverse();
    CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        project.profile(),
        sources,
        Vec::new(),
        ProjectCaptureControls::new(project.limits(), CancellationToken::new()),
    )
}

#[test]
/// Every reviewed fixture has exactly one executable oracle and matches it.
fn conformance_all_graph_fixtures_match_their_oracles() {
    let oracles = oracle_cases();
    assert_eq!(
        oracles.len(),
        CASES.len(),
        "every oracle must have a fixture"
    );
    for text in CASES {
        let fixture = parse_fixture(text);
        let id = required_string(&fixture.root, "case_id");
        let case = oracles.get(&id).expect("fixture must have an oracle");
        let outcome = required_string(case, "expected_outcome");
        match (outcome.as_str(), graph_result(text)) {
            ("accept-graph", Ok(graph)) => assert_graph(case, &graph, &id),
            ("reject-graph", Err(failure)) => assert_failure(case, &failure, &id),
            (expected, actual) => panic!("{id}: expected {expected}, received {actual:?}"),
        }
        if case.contains_key("limit") {
            let controls = fixture.tables.get("controls").expect("limits must exist");
            assert_eq!(
                number(controls, &required_string(case, "limit")),
                number(case, "configured"),
                "{id} configured limit"
            );
        }
    }
}

#[test]
/// All fixture outcomes remain identical after input shuffling and concurrent execution.
fn determinism_corpus_is_shuffle_and_concurrency_invariant() {
    for text in CASES {
        let project = capture_project(request_fixture(text)).expect("fixture must capture");
        let baseline = build_module_graph(&project, &CancellationToken::new());
        let mut shuffled_projects = Vec::new();
        for rotation in 0..project.sources().len().max(1) {
            let shuffled = capture_project(reordered_request(&project, rotation))
                .expect("reordered closure must capture");
            assert_eq!(
                build_module_graph(&shuffled, &CancellationToken::new()),
                baseline,
                "shuffled case {}",
                required_string(&parse_fixture(text).root, "case_id")
            );
            shuffled_projects.push(shuffled);
        }
        thread::scope(|scope| {
            for worker in 0..8 {
                let project = &shuffled_projects[worker % shuffled_projects.len()];
                let baseline = &baseline;
                scope.spawn(move || {
                    for _ in 0..16 {
                        assert_eq!(
                            build_module_graph(project, &CancellationToken::new()),
                            *baseline
                        );
                    }
                });
            }
        });
    }
}

#[test]
/// Public graph facts and cross-unit diagnostics retain exact captured source maps.
fn integration_public_graph_locations_cover_every_unit() {
    let valid = capture_project(request_fixture(include_str!(
        "fixtures/positive/valid-cycle.toml"
    )))
    .expect("valid graph request must capture");
    let graph = valid
        .module_graph(&CancellationToken::new())
        .expect("public project graph must build");
    for module in graph.modules() {
        let source = valid
            .sources()
            .iter()
            .find(|source| source.source_id() == module.source_id())
            .expect("module source ID must name a captured unit");
        let location = module.source_location();
        assert_eq!(location.source(), source.digest());
        assert!(
            usize::try_from(location.span().end()).is_ok_and(|end| end <= source.bytes().len())
        );
    }
    for edge in graph.edges() {
        let source = valid
            .sources()
            .iter()
            .find(|source| source.source_id() == edge.source_id())
            .expect("import source ID must name a captured unit");
        let location = edge.source_location();
        assert_eq!(location.source(), source.digest());
        assert!(
            usize::try_from(location.span().end()).is_ok_and(|end| end <= source.bytes().len())
        );
        assert_eq!(edge.from(), source.module_id());
    }

    let invalid = capture_project(request_fixture(include_str!(
        "fixtures/negative/forbidden-import-forms.toml"
    )))
    .expect("invalid graph request must still capture");
    let failure = invalid
        .module_graph(&CancellationToken::new())
        .expect_err("public graph must expose source-accounted diagnostics");
    assert_eq!(failure.diagnostics().len(), invalid.sources().len());
    for diagnostic in failure.diagnostics() {
        let source = invalid
            .sources()
            .iter()
            .find(|source| source.source_id() == diagnostic.source_id())
            .expect("graph diagnostic must name its captured source unit");
        let location = diagnostic
            .source_location()
            .expect("source-accounted failure must retain a typed location");
        assert_eq!(location.source(), source.digest());
        assert_eq!(location.span(), diagnostic.span());
        assert!(
            usize::try_from(location.span().end()).is_ok_and(|end| end <= source.bytes().len())
        );
    }
}

#[test]
/// An edited closure rebuilt from captured facts equals a fresh clean request.
fn integration_incremental_recapture_equals_clean_graph() {
    let original = capture_project(request_fixture(include_str!(
        "fixtures/positive/valid-cycle.toml"
    )))
    .expect("original request must capture");
    let original_graph = original
        .module_graph(&CancellationToken::new())
        .expect("original graph must build");
    let edited_sources = original
        .sources()
        .iter()
        .map(|source| {
            let bytes = if source.module_id() == "graph::alpha" {
                b"neu \"1.0\"\nmodule graph::alpha\n".to_vec()
            } else {
                source.bytes().to_vec()
            };
            CapturedSourceInput::new(source.source_id(), source.module_id(), bytes)
        })
        .collect::<Vec<_>>();
    let recaptured = capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        original.profile(),
        edited_sources.clone(),
        Vec::new(),
        ProjectCaptureControls::new(original.limits(), CancellationToken::new()),
    ))
    .expect("edited captured closure must be accepted");
    let mut clean_sources = edited_sources;
    clean_sources.reverse();
    let clean = capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        original.profile(),
        clean_sources,
        Vec::new(),
        ProjectCaptureControls::new(original.limits(), CancellationToken::new()),
    ))
    .expect("clean request must be accepted");
    assert!(recaptured.meaning_equivalent(&clean));
    let edited_graph = recaptured
        .module_graph(&CancellationToken::new())
        .expect("edited graph must build");
    assert_eq!(
        edited_graph,
        clean
            .module_graph(&CancellationToken::new())
            .expect("clean graph must build")
    );
    assert_ne!(edited_graph, original_graph);
    assert_eq!(original_graph.edges().len(), 2);
    assert_eq!(edited_graph.edges().len(), 1);
}

#[test]
/// Multiple graph failures preserve source order, diagnostic bounds, and recovery.
fn validation_diagnostic_bound_and_recovery() {
    let valid = capture_project(request_fixture(include_str!(
        "fixtures/positive/valid-cycle.toml"
    )))
    .expect("valid fixture must capture");
    let invalid = capture_project(request_fixture(include_str!(
        "fixtures/negative/forbidden-import-forms.toml"
    )))
    .expect("invalid graph fixture must still capture");
    let first = build_module_graph(&invalid, &CancellationToken::new())
        .expect_err("forbidden import graph must fail");
    assert_eq!(first.diagnostics().len(), 4);
    assert!(
        first
            .diagnostics()
            .windows(2)
            .all(|pair| pair[0].module_id() < pair[1].module_id())
    );
    assert_eq!(
        build_module_graph(&invalid, &CancellationToken::new()),
        Err(first),
        "a failure cannot publish a partial graph or poison later attempts"
    );
    assert!(build_module_graph(&valid, &CancellationToken::new()).is_ok());
}

#[test]
/// Diagnostic overflow is an explicit limit failure, not silent truncation.
fn validation_diagnostic_limit_is_fail_closed() {
    let mut values = valid_limits().values();
    values.diagnostics = 1;
    values.imports_per_module = 2;
    values.import_edges = 2;
    let project = capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        neutral_core::profile::LanguageProfile::V1_0,
        vec![CapturedSourceInput::new(
            "source:overflow",
            "overflow::main",
            b"neu \"1.0\"\nmodule overflow::main\nimport overflow::absent_a as a\nimport overflow::absent_b as b\n".to_vec(),
        )],
        Vec::new(),
        ProjectCaptureControls::new(ProjectCaptureLimits::new(values), CancellationToken::new()),
    ))
    .expect("two unresolved imports must still capture");
    let failure = build_module_graph(&project, &CancellationToken::new())
        .expect_err("diagnostic overflow must fail");
    assert_eq!(failure.diagnostics().len(), 1);
    assert_eq!(failure.diagnostics()[0].code(), "NEU-MOD-007");
}

#[test]
/// A duplicate target takes precedence over a colliding alias on the same import.
fn validation_ambiguous_import_has_stable_precedence() {
    let mut values = valid_limits().values();
    values.imports_per_module = 2;
    values.import_edges = 2;
    let project = capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        neutral_core::profile::LanguageProfile::V1_0,
        vec![
            CapturedSourceInput::new(
                "source:main",
                "ambiguous::main",
                b"neu \"1.0\"\nmodule ambiguous::main\nimport ambiguous::shared as shared\nimport ambiguous::shared as shared\n".to_vec(),
            ),
            CapturedSourceInput::new(
                "source:shared",
                "ambiguous::shared",
                b"neu \"1.0\"\nmodule ambiguous::shared\n".to_vec(),
            ),
        ],
        Vec::new(),
        ProjectCaptureControls::new(ProjectCaptureLimits::new(values), CancellationToken::new()),
    ))
    .expect("ambiguous imports must still capture");
    let failure = build_module_graph(&project, &CancellationToken::new())
        .expect_err("duplicate target must fail");
    assert_eq!(failure.diagnostics()[0].code(), "NEU-MOD-004");
}

#[test]
/// Forbidden acquisition, re-export, and partial-module forms never resolve by host action.
fn security_exclusion_audit() {
    for (source, expected) in [
        ("import audit::* as all\n", "NEU-MOD-006"),
        ("import ::audit::target as target\n", "NEU-MOD-006"),
        ("import ../audit/target as target\n", "NEU-MOD-006"),
        ("import \"/tmp/audit.neu\" as target\n", "NEU-MOD-006"),
        (
            "import \"https://example.invalid/audit\" as target\n",
            "NEU-MOD-006",
        ),
        ("public import audit::target as target\n", "NEU-MOD-006"),
        ("import audit::target\n", "NEU-MOD-001"),
        ("import audit::missing as missing\n", "NEU-MOD-002"),
    ] {
        let request = CapturedProjectRequest::new(
            CAPTURE_REQUEST_VERSION,
            neutral_core::profile::LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "source:audit",
                "audit::main",
                format!("neu \"1.0\"\nmodule audit::main\n{source}").into_bytes(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(valid_limits(), CancellationToken::new()),
        );
        let captured = capture_project(request).expect("source must capture without a resolver");
        let failure = build_module_graph(&captured, &CancellationToken::new())
            .expect_err("excluded form must not produce an import graph");
        assert_eq!(failure.diagnostics()[0].code(), expected, "{source}");
        assert!(
            failure
                .diagnostics()
                .iter()
                .all(|item| item.target().is_empty() || item.target().starts_with("audit::"))
        );
    }
    let partial = CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        neutral_core::profile::LanguageProfile::V1_0,
        vec![CapturedSourceInput::new(
            "source:partial",
            "audit::main",
            b"neu \"1.0\"\nmodule audit::main\nmodule audit::extra\n".to_vec(),
        )],
        Vec::new(),
        ProjectCaptureControls::new(valid_limits(), CancellationToken::new()),
    );
    assert_eq!(
        capture_project(partial),
        Err(ProjectCaptureError::InvalidHeader),
        "one captured unit cannot grow a second partial module"
    );
}

/// Returns the reviewed valid-cycle control limits for local exclusion probes.
fn valid_limits() -> neutral_compiler::ProjectCaptureLimits {
    let fixture = parse_fixture(include_str!("fixtures/positive/valid-cycle.toml"));
    fixture_limits(&fixture)
}
