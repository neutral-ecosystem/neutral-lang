// SPDX-License-Identifier: Apache-2.0

//! Consumer selection, privacy and guarded demand over independently validated artifacts.
//!
//! Reuses companion fixtures to construct genuine reader authority, without a
//! compiler dependency. Literal symbols, paths and error classes below are
//! independent oracles; do not derive expected results from selection helpers.

use super::*;
use crate::consumer::{ENTRY_REQUEST_SCHEMA, EntryRequest, EntrySelection};
use crate::expression::{
    ExpressionEntryError, ExpressionInspectionLimits, ExpressionReadError, ExpressionViewLimits,
    ExpressionViewRequest, GuardCondition, ValidatedExpressionProject,
};
use neutral_ir::expression::{BinaryOperator, profile};
use neutral_semantics::BindingKey;

#[path = "boundary_coverage.rs"]
mod boundary_coverage;

/// Recomputes fixture companions and crosses the real complete-reader boundary.
fn authority(ir: ExpressionProjectIr) -> ValidatedExpressionProject {
    ValidatedExpressionProject::from_ir(complete(ir), limits(), &CancellationToken::new()).unwrap()
}

#[test]
/// Requires one exact nominal entry and its complete public interpretation closure.
fn entry_discovery_selects_exact_public_nominals_and_closes_the_view() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let ty = T::Nominal(symbol("m", "Record"));
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &ty,
        selection: EntrySelection::Unique,
    };
    let cancel = CancellationToken::new();
    let selected = project
        .entries(
            &request,
            &project.ir().semantic_profiles,
            ExpressionViewLimits::default(),
            &cancel,
        )
        .unwrap();
    assert_eq!(selected.bindings().len(), 1);
    assert_eq!(selected.bindings()[0].identity, symbol("m", "a"));
    let view = selected.derive_view(&cancel).unwrap();
    assert_eq!(view.roots().collect::<Vec<_>>(), [&symbol("m", "a")]);
    assert_eq!(
        view.declarations()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["Record", "a"]
    );
    assert_eq!(view.origins().count(), 2);
    assert_eq!(view.structural_edges().count(), 1);
    assert_eq!(view.templates().count(), 0);
    assert_eq!(view.vocabularies().count(), 0);
    assert_eq!(view.semantic_profiles(), &project.ir().semantic_profiles);
}

#[test]
/// Rejects malformed selection requests without returning partial binding authority.
fn entry_requests_reject_wrong_types_duplicate_roots_and_unknown_schema() {
    let mut ir = default_fixture(loc("m", 20, 21));
    scalar(&mut ir, "m", "number", 80);
    let project = authority(ir);
    let ty = T::Nominal(symbol("m", "Record"));
    let cancel = CancellationToken::new();
    for (schema, contract, roots, expected) in [
        (
            ENTRY_REQUEST_SCHEMA,
            &ty,
            vec![symbol("m", "number")],
            ExpressionEntryError::TypeMismatch,
        ),
        (
            ENTRY_REQUEST_SCHEMA,
            &ty,
            vec![symbol("m", "a"), symbol("m", "a")],
            ExpressionEntryError::InvalidInput,
        ),
        (
            ENTRY_REQUEST_SCHEMA,
            &ty,
            vec![symbol("m", "missing")],
            ExpressionEntryError::InvalidInput,
        ),
        (
            ENTRY_REQUEST_SCHEMA,
            &T::Num,
            vec![],
            ExpressionEntryError::InvalidInput,
        ),
        ("unknown", &ty, vec![], ExpressionEntryError::Unsupported),
    ] {
        let result = project.entries(
            &EntryRequest {
                schema,
                contract,
                selection: EntrySelection::Explicit(&roots),
            },
            &project.ir().semantic_profiles,
            ExpressionViewLimits::default(),
            &cancel,
        );
        assert_eq!(result.unwrap_err(), expected);
    }
}

