// SPDX-License-Identifier: Apache-2.0

//! Core graph construction, boundaries, and fail-closed behavior.

use super::*;
use crate::{
    CAPTURE_REQUEST_VERSION, CapturedProjectRequest, CapturedSourceInput, CapturedVocabularyInput,
    ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits, capture_project,
};
use neutral_core::{VocabularyContentDigest, profile::LanguageProfile};
use neutral_vocabulary::{VOCABULARY_ENCODING_VERSION, VOCABULARY_SCHEMA_VERSION, VocabularyLock};

/// Returns complete explicit limits for one graph test request.
fn limits(source_units: u64) -> ProjectCaptureLimitValues {
    ProjectCaptureLimitValues {
        total_source_bytes: 256_000,
        source_bytes_per_unit: 1024,
        source_units,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 1,
        vocabulary_bytes_per_unit: 1024,
        total_vocabulary_bytes: 1024,
        imports_per_module: 8,
        import_edges: source_units.saturating_mul(8),
        scc_units: source_units,
        declarations: 1024,
        diagnostics: 16,
        output_bytes: 256_000,
    }
}

/// Captures exact source text without involving a host path or resolver.
fn captured(sources: &[(&str, &str)], limits: ProjectCaptureLimitValues) -> CapturedProject {
    captured_with_vocabulary(sources, limits, Vec::new())
}

/// Captures exact sources with any already-acquired vocabulary bundles.
fn captured_with_vocabulary(
    sources: &[(&str, &str)],
    limits: ProjectCaptureLimitValues,
    vocabularies: Vec<CapturedVocabularyInput>,
) -> CapturedProject {
    let input = sources
        .iter()
        .enumerate()
        .map(|(index, (module, body))| {
            CapturedSourceInput::new(
                format!("source:{index}"),
                *module,
                format!("neu \"1.0\"\nmodule {module}\n{body}").into_bytes(),
            )
        })
        .collect();
    capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        input,
        vocabularies,
        ProjectCaptureControls::new(ProjectCaptureLimits::new(limits), CancellationToken::new()),
    ))
    .expect("test source set must capture")
}

/// Constructs one exact vocabulary lock for a graph alias test.
fn vocabulary(identity: &str) -> CapturedVocabularyInput {
    let bytes = b"{}\n".to_vec();
    let lock = VocabularyLock::new(
        identity,
        "1.0.0",
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&bytes),
        Vec::new(),
    )
    .expect("test vocabulary lock must be valid");
    CapturedVocabularyInput::new(bytes, lock)
}

/// Returns the first stable graph code for one rejected source set.
fn first_code(sources: &[(&str, &str)], limits: ProjectCaptureLimitValues) -> &'static str {
    build_module_graph(&captured(sources, limits), &CancellationToken::new())
        .expect_err("graph must fail")
        .diagnostics()[0]
        .code()
}

