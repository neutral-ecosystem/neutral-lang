// SPDX-License-Identifier: Apache-2.0

//! Literal construction and independent bounds for the raw expression foundation.

use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity, composition::BindingValue as V, expression::*,
    project_interface::ProjectPublicType as T,
};
mod allocation;
mod inspection;
mod project;

fn symbol(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "example"), name)
}

fn leaf(value: bool) -> Expression {
    Expression {
        ty: T::Bool,
        node: ExpressionNode::Value(V::Bool(value)),
    }
}

#[test]
fn input_null_and_constant_bodies_are_distinct() {
    let input = ExpressionBody::Input;
    let null = ExpressionBody::Value(V::Null);
    let value = ExpressionBody::Value(V::Bool(false));
    assert_ne!(input, null);
    assert_ne!(input, value);
    assert_ne!(null, value);
}

#[test]
fn output_retains_contract_and_typed_selector() {
    let output = Expression {
        ty: T::String,
        node: ExpressionNode::Output {
            target: symbol("operation"),
            result_type: T::Nominal(symbol("Result")),
            selector: vec![
                SelectorSegment::Field("items".into()),
                SelectorSegment::Index(u32::MAX),
                SelectorSegment::Alternative("ready".into()),
            ],
        },
    };
    let ExpressionNode::Output {
        target,
        result_type,
        selector,
    } = output.node
    else {
        panic!("output required")
    };
    assert_eq!(target.declaration_name(), "operation");
    assert_eq!(result_type, T::Nominal(symbol("Result")));
    assert_eq!(
        selector,
        [
            SelectorSegment::Field("items".into()),
            SelectorSegment::Index(u32::MAX),
            SelectorSegment::Alternative("ready".into())
        ]
    );
}

#[test]
fn nodes_preserve_guards_scope_and_operand_order() {
    let tree = Expression {
        ty: T::Bool,
        node: ExpressionNode::If {
            guard: Box::new(leaf(true)),
            then_branch: Box::new(leaf(false)),
            else_branch: Box::new(Expression {
                ty: T::Bool,
                node: ExpressionNode::Bound { depth: 0, slot: 2 },
            }),
        },
    };
    let ExpressionNode::If {
        guard,
        then_branch,
        else_branch,
    } = tree.node
    else {
        panic!("if required")
    };
    assert_eq!(*guard, leaf(true));
    assert_eq!(*then_branch, leaf(false));
    assert_eq!(
        else_branch.node,
        ExpressionNode::Bound { depth: 0, slot: 2 }
    );
}
