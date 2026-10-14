// SPDX-License-Identifier: Apache-2.0

//! Nullable nominal constructors retain outer types without relaxing payload or field contracts.

use super::*;
use neutral_ir::composition::{BindingValue as V, ClosedValue, CompositionAlternative};
use neutral_semantics::{
    Binding, BindingKey, BindingState, EvaluationError as F, EvaluationRequest,
    EvaluationResult as R, ExpressionRoot, evaluate,
};

/// Provides a nullable nominal constructor with either a constrained field or a variant payload.
fn constructor_project(variant: bool, child: Expression, deferred: bool) -> ExpressionProjectIr {
    let name = if variant { "State" } else { "Bucket" };
    let ty = T::Nullable(Box::new(T::Nominal(symbol(name))));
    let contract = if variant {
        CompositionBody::Variant(vec![CompositionAlternative {
            tag: "ready".into(),
            ty: T::Bool,
        }])
    } else {
        CompositionBody::Record(vec![CompositionField {
            name: "flag".into(),
            ty: T::Bool,
            presence: FieldPresence::Required,
            restrictions: FieldRestrictions {
                choices: Some(vec![ClosedValue::Bool(true)]),
                ..FieldRestrictions::default()
            },
            default: None,
        }])
    };
    let node = if variant {
        ExpressionNode::Variant {
            tag: "ready".into(),
            payload: Box::new(child),
        }
    } else {
        ExpressionNode::Record(vec![("flag".into(), Some(child))])
    };
    let mut ir = project(vec![
        D {
            identity: symbol(name),
            public: true,
            signature: S::Definition(contract),
            body: None,
        },
        binding("external", T::Bool, ExpressionBody::Input),
        binding(
            "result",
            ty.clone(),
            ExpressionBody::Expression(Expression { ty, node }),
        ),
    ]);
    // Nullable record+field 41 / variant+tag 42, plus input 27 / bool value 13.
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 2,
        residual_depth: 2,
        input_declarations: 1,
        potential_value_edges: u64::from(deferred),
        retained_expression_text_bytes: match (variant, deferred) {
            (false, true) => 68,
            (true, true) => 69,
            (false, false) => 54,
            (true, false) => 55,
        },
        ..ExpressionResourceFacts::default()
    };
    ir
}

/// Produces the explicit typed deferred child shared by both constructor fixtures.
fn deferred_child() -> Expression {
    Expression {
        ty: T::Bool,
        node: ExpressionNode::Input(symbol("external")),
    }
}

/// Evaluates one fixture generation without modifying the independently validated model.
fn run<'m>(
    model: &'m neutral_semantics::ValidatedExpressionModel,
    bindings: &[Binding<'_>],
) -> R<'m> {
    evaluate(
        model,
        &EvaluationRequest {
            selected: &symbol("result"),
            profiles: &model.ir().semantic_profiles,
            bindings,
            associations: &[],
            limits: ExpressionLimits::hard(),
        },
        &CancellationToken::new(),
    )
}

/// Catches lost nominal restriction owners when a deferred record's outer type is nullable.
#[test]
fn nullable_record_retains_exact_field_restriction_and_recovers_after_failure() {
    let model = validate(constructor_project(false, deferred_child(), true)).unwrap();
    assert_eq!(model.facts().values(), [2, 2, 1, 0, 1, 0, 68]);
    let obligations = model.restriction_obligations();
    assert_eq!(obligations.len(), 1);
    assert_eq!(obligations[0].contract, T::Nominal(symbol("Bucket")));
    assert_eq!(obligations[0].field, "flag");
    assert_eq!(obligations[0].site.owner, symbol("result"));
    assert_eq!(obligations[0].site.root, ExpressionRoot::Binding);
    assert_eq!(obligations[0].site.node_path, [0]);
    let external = symbol("external");
    assert_eq!(
        run(&model, &[]),
        R::RequiredBindings(vec![BindingKey::Input(&external)])
    );
    for (value, expected) in [
        (false, R::Failure(F::Restriction)),
        (
            true,
            R::Value {
                ty: &T::Nullable(Box::new(T::Nominal(symbol("Bucket")))),
                value: V::Record(vec![("flag".into(), Some(V::Bool(true)))]),
            },
        ),
    ] {
        assert_eq!(
            run(
                &model,
                &[Binding {
                    key: BindingKey::Input(&external),
                    ty: &T::Bool,
                    state: BindingState::Value(&V::Bool(value))
                }]
            ),
            expected
        );
    }
    assert_eq!(model.restriction_obligations().len(), 1);
}

/// Catches rejecting a nullable variant's exact non-null tag/payload construction.
#[test]
fn nullable_variant_retains_tag_payload_and_outer_type() {
    let model = validate(constructor_project(true, deferred_child(), true)).unwrap();
    assert_eq!(model.facts().values(), [2, 2, 1, 0, 1, 0, 69]);
    let external = symbol("external");
    assert_eq!(
        run(&model, &[]),
        R::RequiredBindings(vec![BindingKey::Input(&external)])
    );
    for value in [false, true] {
        assert_eq!(
            run(
                &model,
                &[Binding {
                    key: BindingKey::Input(&external),
                    ty: &T::Bool,
                    state: BindingState::Value(&V::Bool(value))
                }]
            ),
            R::Value {
                ty: &T::Nullable(Box::new(T::Nominal(symbol("State")))),
                value: V::Variant {
                    tag: "ready".into(),
                    payload: Box::new(V::Bool(value))
                }
            }
        );
    }
    assert!(model.restriction_obligations().is_empty());
}

/// Catches bypassing immediate field restrictions while admitting nullable static records.
#[test]
fn nullable_static_record_checks_restrictions_without_future_obligations() {
    assert_eq!(
        validate(constructor_project(false, leaf(false), false)).unwrap_err(),
        E::Restriction
    );
    let model = validate(constructor_project(false, leaf(true), false)).unwrap();
    assert_eq!(model.facts().values(), [2, 2, 1, 0, 0, 0, 54]);
    assert!(model.restriction_obligations().is_empty());
    assert_eq!(
        run(&model, &[]),
        R::Value {
            ty: &T::Nullable(Box::new(T::Nominal(symbol("Bucket")))),
            value: V::Record(vec![("flag".into(), Some(V::Bool(true)))])
        }
    );
}

/// Catches general nullable coercion, wrong payloads, and exposure of private contracts.
#[test]
fn nullable_nominal_constructors_keep_exact_children_and_public_closure() {
    for variant in [false, true] {
        for child in [
            Expression {
                ty: T::String,
                node: ExpressionNode::Value(V::String("wrong".into())),
            },
            Expression {
                ty: T::Nullable(Box::new(T::Bool)),
                node: ExpressionNode::Value(V::Null),
            },
        ] {
            assert_eq!(
                validate(constructor_project(variant, child, false)).unwrap_err(),
                E::TypeMismatch
            );
        }
        let mut private = constructor_project(variant, leaf(true), false);
        private.declarations[0].public = false;
        assert_eq!(validate(private).unwrap_err(), E::UnknownOrPrivate);
        let mut primitive = constructor_project(variant, leaf(true), false);
        let ty = T::Nullable(Box::new(T::Bool));
        primitive.declarations[2].signature = S::Binding(ty.clone());
        let Some(ExpressionBody::Expression(expression)) = &mut primitive.declarations[2].body
        else {
            unreachable!();
        };
        expression.ty = ty;
        assert_eq!(validate(primitive).unwrap_err(), E::TypeMismatch);
    }
}
