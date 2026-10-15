// SPDX-License-Identifier: Apache-2.0

//! Private resolution boundaries retain exact failure classifications and locations.

use super::super::{CancellationToken, ExpansionLimits, ExpressionLimits, initial_state, symbol};
use super::*;
use crate::{
    CapturedExpressionProject, CapturedExpressionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_expression_project,
};
use neutral_core::{StructuralLimits, profile::LanguageProfile};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

fn capture(
    bytes: &str,
    expression: ExpressionLimits,
    expansion: ExpansionLimits,
) -> CapturedExpressionProject {
    let limits = ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 4096,
        source_units: 1,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 1,
        vocabulary_bytes_per_unit: 4096,
        total_vocabulary_bytes: 4096,
        imports_per_module: 1,
        import_edges: 1,
        scc_units: 1,
        declarations: 16,
        diagnostics: 16,
        output_bytes: 4096,
    });
    capture_expression_project(CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            expression_profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "source:test",
                "example::expressions",
                bytes.as_bytes().to_vec(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(limits, CancellationToken::new()),
        ),
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(4096, 64).unwrap(),
        )),
        expression,
        expansion,
    ))
    .unwrap()
}

#[test]
fn missing_binding_components_report_the_owner_location_when_available() {
    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\nnum result = 1\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let cancel = CancellationToken::new();
    let owner = symbol("example::expressions", "result").unwrap();
    let mut state = initial_state(&captured, &cancel).unwrap();
    assert_eq!(
        state.binding(&owner).unwrap_err(),
        error(ec::EXP_NAME, None)
    );

    let mut state = initial_state(&captured, &cancel).unwrap();
    parser::parse_source(&captured.sources()[0], &mut state).unwrap();
    let location = state.resolver.roots[&owner].location;
    let syntax = state.bindings.remove(&owner).unwrap();
    assert_eq!(
        state.binding(&owner).unwrap_err(),
        error(ec::EXP_NAME, Some(location))
    );
    state.active.remove(&owner);
    state.bindings.insert(owner.clone(), syntax).unwrap();
    assert_eq!(
        state.binding(&owner).unwrap_err(),
        error(ec::EXP_NAME, Some(location))
    );
}

#[test]
fn cached_binding_clones_and_axis_substitution_obey_expression_limits() {
    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\nnum result = 1\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let cancel = CancellationToken::new();
    let owner = symbol("example::expressions", "result").unwrap();
    let value = || Expression {
        ty: T::Bool,
        node: N::Value(V::Bool(true)),
    };
    for body in [
        ExpressionBody::Value(V::Bool(true)),
        ExpressionBody::Expression(value()),
    ] {
        let mut state = initial_state(&captured, &cancel).unwrap();
        state.bodies.insert(owner.clone(), body).unwrap();
        state.budget.expression.nodes = 0;
        assert_eq!(
            state.binding(&owner).unwrap_err(),
            error(ec::EXP_LIMIT, None)
        );
    }
    let mut state = initial_state(&captured, &cancel).unwrap();
    parser::parse_source(&captured.sources()[0], &mut state).unwrap();
    let location = state.resolver.roots[&owner].location;
    state
        .bindings
        .get_mut(&owner)
        .unwrap()
        .axes
        .push(("axis", value()));
    state.budget.expression.nodes = 0;
    assert_eq!(
        state.binding(&owner).unwrap_err(),
        error(ec::EXP_LIMIT, Some(location))
    );
}

#[test]
fn lifting_a_bound_slot_rejects_depth_overflow_without_changing_it() {
    let mut expression = Expression {
        ty: T::Bool,
        node: N::Bound {
            depth: u64::MAX,
            slot: 0,
        },
    };
    assert_eq!(
        lift(&mut expression, 1, 0).unwrap_err(),
        error(ec::EXP_LIMIT, None)
    );
    assert!(matches!(
        expression.node,
        N::Bound {
            depth: u64::MAX,
            slot: 0
        }
    ));
}