#[test]
/// Cyclic imports and disconnected members yield a stable dependency-first graph.
fn valid_cycle_and_shuffled_input_are_deterministic() {
    let sources = [
        ("graph::zeta", "import graph::alpha as alpha\n"),
        ("graph::orphan", ""),
        ("graph::alpha", "import graph::zeta as zeta\n"),
    ];
    let graph = build_module_graph(&captured(&sources, limits(3)), &CancellationToken::new())
        .expect("import SCC must be valid");
    assert_eq!(
        graph
            .modules()
            .iter()
            .map(GraphModule::module_id)
            .collect::<Vec<_>>(),
        ["graph::alpha", "graph::orphan", "graph::zeta"]
    );
    assert_eq!(
        graph
            .components()
            .iter()
            .map(|component| component
                .modules()
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        [vec!["graph::alpha", "graph::zeta"], vec!["graph::orphan"]]
    );
    assert_eq!(
        graph
            .edges()
            .iter()
            .map(|edge| (edge.from(), edge.target()))
            .collect::<Vec<_>>(),
        [
            ("graph::alpha", "graph::zeta"),
            ("graph::zeta", "graph::alpha")
        ]
    );
    let shuffled = [sources[2], sources[0], sources[1]];
    let other = build_module_graph(&captured(&shuffled, limits(3)), &CancellationToken::new())
        .expect("shuffled import SCC must be valid");
    assert_eq!(graph.components(), other.components());
    assert_eq!(
        graph
            .edges()
            .iter()
            .map(|edge| (edge.from(), edge.target(), edge.alias()))
            .collect::<Vec<_>>(),
        other
            .edges()
            .iter()
            .map(|edge| (edge.from(), edge.target(), edge.alias()))
            .collect::<Vec<_>>()
    );
}

#[test]
/// A semantic value cycle is still a valid import graph.
fn semantic_cycle_does_not_invalidate_import_graph() {
    let project = captured(
        &[
            (
                "cycle::left",
                "import cycle::right as right\npublic num value = right::value\n",
            ),
            (
                "cycle::right",
                "import cycle::left as left\npublic num value = left::value\n",
            ),
        ],
        limits(2),
    );
    let graph = build_module_graph(&project, &CancellationToken::new())
        .expect("graph construction must accept import SCCs independently of value semantics");
    assert_eq!(graph.components().len(), 1);
    assert_eq!(graph.components()[0].modules().len(), 2);
}

#[test]
/// Missing, self, duplicate, alias, and forbidden imports receive frozen codes.
fn invalid_import_classes_fail_without_a_partial_graph() {
    let bound = limits(3);
    assert_eq!(
        first_code(
            &[("missing::main", "import missing::absent as absent\n")],
            bound
        ),
        diagnostics::MISSING_IMPORT
    );
    assert_eq!(
        first_code(
            &[("self_case::main", "import self_case::main as local\n")],
            bound
        ),
        diagnostics::SELF_IMPORT
    );
    assert_eq!(
        first_code(
            &[
                (
                    "duplicate::main",
                    "import duplicate::shared as first\nimport duplicate::shared as second\n"
                ),
                ("duplicate::shared", ""),
            ],
            bound
        ),
        diagnostics::DUPLICATE_IMPORT
    );
    assert_eq!(
        first_code(
            &[
                (
                    "alias::main",
                    "import alias::one as shared\nimport alias::two as shared\n"
                ),
                ("alias::one", ""),
                ("alias::two", ""),
            ],
            bound
        ),
        diagnostics::ALIAS_COLLISION
    );
    for source in [
        "import forbidden::* as all\n",
        "import ::forbidden::target as target\n",
        "import \"https://example.invalid/mod\" as target\n",
        "public import forbidden::target as target\n",
    ] {
        assert_eq!(
            first_code(&[("forbidden::main", source)], bound),
            diagnostics::FORBIDDEN_IMPORT
        );
    }
    assert_eq!(
        first_code(
            &[("missing_alias::main", "import missing_alias::target\n")],
            bound
        ),
        diagnostics::INVALID_SYNTAX
    );
    for source in ["import\n", "import missing\n", "import missing as\n"] {
        assert_eq!(
            first_code(&[("truncated::main", source)], bound),
            diagnostics::INVALID_SYNTAX
        );
    }
}

#[test]
/// Graph diagnostics sort by module ID even when sources arrive out of order.
fn diagnostic_order_is_canonical() {
    let project = captured(
        &[
            ("problem::zeta", "import problem::absent as absent\n"),
            ("problem::alpha", "import problem::missing as missing\n"),
        ],
        limits(2),
    );
    let error = build_module_graph(&project, &CancellationToken::new())
        .expect_err("both targets are absent");
    assert_eq!(
        error
            .diagnostics()
            .iter()
            .map(ModuleGraphDiagnostic::module_id)
            .collect::<Vec<_>>(),
        ["problem::alpha", "problem::zeta"]
    );
    assert!(
        error
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.span().start() > 0
                && diagnostic.code() == diagnostics::MISSING_IMPORT)
    );
}

#[test]
/// Frozen forbidden-form cases retain their module-order oracle and fail closed.
fn forbidden_forms_follow_the_frozen_diagnostic_order() {
    let project = captured(
        &[
            ("forbidden::alpha", "import forbidden::* as all\n"),
            ("forbidden::beta", "import ::forbidden::target as target\n"),
            ("forbidden::gamma", "import forbidden::target\n"),
            (
                "forbidden::delta",
                "public import forbidden::target as target\n",
            ),
        ],
        limits(4),
    );
    let failure = build_module_graph(&project, &CancellationToken::new())
        .expect_err("forbidden forms must never publish a partial graph");
    assert_eq!(
        failure
            .diagnostics()
            .iter()
            .map(|diagnostic| (diagnostic.module_id(), diagnostic.code()))
            .collect::<Vec<_>>(),
        [
            ("forbidden::alpha", diagnostics::FORBIDDEN_IMPORT),
            ("forbidden::beta", diagnostics::FORBIDDEN_IMPORT),
            ("forbidden::delta", diagnostics::FORBIDDEN_IMPORT),
            ("forbidden::gamma", diagnostics::INVALID_SYNTAX),
        ]
    );
}

