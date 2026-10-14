// SPDX-License-Identifier: Apache-2.0

//! Public template frames and defaults are checked without instantiating or evaluating them.

use super::*;
use neutral_ir::expression::templates::{ParameterPhase, PublicTemplate, TemplateParameter};

fn bound(depth: u64, slot: u64) -> Expression {
    Expression {
        ty: T::Bool,
        node: ExpressionNode::Bound { depth, slot },
    }
}

fn template_project() -> ExpressionProjectIr {
    let mut ir = project(vec![]);
    ir.public_templates.push(PublicTemplate {
        identity: symbol("Choose"),
        parameters: vec![
            TemplateParameter {
                name: "first".into(),
                phase: ParameterPhase::Static,
                ty: T::Bool,
                default: None,
            },
            TemplateParameter {
                name: "second".into(),
                phase: ParameterPhase::Deferred,
                ty: T::Bool,
                default: Some(bound(0, 0)),
            },
        ],
        result_type: T::Bool,
        body: bound(0, 1),
    });
    // One default and one body; each bound/bool tree has nine UTF-8 tag bytes.
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 2,
        residual_depth: 1,
        public_templates: 1,
        retained_expression_text_bytes: 18,
        ..ExpressionResourceFacts::default()
    };
    ir
}

#[test]
fn public_template_defaults_see_only_earlier_parameters() {
    let model = validate(template_project()).unwrap();
    assert_eq!(model.facts().values(), [2, 1, 0, 0, 0, 1, 18]);
    for slot in [1, u64::MAX] {
        let mut ir = template_project();
        ir.public_templates[0].parameters[1].default = Some(bound(0, slot));
        assert_eq!(validate(ir).unwrap_err(), E::TypeMismatch);
    }
    let mut ir = template_project();
    ir.public_templates[0].body = bound(u64::MAX, 0);
    assert_eq!(validate(ir).unwrap_err(), E::TypeMismatch);
}

#[test]
fn templates_reject_duplicate_parameters_and_required_after_default() {
    let mut duplicate = template_project();
    duplicate.public_templates[0].parameters[1].name = "first".into();
    assert_eq!(validate(duplicate).unwrap_err(), E::Malformed);
    let mut order = template_project();
    order.public_templates[0].parameters[0].default = Some(leaf(true));
    order.public_templates[0].parameters[1].default = None;
    assert_eq!(validate(order).unwrap_err(), E::Malformed);
}

#[test]
fn template_parameter_bounds_are_independent_and_exact() {
    let mut exact = template_project();
    exact.expansion_limits.parameters = 2;
    exact.expansion_limits.definitions = 1;
    assert!(validate(exact).is_ok());
    let mut over = template_project();
    over.expansion_limits.parameters = 1;
    assert_eq!(validate(over).unwrap_err(), E::Limit);
}

#[test]
fn template_definition_count_is_aggregate_not_per_definition() {
    let mut ir = template_project();
    let mut other = template_project().public_templates.remove(0);
    other.identity = symbol("ChooseAgain");
    ir.public_templates.push(other);
    ir.expression_resources.residual_nodes = 4;
    ir.expression_resources.public_templates = 2;
    ir.expression_resources.retained_expression_text_bytes = 36;
    ir.expansion_limits.definitions = 1;
    assert_eq!(validate(ir).unwrap_err(), E::Limit);
    let mut ir = template_project();
    let mut other = template_project().public_templates.remove(0);
    other.identity = symbol("ChooseAgain");
    ir.public_templates.push(other);
    ir.expression_resources.residual_nodes = 4;
    ir.expression_resources.public_templates = 2;
    ir.expression_resources.retained_expression_text_bytes = 36;
    ir.expansion_limits.definitions = 2;
    assert!(validate(ir).is_ok());
}

#[test]
fn template_retention_and_work_policies_are_not_ignored() {
    let mut bytes = template_project();
    bytes.expansion_limits.retained_bytes = 1;
    assert_eq!(validate(bytes).unwrap_err(), E::Limit);
    let mut work = template_project();
    work.expansion_limits.work = 1;
    assert_eq!(validate(work).unwrap_err(), E::Limit);
}

