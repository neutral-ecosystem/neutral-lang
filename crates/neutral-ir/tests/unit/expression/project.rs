// SPDX-License-Identifier: Apache-2.0

//! Complete successor data distinguishes input bodies and exact profile selection.

use super::*;
use neutral_ir::composition::project::CompositionSignature as S;
use neutral_ir::expression::{profile::SemanticProfiles, project::ExpressionDeclaration};

#[test]
fn project_inputs_have_no_fabricated_value() {
    let declaration = ExpressionDeclaration {
        identity: symbol("external"),
        public: true,
        signature: S::Binding(T::String),
        body: Some(ExpressionBody::Input),
    };
    assert_eq!(declaration.validate_body_shape(), Ok(()));
    assert_eq!(declaration.body, Some(ExpressionBody::Input));
    let missing = ExpressionDeclaration {
        body: None,
        ..declaration
    };
    assert_eq!(
        missing.validate_body_shape(),
        Err(ExpressionShapeError::Malformed)
    );
}

#[test]
fn definitions_reject_binding_bodies() {
    let mut declaration = ExpressionDeclaration {
        identity: symbol("Record"),
        public: true,
        signature: S::Definition(neutral_ir::composition::CompositionBody::Record(vec![])),
        body: Some(ExpressionBody::Input),
    };
    assert_eq!(
        declaration.validate_body_shape(),
        Err(ExpressionShapeError::Malformed)
    );
    declaration.body = None;
    assert_eq!(declaration.validate_body_shape(), Ok(()));
}

#[test]
fn project_profile_selection_never_falls_back() {
    let mut profiles = SemanticProfiles::try_frozen().unwrap();
    assert!(profiles.matches_frozen());
    profiles.required_features.pop();
    assert!(!profiles.matches_frozen());
    profiles = SemanticProfiles::try_frozen().unwrap();
    profiles.required_features.swap(0, 1);
    assert!(!profiles.matches_frozen());
    profiles = SemanticProfiles::try_frozen().unwrap();
    profiles.expression = "neutral.expression/unknown".into();
    assert!(!profiles.matches_frozen());
}

#[test]
fn contract_preflight_rejects_deep_types_before_copy() {
    use neutral_core::{CancellationToken, allocation::testing::observe};
    use neutral_ir::{
        composition::{
            CompositionAlternative, CompositionBody, project::inspect_composition_bodies,
        },
        project::ProjectLimits,
    };
    let mut ty = T::Bool;
    for _ in 0..65 {
        ty = T::List(Box::new(ty));
    }
    let body = CompositionBody::Variant(vec![CompositionAlternative {
        tag: "ready".into(),
        ty,
    }]);
    let limits = ProjectLimits {
        modules: 1,
        declarations: 4,
        import_edges: 4,
        nodes: 1024,
        text_bytes: 1024,
        artifact_bytes: 4096,
    };
    let policy = neutral_ir::composition::project::CompositionPolicy::from_values([1024; 15]);
    let (result, copies) = observe(None, || {
        inspect_composition_bodies([&body], limits, policy, &CancellationToken::new())
    });
    assert!(result.is_err());
    assert_eq!(copies, 0);
}
