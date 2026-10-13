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
        ir
    };
    assert!(validate(make()).is_ok());
    let mut nodes = make();
    nodes.expression_limits.nodes = 3;
    assert_eq!(validate(nodes).unwrap_err(), E::Limit);
    let mut depth = make();
    depth.expression_limits.evaluation_depth = 1;
    assert_eq!(validate(depth).unwrap_err(), E::Limit);
}