#[test]
/// Keeps explicit empty roots empty and prevents access to private bindings.
fn empty_view_is_empty_and_private_roots_never_escape() {
    let mut ir = empty();
    scalar(&mut ir, "m", "hidden", 10);
    ir.declarations[0].public = false;
    scalar(&mut ir, "m", "visible", 30);
    let project = authority(ir);
    let cancel = CancellationToken::new();
    let request = ExpressionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA,
        semantic_profiles: &project.ir().semantic_profiles,
        roots: &[],
    };
    let view = project.derive_view(&request, &cancel).unwrap();
    assert_eq!(view.roots().count(), 0);
    assert_eq!(view.declarations().count(), 0);
    assert_eq!(view.origins().count(), 0);
    let hidden = [symbol("m", "hidden")];
    assert_eq!(
        project
            .derive_view(
                &ExpressionViewRequest {
                    roots: &hidden,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ExpressionReadError::Semantic
    );
    assert_eq!(
        view.dependencies(&hidden[0], ExpressionInspectionLimits::default(), &cancel)
            .unwrap_err(),
        ExpressionReadError::Semantic
    );
}

/// Builds two real external Boolean ports and one retained short-circuit expression.
fn guarded_project(operator: BinaryOperator) -> ValidatedExpressionProject {
    authority(guarded_fixture(operator))
}

/// Retains a complete Boolean fixture before crossing the reader authority boundary.
fn guarded_fixture(operator: BinaryOperator) -> ExpressionProjectIr {
    let mut ir = empty();
    for (name, start) in [("enabled", 10), ("other", 30)] {
        ir.declarations.push(ExpressionDeclaration {
            identity: symbol("m", name),
            public: true,
            signature: S::Binding(T::Bool),
            body: Some(ExpressionBody::Input),
        });
        ir.source_maps.push(ProjectSourceMap {
            declaration: symbol("m", name),
            location: loc("m", start, start + 10),
        });
    }
    ir.declarations.push(ExpressionDeclaration {
        identity: symbol("m", "result"),
        public: true,
        signature: S::Binding(T::Bool),
        body: Some(ExpressionBody::Expression(Expression {
            ty: T::Bool,
            node: ExpressionNode::Binary {
                operator,
                left: Box::new(Expression {
                    ty: T::Bool,
                    node: ExpressionNode::Input(symbol("m", "enabled")),
                }),
                right: Box::new(Expression {
                    ty: T::Bool,
                    node: ExpressionNode::Input(symbol("m", "other")),
                }),
            },
        })),
    });
    ir.source_maps.push(ProjectSourceMap {
        declaration: symbol("m", "result"),
        location: loc("m", 50, 70),
    });
    ir
}

#[test]
/// Retains each potential port with its original guard site instead of evaluating it.
fn boolean_dependencies_keep_guard_paths_without_evaluating_inputs() {
    for (operator, condition) in [(BinaryOperator::And, true), (BinaryOperator::Or, false)] {
        let project = guarded_project(operator);
        let owner = symbol("m", "result");
        let cancel = CancellationToken::new();
        let edges = project
            .dependencies(&owner, ExpressionInspectionLimits::default(), &cancel)
            .unwrap();
        assert_eq!(edges.edges().len(), 2);
        assert_eq!(
            edges.edges()[0].key,
            BindingKey::Input(&symbol("m", "enabled"))
        );
        assert_eq!(edges.edges()[0].node_path, [0]);
        assert_eq!(edges.edges()[0].guards.len(), 0);
        assert_eq!(
            edges.edges()[1].key,
            BindingKey::Input(&symbol("m", "other"))
        );
        assert_eq!(edges.edges()[1].node_path, [1]);
        assert_eq!(edges.edges()[1].guards[0].node_path, [0]);
        assert_eq!(
            edges.edges()[1].guards[0].condition,
            GuardCondition::Boolean(condition)
        );
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
        assert_eq!(view.declarations().count(), 3);
        assert_eq!(
            view.dependencies(&owner, ExpressionInspectionLimits::default(), &cancel)
                .unwrap(),
            edges
        );
    }
}

#[test]
/// Enforces independent inspection limits and permits recovery after cancellation.
fn dependency_limits_and_cancellation_fail_atomically_then_recover() {
    let project = guarded_project(BinaryOperator::And);
    let owner = symbol("m", "result");
    let cancel = CancellationToken::new();
    for controls in [
        ExpressionInspectionLimits {
            edges: 1,
            ..Default::default()
        },
        ExpressionInspectionLimits {
            work: 0,
            ..Default::default()
        },
        ExpressionInspectionLimits {
            output_bytes: 0,
            ..Default::default()
        },
    ] {
        assert_eq!(
            project.dependencies(&owner, controls, &cancel).unwrap_err(),
            ExpressionReadError::Limit
        );
    }
    let exact = ExpressionInspectionLimits {
        edges: 2,
        ..Default::default()
    };
    assert_eq!(
        project
            .dependencies(&owner, exact, &cancel)
            .unwrap()
            .edges()
            .len(),
        2
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        project.dependencies(&owner, exact, &cancelled).unwrap_err(),
        ExpressionReadError::Cancelled
    );
    assert_eq!(
        project
            .dependencies(&owner, exact, &cancel)
            .unwrap()
            .edges()
            .len(),
        2
    );
}

#[test]
/// Distinguishes enumeration, ambiguous uniqueness and missing uniqueness.
fn entry_cardinality_distinguishes_empty_unique_and_ambiguous_matches() {
    let mut ir = default_fixture(loc("m", 20, 21));
    let owner = symbol("m", "b");
    ir.declarations.push(ExpressionDeclaration {
        identity: owner.clone(),
        public: true,
        signature: S::Binding(T::Nominal(symbol("m", "Record"))),
        body: Some(ExpressionBody::Input),
    });
    ir.source_maps.push(ProjectSourceMap {
        declaration: owner.clone(),
        location: loc("m", 80, 90),
    });
    ir.provenance.push(ProjectProvenance {
        from: owner,
        to: symbol("m", "Record"),
        kind: K::Type,
        location: loc("m", 81, 82),
    });
    let project = authority(ir);
    let ty = T::Nominal(symbol("m", "Record"));
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &ty,
        selection: EntrySelection::Enumerate,
    };
    let cancel = CancellationToken::new();
    let select = |request: &EntryRequest<'_>| {
        project.entries(
            request,
            &project.ir().semantic_profiles,
            ExpressionViewLimits::default(),
            &cancel,
        )
    };
    assert_eq!(
        select(&request)
            .unwrap()
            .bindings()
            .iter()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(
        select(&EntryRequest {
            selection: EntrySelection::Unique,
            ..request
        })
        .unwrap_err(),
        ExpressionEntryError::AmbiguousEntry
    );
    assert_eq!(
        select(&EntryRequest {
            selection: EntrySelection::Explicit(&[]),
            ..request
        })
        .unwrap()
        .bindings()
        .len(),
        0
    );
    let mut absent = default_fixture(loc("m", 20, 21));
    absent
        .declarations
        .retain(|d| d.identity.declaration_name() != "a");
    absent
        .source_maps
        .retain(|m| m.declaration.declaration_name() != "a");
    absent.origins.clear();
    absent.provenance.clear();
    let absent = authority(absent);
    assert_eq!(
        absent
            .entries(
                &EntryRequest {
                    selection: EntrySelection::Unique,
                    ..request
                },
                &absent.ir().semantic_profiles,
                ExpressionViewLimits::default(),
                &cancel
            )
            .unwrap_err(),
        ExpressionEntryError::MissingEntry
    );
}

