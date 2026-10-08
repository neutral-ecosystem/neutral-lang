// SPDX-License-Identifier: Apache-2.0

//! Generated transitive graph families supplement frozen literals without rewriting reviewed oracles.

use super::*;
use neutral_core::VocabularyContentDigest;
use neutral_probe::composition::inspect_composition_encoded;
use neutral_vocabulary::composition::CompositionError;

/// Builds source import graphs whose nominal reference contracts permit valid semantic cycles.
fn source_graph(edges: &[Vec<usize>]) -> Value {
    let mut case = source_case("");
    case["capture"]["sources"] = json!(edges.iter().enumerate().map(|(node, targets)| {
        let mut source = format!("neu \"1.0\"\nmodule node{node}\n");
        for target in targets { writeln!(source, "import node{target} as edge{target}").unwrap(); }
        source.push_str("public record Item {\n");
        for target in targets { writeln!(source, "Ref<edge{target}::Item>? next{target} = null,").unwrap(); }
        source.push_str("}\npublic Item root = {}\n");
        json!({"module":format!("node{node}"), "source_id":format!("source:node{node}"), "source_utf8":source})
    }).collect::<Vec<_>>());
    case
}

/// Successor topology matches the existing independent graph projection and survives complete wire validation.
#[test]
fn integration_composition_source_graphs_preserve_cycles_and_canonical_order() {
    for edges in [
        vec![vec![1], vec![2], vec![]],
        vec![vec![1, 2], vec![3], vec![3], vec![]],
        vec![vec![1], vec![2], vec![0], vec![]],
    ] {
        let mut case = source_graph(&edges);
        let captured = capture_composition_project(request(&case)).unwrap();
        let expected = captured.module_graph(&CancellationToken::new()).unwrap();
        let project = compile(&case);
        let ir = project.complete_ir();
        assert_eq!(ir.modules.len(), expected.modules().len());
        assert_eq!(
            usize::try_from(ir.resources.import_edges).unwrap(),
            expected.edges().len()
        );
        for module in &ir.modules {
            let targets = expected
                .edges()
                .iter()
                .filter(|e| e.from() == module.identity.module_name())
                .map(neutral_compiler::GraphEdge::target)
                .collect::<Vec<_>>();
            assert_eq!(module.imports, targets);
        }
        let wire = round_trip(&case, 0);
        case["capture"]["sources"].as_array_mut().unwrap().reverse();
        assert_eq!(round_trip(&case, 0), wire);
    }
}

/// Syntax-first precedence and diagnostic overflow agree with the independent existing graph boundary.
#[test]
fn security_composition_source_graph_diagnostic_precedence_and_overflow_are_preserved() {
    let mut syntax_first = source_graph(&[vec![], vec![]]);
    syntax_first["capture"]["sources"][0]["source_utf8"] =
        json!("neu \"1.0\"\nmodule node0\nimport missing as absent\npublic num root = 1\n");
    syntax_first["capture"]["sources"][1]["source_utf8"] =
        json!("neu \"1.0\"\nmodule node1\npublic import node0 as dep\npublic num root = 1\n");
    let mut overflow = source_graph(&[vec![], vec![]]);
    for node in 0..2 {
        let mut source = format!("neu \"1.0\"\nmodule node{node}\n");
        for missing in 0..36 {
            writeln!(source, "import missing{missing} as absent{missing}").unwrap();
        }
        source.push_str("public num root = 1\n");
        overflow["capture"]["sources"][node]["source_utf8"] = json!(source);
    }
    for case in [syntax_first, overflow] {
        let captured = capture_composition_project(request(&case)).unwrap();
        let expected = captured
            .module_graph(&CancellationToken::new())
            .unwrap_err();
        let expected = &expected.diagnostics()[0];
        let actual = compile_composition_project(&captured, &CancellationToken::new()).unwrap_err();
        assert_eq!(actual.location, expected.source_location());
        assert_eq!(
            actual.code,
            if expected.code() == neutral_compiler::module_graph_diagnostics::LIMIT_EXCEEDED {
                neutral_compiler::composition_diagnostics::LIMIT
            } else {
                neutral_compiler::composition_diagnostics::INVALID_SOURCE
            }
        );
    }
}

/// Rebuilds exact runtime lock evidence after an intentional bundle mutation.
pub(super) fn replace_bundle(input: &mut Value, bundle: &Value) {
    let text = serde_json::to_string_pretty(bundle).unwrap();
    let digest = VocabularyContentDigest::from_bytes(text.as_bytes()).to_string();
    input["bundle_utf8"] = json!(text);
    input["digest"] = json!(digest.strip_prefix("sha256:").unwrap());
    input["byte_len"] = json!(text.len());
}

