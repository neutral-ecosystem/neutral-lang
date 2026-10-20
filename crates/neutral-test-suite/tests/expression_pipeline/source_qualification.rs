// SPDX-License-Identifier: Apache-2.0

//! Source-produced lineage is independently checked before public redaction.

use super::*;
use neutral_ir::project_interface::ProjectPublicEdgeKind as K;

/// Explicit successor syntax never widens either historical contextual parser.
#[test]
fn integration_expression_syntax_is_rejected_by_historical_pipelines() {
    use neutral_compiler::{
        CapturedCompositionProjectRequest, capture_composition_project, capture_project,
        compile_composition_project, compile_project,
    };
    use neutral_ir::composition::profile as old;

    for body in [
        "public input num supplied",
        "public num result = 1 + 2",
        "public num result = if true { 1 } else { 2 }",
        "template Item(static num value) -> num { value }\npublic num result = instantiate Item(value: 2)",
        "template Item(static num value) -> num { value }\nexpand items = Item(value: axis) for (axis in [1, 2])",
    ] {
        let source = format!("neu \"1.0\"\nmodule pipeline\n{body}\n");
        let sources = || {
            vec![CapturedSourceInput::new(
                "source:pipeline",
                "pipeline",
                source.as_bytes().to_vec(),
            )]
        };
        if let Ok(captured) = capture_project(capture_envelope(
            sources(),
            vec![],
            neutral_compiler::CAPTURE_REQUEST_VERSION,
            CancellationToken::new(),
        )) {
            assert!(
                compile_project(&captured, &CancellationToken::new()).is_err(),
                "{body}"
            );
        }
        let request = CapturedCompositionProjectRequest::new(
            capture_envelope(
                sources(),
                vec![],
                old::CAPTURE_REQUEST_VERSION,
                CancellationToken::new(),
            ),
            old::REQUIRED_FEATURES
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            composition_limits(),
        );
        if let Ok(captured) = capture_composition_project(request) {
            assert!(
                compile_composition_project(&captured, &CancellationToken::new()).is_err(),
                "{body}"
            );
        }
    }
}

/// Materializing an immutable alias must not erase its checked source dependency.
#[test]
fn integration_source_reuse_survives_wire_without_exposing_private_origins() {
    let source = include_str!("fixtures/reuse.neu");
    let (_, project) = pipeline(source.as_bytes());
    let exposed = symbol("exposed");
    let seed = symbol("private_seed");
    let edges = project
        .ir()
        .provenance
        .iter()
        .filter(|e| e.from == exposed && e.kind == K::Value)
        .collect::<Vec<_>>();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].to, symbol("private_alias"));
    for (name, count) in [("copies", 2), ("container", 1), ("choice", 1)] {
        let owner = symbol(name);
        let edges: Vec<_> = project
            .ir()
            .provenance
            .iter()
            .filter(|e| e.from == owner && e.kind == K::Value)
            .collect();
        assert_eq!(edges.len(), count, "{name}");
        for edge in edges {
            assert_eq!(edge.to, seed);
            assert_eq!(
                &source[usize::try_from(edge.location.span().start()).unwrap()
                    ..usize::try_from(edge.location.span().end()).unwrap()],
                "private_seed"
            );
        }
    }
    for name in ["computed", "selected"] {
        assert!(
            project
                .ir()
                .provenance
                .iter()
                .all(|e| e.from != symbol(name) || e.kind != K::Value)
        );
    }
    let roots = [exposed.clone()];
    let view = project
        .derive_view(
            &ExpressionViewRequest {
                schema: profile::PROJECT_VIEW_SCHEMA,
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            &CancellationToken::new(),
        )
        .unwrap();
    for private in [seed, symbol("private_alias")] {
        assert!(view.declarations().all(|d| d.identity != private));
        assert!(view.structural_edges().all(|e| e.to != &private));
        assert!(view.expression_attributions().all(|a| a.owner != &private));
    }
}
