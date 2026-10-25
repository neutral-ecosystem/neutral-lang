// SPDX-License-Identifier: Apache-2.0

//! Source-produced lineage is independently checked before public redaction.

use super::*;
use neutral_ir::project_interface::ProjectPublicEdgeKind as K;

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