/// Creates exact data-only nominal dependency graphs from the reviewed envelope and revision.
fn graph(edges: &[Vec<usize>]) -> Value {
    let registered: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let reviewed = registered["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "exact-diamond-closure")
        .unwrap();
    let template = &reviewed["capture"]["vocabularies"][0];
    let prototype: Value = serde_json::from_str(template["bundle_utf8"].as_str().unwrap()).unwrap();
    let revision = prototype["version"].clone();
    let mut inputs = Vec::new();
    for (index, targets) in edges.iter().enumerate() {
        let mut bundle = prototype.clone();
        bundle["identity"] = json!(format!("Node{index}"));
        bundle["dependencies"] = json!(
            targets
                .iter()
                .map(|target| json!({"identity":format!("Node{target}"), "version":revision}))
                .collect::<Vec<_>>()
        );
        bundle["types"] = json!([{"kind":"record", "name":"Item", "public":true,
            "fields": targets.iter().map(|target| json!({"name":format!("edge{target}"),
                "presence":"optional", "restrictions":{},
                "type":{"kind":"ref", "target":{"kind":"external",
                    "identity":format!("Node{target}"),"version":revision,"name":"Item"}}})).collect::<Vec<_>>() }]);
        let mut input = template.clone();
        input["identity"] = bundle["identity"].clone();
        replace_bundle(&mut input, &bundle);
        inputs.push(input);
    }
    let mut case = source_case(
        "neu \"1.0\"\nmodule example\nuse Node0 as domain\npublic domain::Item root = {}\n",
    );
    case["capture"]["vocabularies"] = json!(inputs);
    case
}

/// Inspects complete wire output and public closure independently for every required graph owner.
fn round_trip(case: &Value, expected_bundles: usize) -> Vec<u8> {
    let project = compile(case);
    let cancel = CancellationToken::new();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    let summary = inspect_composition_encoded(
        &bytes,
        DecodeLimits::hard(),
        project.complete_ir().limits,
        limits_for(case),
        None,
        &cancel,
    )
    .unwrap();
    assert_eq!(summary.view.vocabularies().len(), expected_bundles);
    assert!(
        summary
            .view
            .vocabularies()
            .iter()
            .all(|b| b.definitions.len() == 1)
    );
    bytes
}

/// Chains, wide fanout, diamonds and unequal reconvergent paths retain all unselected reference contracts.
#[test]
fn integration_composition_transitive_graph_families_are_order_and_schedule_invariant() {
    let families = [
        vec![vec![1], vec![2], vec![3], vec![]],
        vec![
            vec![1, 2, 3, 4, 5, 6, 7],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        vec![vec![1, 2], vec![3], vec![3], vec![]],
        vec![vec![1, 2], vec![4], vec![3], vec![4], vec![]],
    ];
    for edges in families {
        let mut case = graph(&edges);
        let bytes = round_trip(&case, edges.len());
        case["capture"]["vocabularies"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert_eq!(round_trip(&case, edges.len()), bytes);
        std::thread::scope(|scope| {
            let count = edges.len();
            for _ in 0..3 {
                let case = &case;
                let bytes = &bytes;
                scope.spawn(move || assert_eq!(&round_trip(case, count), bytes));
            }
        });
    }
}

/// Longest dependency path and independent edge/fanout/bundle limits are enforced, including diamond memo hits.
#[test]
fn security_composition_transitive_graph_bounds_are_exact_and_independent() {
    let case = graph(&[vec![1, 2], vec![4], vec![3], vec![4], vec![]]);
    let original = limits_for(&case);
    for (control, exact) in [(0, 5), (1, 5), (2, 2), (3, 4)] {
        let accepts = |bound| {
            let mut limits = original;
            match control {
                0 => limits.bundles = bound,
                1 => limits.dependency_edges = bound,
                2 => limits.dependencies_per_bundle = bound,
                _ => limits.dependency_depth = bound,
            }
            capture_composition_project(request_with_limits(&case, limits)).is_ok()
        };
        assert!(accepts(exact), "control {control}");
        assert!(!accepts(exact - 1), "control {control}");
        assert!(accepts(exact + 1));
    }
}

/// Missing/extra locks, divergent revisions, private targets and cycles never produce a captured project.
#[test]
fn security_composition_transitive_graph_mutations_fail_before_publication() {
    let original = graph(&[vec![1, 2], vec![3], vec![3], vec![]]);
    let mut missing = original.clone();
    missing["capture"]["vocabularies"]
        .as_array_mut()
        .unwrap()
        .pop();
    let mut extra = graph(&[vec![1], vec![], vec![]]);
    extra["id"] = json!("extra-unreachable");
    for (case, expected) in [
        (missing, CompositionError::MissingDependency),
        (extra, CompositionError::ExtraBundle),
    ] {
        assert_eq!(
            capture_composition_project(request(&case))
                .unwrap_err()
                .code(),
            expected.diagnostic_code()
        );
    }
    for (index, mutation, expected) in [
        (2, 0, CompositionError::MissingDependency),
        (3, 1, CompositionError::PrivateType),
        (3, 2, CompositionError::DependencyCycle),
    ] {
        let mut case = original.clone();
        let input = &mut case["capture"]["vocabularies"][index];
        let mut bundle: Value =
            serde_json::from_str(input["bundle_utf8"].as_str().unwrap()).unwrap();
        match mutation {
            0 => {
                bundle["dependencies"][0]["version"] = json!("9.9.9");
            }
            1 => {
                bundle["types"][0]["public"] = json!(false);
            }
            _ => {
                bundle["dependencies"] = json!([{"identity":"Node0", "version":bundle["version"]}]);
            }
        }
        replace_bundle(input, &bundle);
        assert_eq!(
            capture_composition_project(request(&case))
                .unwrap_err()
                .code(),
            expected.diagnostic_code()
        );
    }
    assert_eq!(round_trip(&original, 4), round_trip(&original, 4));
}
