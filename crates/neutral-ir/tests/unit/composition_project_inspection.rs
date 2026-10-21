// SPDX-License-Identifier: Apache-2.0

//! Crate-local tests for retained expression inspection inside composition projects.

use super::*;
use crate::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{CompositionValue, project::CompositionResourceFacts},
    expression::{Expression, ExpressionNode},
    project::ProjectLimits,
    project_interface::ProjectPublicType,
};
use neutral_core::CancellationToken;

#[test]
fn expression_reference_values_charge_their_identity() {
    let cancellation = CancellationToken::new();
    let expression = Expression {
        ty: ProjectPublicType::Bool,
        node: ExpressionNode::Value(CompositionValue::Reference(ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new("1.0", "module"),
            "target",
        ))),
    };
    let mut inspector = Inspector {
        bytes: 0,
        limits: ProjectLimits {
            modules: 1,
            declarations: 1,
            import_edges: 1,
            nodes: 16,
            text_bytes: 256,
            artifact_bytes: 1024,
        },
        policy: CompositionPolicy::from_values([256; 15]),
        cancellation: &cancellation,
        work: 0,
        text: 0,
        facts: CompositionResourceFacts::from_values([0; 7]),
    };

    inspector.expression(&expression).unwrap();
    assert_eq!(inspector.facts.retained_value_nodes, 1);
}