#[test]
fn public_templates_reject_private_types_and_namespace_collisions() {
    let mut private = template_project();
    private.declarations.push(D {
        identity: symbol("Private"),
        public: false,
        signature: S::Definition(CompositionBody::Record(vec![])),
        body: None,
    });
    private.public_templates[0].parameters[0].ty = T::Nominal(symbol("Private"));
    assert_eq!(validate(private).unwrap_err(), E::UnknownOrPrivate);
    let mut collision = template_project();
    collision.declarations.push(D {
        identity: symbol("Choose"),
        public: true,
        signature: S::Definition(CompositionBody::Record(vec![])),
        body: None,
    });
    assert_eq!(validate(collision).unwrap_err(), E::Malformed);
    let mut missing = template_project();
    missing.public_templates[0].identity =
        ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "missing"), "Choose");
    assert_eq!(validate(missing).unwrap_err(), E::Malformed);
}

#[test]
fn nested_match_payload_does_not_capture_template_parameter_slots() {
    let make = |depth, slot| {
        let mut ir = project(vec![D {
            identity: symbol("State"),
            public: true,
            signature: S::Definition(CompositionBody::Variant(vec![
                neutral_ir::composition::CompositionAlternative {
                    tag: "ready".into(),
                    ty: T::Bool,
                },
            ])),
            body: None,
        }]);
        ir.public_templates.push(PublicTemplate {
            identity: symbol("Choose"),
            parameters: vec![
                TemplateParameter {
                    name: "state".into(),
                    phase: ParameterPhase::Deferred,
                    ty: T::Nominal(symbol("State")),
                    default: None,
                },
                TemplateParameter {
                    name: "second".into(),
                    phase: ParameterPhase::Static,
                    ty: T::Bool,
                    default: None,
                },
            ],
            result_type: T::Bool,
            body: Expression {
                ty: T::Bool,
                node: ExpressionNode::Match {
                    scrutinee: Box::new(Expression {
                        ty: T::Nominal(symbol("State")),
                        node: ExpressionNode::Bound { depth: 0, slot: 0 },
                    }),
                    arms: vec![MatchArm {
                        tag: "ready".into(),
                        payload_type: T::Bool,
                        body: bound(depth, slot),
                    }],
                },
            },
        });
        // match/bool 9 + bound/nominal/owner 27 + arm tag/type 9 + bound/bool 9.
        ir.expression_resources = ExpressionResourceFacts {
            residual_nodes: 3,
            residual_depth: 2,
            public_templates: 1,
            retained_expression_text_bytes: 54,
            ..ExpressionResourceFacts::default()
        };
        ir
    };
    assert!(validate(make(1, 1)).is_ok());
    assert_eq!(validate(make(0, 1)).unwrap_err(), E::TypeMismatch);
    assert!(validate(make(0, 0)).is_ok());
}

#[test]
fn template_type_validation_does_not_evaluate_arithmetic() {
    let number = |digits| Expression {
        ty: T::Num,
        node: ExpressionNode::Value(neutral_ir::composition::BindingValue::Number(
            neutral_ir::ExactNumber::from_unsigned_integer(digits).unwrap(),
        )),
    };
    let mut ir = project(vec![]);
    ir.public_templates.push(PublicTemplate {
        identity: symbol("Unevaluated"),
        parameters: vec![],
        result_type: T::Num,
        body: Expression {
            ty: T::Num,
            node: ExpressionNode::Binary {
                operator: BinaryOperator::Divide,
                left: Box::new(number("1")),
                right: Box::new(number("0")),
            },
        },
    });
    // binary/num/operator 10 + two value/num/num/coefficient leaves of 12 bytes.
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 3,
        residual_depth: 2,
        public_templates: 1,
        retained_expression_text_bytes: 34,
        ..ExpressionResourceFacts::default()
    };
    assert!(validate(ir).is_ok());
}
