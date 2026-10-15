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
fn profile_names_and_limits_are_exact_and_intersect_independently() {
    use neutral_ir::expression::profile::{ExpressionLimits, is_binding_name};

    for valid in ["answer", "choice__0", "choice__12__3"] {
        assert!(is_binding_name(valid), "{valid}");
    }
    for invalid in [
        "Upper",
        "choice__",
        "choice__01",
        "choice__1x",
        "choice__18446744073709551616",
    ] {
        assert!(!is_binding_name(invalid), "{invalid}");
    }

    let hard = ExpressionLimits::hard();
    assert_eq!(ExpressionLimits::default(), hard);
    assert_eq!(hard.values().len(), 11);
    let caller = ExpressionLimits {
        nodes: 2,
        work: 30,
        ..hard
    };
    let producer = ExpressionLimits {
        nodes: 9,
        work: 10,
        ..hard
    };
    let effective = caller.intersect(producer);
    assert_eq!(effective.nodes, 2);
    assert_eq!(effective.work, 10);
    assert_eq!(
        ExpressionLimits {
            bindings: 0,
            ..hard
        }
        .intersect(hard)
        .bindings,
        0
    );
}

#[test]
fn fallible_composition_value_clone_preserves_every_variant() {
    use neutral_core::allocation::TryClone;
    use neutral_ir::composition::{
        BindingValue, CompositionBinding, CompositionBindingReference, CompositionValue as V,
        ValuePathSegment,
    };

    let value = V::Record(vec![
        ("bool".into(), Some(V::Bool(true))),
        ("null".into(), Some(V::Null)),
        ("string".into(), Some(V::String("text".into()))),
        ("url".into(), Some(V::Url("https://example.invalid".into()))),
        ("path".into(), Some(V::Path("/tmp/value".into()))),
        (
            "number".into(),
            Some(V::Number(
                neutral_ir::ExactNumber::from_normalized_parts(false, "12", 1, 2, 2).unwrap(),
            )),
        ),
        (
            "list".into(),
            Some(V::List(vec![V::Variant {
                tag: "ready".into(),
                payload: Box::new(V::Reference(symbol("target"))),
            }])),
        ),
        ("omitted".into(), None),
    ]);
    let copy: BindingValue = value.try_clone().unwrap();
    assert_eq!(copy, value);

    let reference = CompositionBindingReference {
        path: vec![
            ValuePathSegment::Field("item".into()),
            ValuePathSegment::Element(2),
            ValuePathSegment::Payload,
        ],
        target: symbol("target"),
    };
    assert_eq!(reference.try_clone().unwrap(), reference);
    let value = V::Reference(reference);
    let copy: V<CompositionBindingReference> = value.try_clone().unwrap();
    assert_eq!(copy, value);
    let binding = CompositionBinding {
        owner: symbol("binding"),
        public: true,
        ty: T::Bool,
        value: BindingValue::Bool(true),
    };
    assert_eq!(binding.try_clone().unwrap(), binding);
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

#[test]
fn composition_preflight_visits_closed_choices_and_defaults() {
    use neutral_core::CancellationToken;
    use neutral_ir::{
        composition::{
            ClosedValue, CompositionBody, CompositionField, FieldPresence, FieldRestrictions,
            project::{CompositionPolicy, inspect_composition_bodies},
        },
        project::ProjectLimits,
    };

    let body = CompositionBody::Record(vec![CompositionField {
        name: "status".into(),
        ty: T::String,
        presence: FieldPresence::Defaulted,
        restrictions: FieldRestrictions {
            choices: Some(vec![ClosedValue::String("ready".into())]),
            ..FieldRestrictions::default()
        },
        default: Some(ClosedValue::String("ready".into())),
    }]);
    let limits = ProjectLimits {
        modules: 1,
        declarations: 1,
        import_edges: 1,
        nodes: 1024,
        text_bytes: 4096,
        artifact_bytes: 4096,
    };
    let policy = CompositionPolicy::from_values([1024; 15]);
    let inspection =
        inspect_composition_bodies([&body], limits, policy, &CancellationToken::new()).unwrap();
    assert_eq!(inspection.facts.types, 1);
    assert_eq!(inspection.facts.retained_value_nodes, 2);
}