#[test]
/// Vocabulary and own-module aliases occupy the same local namespace as imports.
fn vocabulary_and_own_module_aliases_are_reserved() {
    for (source, vocabularies) in [
        (
            "use Ledger as shared\nimport alias::shared as shared\n",
            vec![vocabulary("Ledger")],
        ),
        ("import alias::shared as main\n", Vec::new()),
    ] {
        let project = captured_with_vocabulary(
            &[("alias::main", source), ("alias::shared", "")],
            limits(2),
            vocabularies,
        );
        assert_eq!(
            build_module_graph(&project, &CancellationToken::new())
                .expect_err("alias must collide")
                .diagnostics()[0]
                .code(),
            diagnostics::ALIAS_COLLISION
        );
    }
}

#[test]
/// The condensation order is dependency-first with lexical tie breaking.
fn dependency_order_is_stable_for_multiple_ready_components() {
    let project = captured(
        &[
            (
                "order::root",
                "import order::beta as beta\nimport order::alpha as alpha\n",
            ),
            ("order::beta", ""),
            ("order::alpha", ""),
        ],
        limits(3),
    );
    let graph = build_module_graph(&project, &CancellationToken::new())
        .expect("acyclic import graph must be valid");
    assert_eq!(
        graph
            .components()
            .iter()
            .map(|component| component.modules()[0].as_ref())
            .collect::<Vec<_>>(),
        ["order::alpha", "order::beta", "order::root"]
    );
    assert_eq!(
        graph
            .edges()
            .iter()
            .map(GraphEdge::target)
            .collect::<Vec<_>>(),
        ["order::alpha", "order::beta"]
    );
}

#[test]
/// The three independent graph limits pass exactly and reject one over.
fn graph_limits_are_exact() {
    let chain = [
        ("chain::a", "import chain::b as b\n"),
        ("chain::b", "import chain::c as c\n"),
        ("chain::c", ""),
    ];
    let mut bound = limits(3);
    bound.import_edges = 2;
    assert!(build_module_graph(&captured(&chain, bound), &CancellationToken::new()).is_ok());
    bound.import_edges = 1;
    assert_eq!(first_code(&chain, bound), diagnostics::LIMIT_EXCEEDED);

    let fan = [
        (
            "fan::main",
            "import fan::left as left\nimport fan::right as right\n",
        ),
        ("fan::left", ""),
        ("fan::right", ""),
    ];
    bound = limits(3);
    bound.imports_per_module = 2;
    assert!(build_module_graph(&captured(&fan, bound), &CancellationToken::new()).is_ok());
    bound.imports_per_module = 1;
    assert_eq!(first_code(&fan, bound), diagnostics::LIMIT_EXCEEDED);

    let cycle = [
        ("scc::a", "import scc::b as b\n"),
        ("scc::b", "import scc::c as c\n"),
        ("scc::c", "import scc::a as a\n"),
    ];
    bound = limits(3);
    bound.scc_units = 3;
    assert!(build_module_graph(&captured(&cycle, bound), &CancellationToken::new()).is_ok());
    bound.scc_units = 2;
    assert_eq!(first_code(&cycle, bound), diagnostics::LIMIT_EXCEEDED);
}

#[test]
/// Cancellation before graph work prevents publication.
fn cancellation_fails_closed() {
    let project = captured(&[("cancel::main", "")], limits(1));
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let failure =
        build_module_graph(&project, &cancellation).expect_err("cancelled work must fail");
    assert_eq!(failure.diagnostics()[0].code(), diagnostics::CANCELLED);
}

#[test]
/// Deep acyclic graphs use iterative traversal within the source-unit bound.
fn deep_graph_does_not_use_recursive_call_stack() {
    let count = 512_usize;
    let sources = (0..count)
        .map(|index| {
            let module = format!("deep::m{index}");
            let body = if index + 1 < count {
                format!("import deep::m{} as next\n", index + 1)
            } else {
                String::new()
            };
            (module, body)
        })
        .collect::<Vec<_>>();
    let borrowed = sources
        .iter()
        .map(|(module, body)| (module.as_str(), body.as_str()))
        .collect::<Vec<_>>();
    let mut bound = limits(count as u64);
    bound.total_source_bytes = 256_000;
    let project = captured(&borrowed, bound);
    let graph = build_module_graph(&project, &CancellationToken::new())
        .expect("bounded deep graph must succeed");
    assert_eq!(graph.components().len(), count);
}
