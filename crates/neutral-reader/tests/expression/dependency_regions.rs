// SPDX-License-Identifier: Apache-2.0

//! Dependency paths preserve typed node positions, branch guards and atomic resource failures.

use super::*;
use neutral_core::allocation::testing::{observe, observe_cancellation};

/// One exact typed external leaf shared by the raw test trees.
fn input(name: &str) -> Expression {
    Expression {
        ty: T::Bool,
        node: ExpressionNode::Input(symbol("m", name)),
    }
}

/// Creates one independently checked tree without invoking the compiler or evaluating a guard.
fn tree(kind: usize) -> Expression {
    let (ty, node) = match kind {
        0 => (
            T::Bool,
            ExpressionNode::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(input("enabled")),
            },
        ),
        1 => (T::String, ExpressionNode::Text(Box::new(input("enabled")))),
        2 => (
            T::Bool,
            ExpressionNode::Binary {
                operator: BinaryOperator::Equal,
                left: Box::new(input("enabled")),
                right: Box::new(input("other")),
            },
        ),
        3 => (
            T::Bool,
            ExpressionNode::If {
                guard: Box::new(input("enabled")),
                then_branch: Box::new(input("other")),
                else_branch: Box::new(input("enabled")),
            },
        ),
        4 => (
            T::List(Box::new(T::Bool)),
            ExpressionNode::List(vec![input("enabled"), input("other")]),
        ),
        5 => (
            T::String,
            ExpressionNode::Interpolate(vec![Expression {
                ty: T::String,
                node: ExpressionNode::Text(Box::new(input("enabled"))),
            }]),
        ),
        6 => (
            T::Nominal(symbol("m", "Row")),
            ExpressionNode::Record(vec![("flag".into(), Some(input("enabled")))]),
        ),
        7 => (
            T::Nominal(symbol("m", "Choice")),
            ExpressionNode::Variant {
                tag: "ready".into(),
                payload: Box::new(input("enabled")),
            },
        ),
        _ => (
            T::Bool,
            ExpressionNode::Match {
                scrutinee: Box::new(Expression {
                    ty: T::Nominal(symbol("m", "Choice")),
                    node: ExpressionNode::Variant {
                        tag: "ready".into(),
                        payload: Box::new(input("enabled")),
                    },
                }),
                arms: vec![
                    MatchArm {
                        tag: "failed".into(),
                        payload_type: T::Bool,
                        body: input("other"),
                    },
                    MatchArm {
                        tag: "ready".into(),
                        payload_type: T::Bool,
                        body: input("enabled"),
                    },
                ],
            },
        ),
    };
    Expression { ty, node }
}

/// Adds the exact nominal contracts and original companion evidence needed by each tree.
fn project(kind: usize) -> ValidatedExpressionProject {
    let mut ir = guarded_fixture(BinaryOperator::And);
    let tree = tree(kind);
    let result = ir.declarations.last_mut().unwrap();
    result.signature = S::Binding(tree.ty.clone());
    result.body = Some(ExpressionBody::Expression(tree));
    let definition = match kind {
        6 => Some((
            "Row",
            CompositionBody::Record(vec![CompositionField {
                name: "flag".into(),
                ty: T::Bool,
                presence: FieldPresence::Required,
                restrictions: FieldRestrictions::default(),
                default: None,
            }]),
        )),
        7 | 8 => Some((
            "Choice",
            CompositionBody::Variant(vec![
                neutral_ir::composition::CompositionAlternative {
                    tag: "failed".into(),
                    ty: T::Bool,
                },
                neutral_ir::composition::CompositionAlternative {
                    tag: "ready".into(),
                    ty: T::Bool,
                },
            ]),
        )),
        _ => None,
    };
    if let Some((name, body)) = definition {
        let owner = symbol("m", name);
        ir.declarations.push(ExpressionDeclaration {
            identity: owner.clone(),
            public: true,
            signature: S::Definition(body),
            body: None,
        });
        ir.source_maps.push(ProjectSourceMap {
            declaration: owner.clone(),
            location: loc("m", 80, 100),
        });
        ir.provenance.push(ProjectProvenance {
            from: symbol("m", "result"),
            to: owner,
            kind: K::Type,
            location: loc("m", 51, 52),
        });
    }
    authority(ir)
}

/// Canonical child indices and branch conditions are literal expectations, independent of traversal.
#[test]
fn every_residual_shape_preserves_dependency_paths_and_guards() {
    let paths = [
        vec![vec![0]],
        vec![vec![0]],
        vec![vec![0], vec![1]],
        vec![vec![0], vec![1], vec![2]],
        vec![vec![0], vec![1]],
        vec![vec![0, 0]],
        vec![vec![0]],
        vec![vec![0]],
        vec![vec![0, 0], vec![1], vec![2]],
    ];
    for (kind, expected) in paths.into_iter().enumerate() {
        let project = project(kind);
        let owner = symbol("m", "result");
        let cancel = CancellationToken::new();
        let run = |signal: &CancellationToken| {
            project.dependencies(&owner, ExpressionInspectionLimits::default(), signal)
        };
        let (result, checkpoints) = observe(None, || run(&cancel));
        let edges = result.unwrap();
        assert_eq!(
            edges
                .edges()
                .iter()
                .map(|edge| edge.node_path.clone())
                .collect::<Vec<_>>(),
            expected,
            "shape {kind}"
        );
        assert!(edges.edges().iter().all(|edge| edge.owner == &owner));
        if matches!(kind, 3 | 8) {
            assert_eq!(edges.edges()[0].guards.len(), 0);
            for (edge, condition) in edges.edges()[1..].iter().zip(if kind == 3 {
                [
                    GuardCondition::Boolean(true),
                    GuardCondition::Boolean(false),
                ]
            } else {
                [
                    GuardCondition::Alternative("failed"),
                    GuardCondition::Alternative("ready"),
                ]
            }) {
                assert_eq!(edge.guards.len(), 1);
                assert_eq!(edge.guards[0].node_path, [0]);
                assert_eq!(edge.guards[0].condition, condition);
            }
        } else {
            assert!(edges.edges().iter().all(|edge| edge.guards.is_empty()));
        }
        for index in 0..checkpoints {
            assert_eq!(
                observe(Some(index), || run(&cancel)).0.unwrap_err(),
                E::Limit,
                "allocation {index}, shape {kind}"
            );
            let signal = CancellationToken::new();
            assert_eq!(
                observe_cancellation(index, &signal, || run(&signal))
                    .0
                    .unwrap_err(),
                E::Cancelled,
                "cancellation {index}, shape {kind}"
            );
        }
        assert!(run(&cancel).is_ok());
    }
}

/// Structural companion walks reject every insufficient budget before granting graph authority.
#[test]
fn companion_graphs_enforce_the_exact_aggregate_work_budget() {
    for kind in 0..9 {
        let project = project(kind);
        let cancel = CancellationToken::new();
        let mut budget = Budget {
            work: limits().expression.work,
            cancel: &cancel,
        };
        crate::expression::companions::provenance::validate(project.model(), &mut budget).unwrap();
        let exact = limits().expression.work - budget.work;
        assert!(exact > 0);
        for work in 0..exact {
            assert_eq!(
                crate::expression::companions::provenance::validate(
                    project.model(),
                    &mut Budget {
                        work,
                        cancel: &cancel
                    }
                ),
                Err(E::Limit),
                "shape {kind}, work {work}/{exact}"
            );
        }
        assert_eq!(
            crate::expression::companions::provenance::validate(
                project.model(),
                &mut Budget {
                    work: exact,
                    cancel: &cancel
                }
            ),
            Ok(())
        );
    }
}
