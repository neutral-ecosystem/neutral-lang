// SPDX-License-Identifier: Apache-2.0

//! Literal guarded occurrences and redacted interpretive closure after independent decoding.

use super::*;
use GuardCondition::{Alternative as A, Boolean as B};
use neutral_ir::{
    composition::ValueOriginKind, expression::SelectorSegment,
    project_interface::ProjectPublicType as T,
};
use neutral_reader::expression::ExpressionReadError;
use neutral_semantics::{AvailabilityRequirement, BindingKey, ExpressionRoot};

/// Literal enclosing conditions in outer-to-inner order, independent of production traversal.
type GuardChain<'a> = &'a [(&'a [u64], GuardCondition<'a>)];

/// Nested guards preserve every occurrence even when four leaves share the same exact output key.
#[test]
fn integration_expression_nested_guard_oracles_preserve_repeated_output_sites() {
    let (_, project) = pipeline(include_bytes!("fixtures/nested-guards.neu"));
    let owner = symbol("decision");
    let cancel = CancellationToken::new();
    let dependencies = project
        .dependencies(&owner, ExpressionInspectionLimits::default(), &cancel)
        .unwrap();
    let expected: &[(&[u64], GuardChain<'_>)] = &[
        (&[0], &[]),
        (&[1, 0], &[(&[0], B(true))]),
        (&[1, 1], &[(&[0], B(true)), (&[1, 0], A("failed"))]),
        (&[1, 2, 0], &[(&[0], B(true)), (&[1, 0], A("ready"))]),
        (
            &[1, 2, 1, 0],
            &[
                (&[0], B(true)),
                (&[1, 0], A("ready")),
                (&[1, 2, 0], B(true)),
            ],
        ),
        (
            &[1, 2, 1, 1],
            &[
                (&[0], B(true)),
                (&[1, 0], A("ready")),
                (&[1, 2, 0], B(true)),
                (&[1, 2, 1, 0], B(false)),
            ],
        ),
        (&[2], &[(&[0], B(false))]),
    ];
    assert_eq!(dependencies.edges().len(), expected.len());
    let selector = [SelectorSegment::Field("flag".into())];
    let producer = symbol("producer");
    let result = T::Nominal(symbol("Result"));
    for (index, (edge, &(path, guards))) in dependencies.edges().iter().zip(expected).enumerate() {
        assert_eq!(edge.owner, &owner);
        assert_eq!(edge.root, ExpressionRoot::Binding);
        assert_eq!(edge.node_path, path, "edge {index}");
        assert_eq!(
            edge.guards
                .iter()
                .map(|g| (g.node_path.as_slice(), g.condition))
                .collect::<Vec<_>>(),
            guards,
            "edge {index}"
        );
        let input = match index {
            0 => Some(symbol("enabled")),
            1 => Some(symbol("choice")),
            3 => Some(symbol("other")),
            _ => None,
        };
        if let Some(input) = input {
            assert_eq!(edge.key, BindingKey::Input(&input));
        } else {
            assert_eq!(
                edge.key,
                BindingKey::Output {
                    target: &producer,
                    result_type: &result,
                    selector: &selector
                }
            );
        }
    }
    let roots = [owner.clone()];
    let view = project
        .derive_view(
            &ExpressionViewRequest {
                schema: profile::PROJECT_VIEW_SCHEMA,
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(
        view.dependencies(&owner, ExpressionInspectionLimits::default(), &cancel)
            .unwrap(),
        dependencies
    );
    assert_eq!(
        view.declarations()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        [
            "Mode", "Result", "choice", "decision", "enabled", "other", "producer"
        ]
    );
}

/// Selects a public export and residual entry from a source-produced, independently decoded artifact.
fn projection() -> (
    ValidatedExpressionProject,
    neutral_reader::expression::ExpressionView,
) {
    let project = super::consumer::vocabulary_project(
        include_bytes!("fixtures/projection.neu"),
        include_bytes!("fixtures/projection-domain.json"),
    );
    let roots = [symbol("exposed"), symbol("entry")];
    let cancel = CancellationToken::new();
    let view = project
        .derive_view(
            &ExpressionViewRequest {
                schema: profile::PROJECT_VIEW_SCHEMA,
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            &cancel,
        )
        .unwrap();
    (project, view)
}

/// Optional fields, future indices/alternatives and restrictions keep exact contracts after decode.
#[test]
fn integration_expression_decoded_projection_keeps_every_future_obligation() {
    let (_, view) = projection();
    assert_eq!(
        view.declarations()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["enabled", "entry", "exposed", "producer"]
    );
    let vocabulary = view.vocabularies().next().unwrap();
    assert_eq!(vocabulary.identity(), "PipelineDomain");
    assert_eq!(vocabulary.version(), "1.0.0");
    assert_eq!(
        vocabulary
            .definitions()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>(),
        ["Operation", "Plan", "Result", "State"]
    );
    let availability = view.availability_obligations().collect::<Vec<_>>();
    assert_eq!(availability.len(), 3);
    for (obligation, (segment, requirement)) in availability.iter().zip([
        (0, AvailabilityRequirement::Presence),
        (1, AvailabilityRequirement::Index),
        (2, AvailabilityRequirement::Alternative),
    ]) {
        assert_eq!(obligation.site.owner, symbol("entry"));
        assert_eq!(obligation.site.root, ExpressionRoot::Binding);
        assert_eq!(obligation.site.node_path, [0, 1]);
        assert_eq!(obligation.segment, segment);
        assert_eq!(obligation.requirement, requirement);
    }
    let restrictions = view.restriction_obligations().collect::<Vec<_>>();
    assert_eq!(restrictions.len(), 1);
    assert_eq!(restrictions[0].site.node_path, [0]);
    assert_eq!(
        restrictions[0].contract,
        T::VocabularyNominal {
            identity: "PipelineDomain".into(),
            version: "1.0.0".into(),
            name: "Plan".into()
        }
    );
    assert_eq!(restrictions[0].field, "value");
    let plan = vocabulary.definitions().find(|d| d.name == "Plan").unwrap();
    let neutral_ir::composition::CompositionBody::Record(fields) = &plan.body else {
        panic!("Plan must remain a record")
    };
    let field = &fields[0];
    assert_eq!(field.name, "value");
    assert_eq!(field.ty, T::Num);
    assert_eq!(
        field.restrictions.minimum.as_ref().unwrap(),
        &ExactNumber::from_source("0", 128, 128).unwrap()
    );
    assert_eq!(
        field.restrictions.maximum.as_ref().unwrap(),
        &ExactNumber::from_source("3", 128, 128).unwrap()
    );
}

/// Public values survive private reuse, but complete private occurrences are not projected.
#[test]
fn integration_expression_decoded_projection_redacts_private_occurrences() {
    let (project, view) = projection();
    let cancel = CancellationToken::new();
    assert!(
        project
            .ir()
            .origins
            .iter()
            .any(|o| o.binding == symbol("private_seed"))
    );
    let public = ["enabled", "entry", "exposed", "producer"];
    assert_eq!(
        view.origins()
            .map(|o| (o.owner.declaration_name(), o.path, o.kind))
            .collect::<Vec<_>>(),
        [
            ("exposed", [].as_slice(), ValueOriginKind::Supplied),
            ("producer", [].as_slice(), ValueOriginKind::Supplied)
        ]
    );
    assert_eq!(view.expression_attributions().count(), 1);
    assert!(
        view.expression_attributions()
            .all(|a| public.contains(&a.owner.declaration_name()))
    );
    assert!(
        view.structural_edges()
            .all(|e| public.contains(&e.from.declaration_name())
                && public.contains(&e.to.declaration_name()))
    );
    assert_eq!(
        value(&project, "exposed"),
        &V::Number(ExactNumber::from_source("0.125", 128, 128).unwrap())
    );
    assert_eq!(
        view.dependencies(
            &symbol("exposed"),
            ExpressionInspectionLimits::default(),
            &cancel
        )
        .unwrap()
        .edges(),
        []
    );
    assert_eq!(
        view.dependencies(
            &symbol("private_seed"),
            ExpressionInspectionLimits::default(),
            &cancel
        )
        .unwrap_err(),
        ExpressionReadError::Semantic
    );
}
