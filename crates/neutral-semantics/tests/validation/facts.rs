// SPDX-License-Identifier: Apache-2.0

//! Literal resource oracles cover every fact and aggregate residual tree boundaries.

use super::*;

fn input_project() -> ExpressionProjectIr {
    let mut ir = project(vec![
        binding("external", T::Bool, ExpressionBody::Input),
        binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Input(symbol("external")),
            }),
        ),
    ]);
    // input + bool + source profile + module + binding name = 5 + 4 + 3 + 7 + 8.
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 1,
        residual_depth: 1,
        input_declarations: 1,
        potential_value_edges: 1,
        retained_expression_text_bytes: 27,
        ..ExpressionResourceFacts::default()
    };
    ir
}

#[test]
fn every_producer_expression_fact_is_independently_checked() {
    assert_eq!(
        validate(input_project()).unwrap().facts().values(),
        [1, 1, 1, 0, 1, 0, 27]
    );
    for index in 0..7 {
        let mut raw = input_project();
        let facts = &mut raw.expression_resources;
        let claim = match index {
            0 => &mut facts.residual_nodes,
            1 => &mut facts.residual_depth,
            2 => &mut facts.input_declarations,
            3 => &mut facts.output_leaves,
            4 => &mut facts.potential_value_edges,
            5 => &mut facts.public_templates,
            _ => &mut facts.retained_expression_text_bytes,
        };
        *claim += 1;
        assert_eq!(validate(raw).unwrap_err(), E::Resources);
    }
}

#[test]
fn residual_counts_and_depth_accept_exact_and_reject_one_over() {
    let make = || {
        let mut ir = project(vec![binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::If {
                    guard: Box::new(leaf(true)),
                    then_branch: Box::new(leaf(false)),
                    else_branch: Box::new(leaf(true)),
                },
            }),
        )]);
        // if/bool plus three value/bool/bool leaves = 6 + 3 * 13.
        ir.expression_resources = ExpressionResourceFacts {
            residual_nodes: 4,
            residual_depth: 2,
            retained_expression_text_bytes: 45,
            ..ExpressionResourceFacts::default()
        };
        ir.expression_limits.nodes = 4;
        ir.expression_limits.evaluation_depth = 2;
        ir.expression_limits.syntax_depth = 2;
        ir
    };
    assert!(validate(make()).is_ok());
    let mut nodes = make();
    nodes.expression_limits.nodes = 3;
    assert_eq!(validate(nodes).unwrap_err(), E::Limit);
    let mut depth = make();
    depth.expression_limits.evaluation_depth = 1;
    assert_eq!(validate(depth).unwrap_err(), E::Limit);
    let mut syntax = make();
    syntax.expression_limits.syntax_depth = 1;
    assert_eq!(validate(syntax).unwrap_err(), E::Limit);
}

#[test]
fn model_counts_obey_each_producer_and_caller_boundary() {
    for select in 0..3 {
        let set = |limits: &mut SemanticLimits, count| match select {
            0 => limits.project.declarations = count,
            1 => limits.expression.bindings = count,
            _ => limits.expression.dependency_edges = count,
        };
        for count in [1, 2] {
            for caller_bound in [false, true] {
                let mut ir = input_project();
                if select == 2 {
                    ir.declarations[1].body = Some(ExpressionBody::Expression(Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Binary {
                            operator: BinaryOperator::And,
                            left: Box::new(Expression {
                                ty: T::Bool,
                                node: ExpressionNode::Input(symbol("external")),
                            }),
                            right: Box::new(Expression {
                                ty: T::Bool,
                                node: ExpressionNode::Input(symbol("external")),
                            }),
                        },
                    }));
                    ir.expression_resources = ExpressionResourceFacts {
                        residual_nodes: 3,
                        residual_depth: 2,
                        input_declarations: 1,
                        potential_value_edges: 2,
                        retained_expression_text_bytes: 66,
                        ..ExpressionResourceFacts::default()
                    };
                }
                let mut caller = controls();
                let mut producer = controls();
                if caller_bound {
                    set(&mut caller, count);
                } else {
                    set(&mut producer, count);
                }
                ir.limits = producer.project;
                ir.expression_limits = producer.expression;
                let borrowed = Shared::try_new(ir).unwrap();
                let result = validate_expression_model(borrowed, caller, &CancellationToken::new());
                if count == 2 {
                    assert!(result.is_ok(), "control {select}");
                } else {
                    assert_eq!(result.unwrap_err(), E::Limit, "control {select}");
                }
            }
        }
    }
}

/// Multiple imports must be canonical even when every referenced module exists.
#[test]
fn module_imports_reject_duplicate_and_reversed_targets() {
    for (targets, accepted) in [
        (vec!["left".into(), "right".into()], true),
        (vec!["right".into(), "left".into()], false),
        (vec!["left".into(), "left".into()], false),
    ] {
        let mut ir = project(vec![]);
        for name in ["left", "right"] {
            ir.modules.push(ProjectModule {
                identity: LogicalModuleIdentity::new("1.0", name),
                imports: vec![],
            });
        }
        ir.modules[0].imports = targets;
        let result = validate(ir);
        if accepted {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err(), E::Malformed);
        }
    }
}
