// SPDX-License-Identifier: Apache-2.0

//! Output binding needs actual typed root evidence; selected values alone cannot prove availability.

use super::*;
use neutral_ir::composition::BindingValue as V;
use neutral_semantics::{
    Binding, BindingKey, BindingState, EvaluationError as F, EvaluationRequest,
    EvaluationResult as R, OutputAssociation, evaluate,
};

fn program() -> neutral_semantics::ValidatedExpressionModel {
    let field = CompositionField {
        name: "items".into(),
        ty: T::List(Box::new(T::Bool)),
        presence: FieldPresence::Required,
        restrictions: FieldRestrictions::default(),
        default: None,
    };
    let mut ir = project(vec![
        D {
            identity: symbol("Operation"),
            public: true,
            signature: S::Definition(CompositionBody::Record(vec![])),
            body: None,
        },
        D {
            identity: symbol("Result"),
            public: true,
            signature: S::Definition(CompositionBody::Record(vec![field])),
            body: None,
        },
        binding("ask", T::Bool, ExpressionBody::Input),
        binding(
            "operation",
            T::Nominal(symbol("Operation")),
            ExpressionBody::Input,
        ),
        binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::If {
                    guard: Box::new(Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Input(symbol("ask")),
                    }),
                    then_branch: Box::new(leaf(false)),
                    else_branch: Box::new(Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Output {
                            target: symbol("operation"),
                            result_type: T::Nominal(symbol("Result")),
                            selector: vec![
                                SelectorSegment::Field("items".into()),
                                SelectorSegment::Index(1),
                            ],
                        },
                    }),
                },
            }),
        ),
    ]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 4,
        residual_depth: 2,
        input_declarations: 2,
        output_leaves: 1,
        potential_value_edges: 2,
        retained_expression_text_bytes: 108,
        ..ExpressionResourceFacts::default()
    };
    validate(ir).unwrap()
}

fn run<'m>(
    model: &'m neutral_semantics::ValidatedExpressionModel,
    bindings: &[Binding<'_>],
    associations: &[OutputAssociation<'_>],
) -> R<'m> {
    evaluate(
        model,
        &EvaluationRequest {
            selected: &symbol("result"),
            profiles: &model.ir().semantic_profiles,
            bindings,
            associations,
            limits: ExpressionLimits::hard(),
        },
        &CancellationToken::new(),
    )
}

#[test]
fn unselected_output_is_not_required() {
    let model = program();
    let ask = symbol("ask");
    assert_eq!(
        run(
            &model,
            &[Binding {
                key: BindingKey::Input(&ask),
                ty: &T::Bool,
                state: BindingState::Value(&V::Bool(true))
            }],
            &[]
        ),
        R::Value {
            ty: &T::Bool,
            value: V::Bool(false)
        }
    );
    assert_eq!(
        run(&model, &[], &[]),
        R::RequiredBindings(vec![BindingKey::Input(&ask)])
    );
}

#[test]
fn supplied_output_root_must_discharge_actual_index_obligations() {
    let model = program();
    let ask = symbol("ask");
    let target = symbol("operation");
    let result_type = T::Nominal(symbol("Result"));
    let selector = [
        SelectorSegment::Field("items".into()),
        SelectorSegment::Index(1),
    ];
    let key = BindingKey::Output {
        target: &target,
        result_type: &result_type,
        selector: &selector,
    };
    let association = [OutputAssociation {
        target: &target,
        result_type: &result_type,
    }];
    let valid = V::Record(vec![(
        "items".into(),
        Some(V::List(vec![V::Bool(false), V::Bool(true)])),
    )]);
    let invalid = V::Record(vec![("items".into(), Some(V::List(vec![V::Bool(true)])))]);
    let ask_false = Binding {
        key: BindingKey::Input(&ask),
        ty: &T::Bool,
        state: BindingState::Value(&V::Bool(false)),
    };
    assert_eq!(
        run(
            &model,
            &[
                ask_false,
                Binding {
                    key,
                    ty: &T::Bool,
                    state: BindingState::OutputResult(&valid)
                }
            ],
            &association
        ),
        R::Value {
            ty: &T::Bool,
            value: V::Bool(true)
        }
    );
    assert_eq!(
        run(
            &model,
            &[Binding {
                key,
                ty: &T::Bool,
                state: BindingState::OutputResult(&invalid)
            }],
            &association
        ),
        R::Failure(F::Restriction)
    );
    // Detached selected data cannot prove a parent field restriction or index availability.
    assert_eq!(
        run(
            &model,
            &[Binding {
                key,
                ty: &T::Bool,
                state: BindingState::Value(&V::Bool(true))
            }],
            &association
        ),
        R::Failure(F::BindingType)
    );
}