#[test]
/// Sweeps every projection reservation while preserving previously accepted authority.
fn public_view_allocation_failures_preserve_the_complete_project() {
    use neutral_core::allocation::testing;
    let project = guarded_project(BinaryOperator::And);
    let roots = [symbol("m", "result")];
    let request = ExpressionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA,
        semantic_profiles: &project.ir().semantic_profiles,
        roots: &roots,
    };
    let cancel = CancellationToken::new();
    let (accepted, count) = testing::observe(None, || project.derive_view(&request, &cancel));
    let accepted = accepted.unwrap();
    assert_ne!(count, 0);
    for index in 0..count {
        let (failed, _) = testing::observe(Some(index), || project.derive_view(&request, &cancel));
        assert_eq!(
            failed.unwrap_err(),
            ExpressionReadError::Limit,
            "reservation {index}"
        );
        let signal = CancellationToken::new();
        let (failed, _) = testing::observe_cancellation(index, &signal, || {
            project.derive_view(&request, &signal)
        });
        assert_eq!(
            failed.unwrap_err(),
            ExpressionReadError::Cancelled,
            "reservation {index}"
        );
        assert_eq!(accepted.declarations().count(), 3);
        assert_eq!(
            project
                .derive_view(&request, &cancel)
                .unwrap()
                .declarations()
                .count(),
            3
        );
    }
}
