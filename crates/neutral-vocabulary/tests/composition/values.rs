// SPDX-License-Identifier: Apache-2.0

//! Independent work and depth acceptance boundaries for the shared value engine.

use super::*;
use neutral_core::{CancellationToken, StructuralLimits, VocabularyContentDigest};
use neutral_ir::{LogicalModuleIdentity, VocabularyIdentity, composition::CompositionAlternative};

/// Keeps unrelated controls generous while each test narrows one independent budget.
fn limits() -> super::super::CompositionLimits {
    super::super::CompositionLimits::from_vocabulary(crate::VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ))
}

/// Accepts exactly the hand-counted work and rejects a budget one unit shorter.
fn work_boundary(work: u64, mut operation: impl FnMut(&mut Budget<'_>) -> Result<(), E>) {
    let cancellation = CancellationToken::new();
    for (available, expected) in [(work, Ok(())), (work - 1, Err(E::Limit))] {
        let mut policy = limits();
        policy.work = available;
        assert_eq!(operation(&mut Budget::new(policy, &cancellation)), expected);
    }
}

/// Builds a required field whose omitted/defaulted policy can be varied by the caller.
fn field(ty: T) -> CompositionField {
    CompositionField {
        name: "x".to_owned(),
        ty,
        presence: FieldPresence::Required,
        restrictions: FieldRestrictions::default(),
        default: None,
    }
}

/// Supplies one exact owner with independently counted key component lengths.
fn nominal() -> T {
    T::VocabularyNominal {
        identity: "Owner".to_owned(),
        version: "1.0".to_owned(),
        name: "Thing".to_owned(),
    }
}

/// Creates a closed definition without deriving test expectations from materialization.
fn definition(body: CompositionBody) -> CompositionDefinition {
    CompositionDefinition {
        name: "Thing".to_owned(),
        public: true,
        body,
    }
}

/// Constructs raw bundle ownership without JSON preprocessing that could mask value limits.
fn bundle(definitions: Vec<CompositionDefinition>) -> CompositionBundle {
    CompositionBundle {
        identity: VocabularyIdentity::new(
            "Owner",
            "1.0",
            super::super::SCHEMA_VERSION,
            super::super::ENCODING_VERSION,
            VocabularyContentDigest::from_bytes(b"value-boundaries"),
            vec![super::super::REQUIRED_FEATURE.to_owned()],
        ),
        dependencies: Vec::new(),
        definitions,
    }
}

/// Four owners require three key-comparison rounds, including both nominal origins.
#[test]
fn nominal_lookup_charges_all_owner_components_and_both_catalogues() {
    let empty = definition(CompositionBody::Record(Vec::new()));
    let owner = ModuleSymbolIdentity::new(LogicalModuleIdentity::new("0.1.0", "Module"), "Thing");
    let mut catalogue = Catalogue::new();
    catalogue.insert(("Owner", "1.0", "Thing"), &empty).unwrap();
    catalogue.insert(("Other", "1.0", "Thing"), &empty).unwrap();
    catalogue.insert(("Third", "1.0", "Thing"), &empty).unwrap();
    catalogue.sources.insert(&owner, &empty).unwrap();
    // (5 + 3 + 5) bytes * 3 rounds * 4 conservative comparisons.
    work_boundary(156, |budget| {
        catalogue.resolve(&nominal(), budget).map(|_| ())
    });
    // (6 + 5) bytes * 3 rounds * 4 conservative comparisons.
    work_boundary(132, |budget| {
        catalogue
            .resolve(&T::Nominal(owner.clone()), budget)
            .map(|_| ())
    });
}

/// Two insertions reserve one and then two units before publishing any defaults.
#[test]
fn default_catalogue_charges_the_first_and_subsequent_insertions() {
    let bundles = vec![bundle(vec![
        definition(CompositionBody::Record(Vec::new())),
        CompositionDefinition {
            name: "Other".to_owned(),
            ..definition(CompositionBody::Record(Vec::new()))
        },
    ])];
    work_boundary(3, |budget| {
        validate_defaults_policy(&mut bundles.clone(), budget, false)
    });
}

/// A default list starts at one for standalone values and zero for project occurrences.
#[test]
fn default_depth_policy_keeps_standalone_and_project_roots_distinct() {
    let mut item = field(T::List(Box::new(T::Bool)));
    item.presence = FieldPresence::Defaulted;
    item.default = Some(V::List(vec![V::Bool(true)]));
    let bundles = vec![bundle(vec![definition(CompositionBody::Record(vec![
        item,
    ]))])];
    let cancellation = CancellationToken::new();
    for (project, depth, expected) in [
        (false, 2, Ok(())),
        (false, 1, Err(E::Limit)),
        (true, 1, Ok(())),
    ] {
        let mut policy = limits();
        policy.value_depth = depth;
        assert_eq!(
            validate_defaults_policy(
                &mut bundles.clone(),
                &mut Budget::new(policy, &cancellation),
                project
            ),
            expected
        );
    }
}

/// Sorting charges coefficient/text bytes and logarithmic rounds before retaining choices.
#[test]
fn choice_sorting_reserves_work_for_long_numeric_and_text_keys() {
    let mut text = field(T::String);
    text.restrictions.choices = Some(vec![
        V::String("ccc".to_owned()),
        V::String("aaa".to_owned()),
        V::String("bbb".to_owned()),
    ]);
    // 3 visits + 3 choices * 3 bytes * 2 rounds * 4 comparisons + 3 * (1 + 3).
    work_boundary(87, |budget| {
        validate_restrictions(&text, budget).map(|_| ())
    });
    let mut numeric = field(T::Num);
    numeric.restrictions.choices = Some(
        ["345", "123", "234"]
            .map(|digits| V::Number(ExactNumber::from_source(digits, 64, 64).unwrap()))
            .to_vec(),
    );
    // Numeric bound checks visit each choice once without rescanning text.
    work_boundary(78, |budget| {
        validate_restrictions(&numeric, budget).map(|_| ())
    });
}

/// Searching charges the larger of a stored choice and the supplied scalar key.
#[test]
fn choice_search_reserves_work_for_the_supplied_key_and_all_rounds() {
    let choices = FieldRestrictions {
        choices: Some(vec![
            V::String("a".to_owned()),
            V::String("bbb".to_owned()),
            V::String("ccc".to_owned()),
        ]),
        ..FieldRestrictions::default()
    };
    // One visit + 3 UTF-8 bytes + 3 bytes * 2 search rounds.
    work_boundary(10, |budget| {
        check_restrictions(&choices, &V::String("bbb".to_owned()), budget)
    });
    let numeric = FieldRestrictions {
        choices: Some(
            ["1", "2", "1234"]
                .map(|digits| V::Number(ExactNumber::from_source(digits, 64, 64).unwrap()))
                .to_vec(),
        ),
        ..FieldRestrictions::default()
    };
    // One visit + 4 coefficient bytes * 2 search rounds.
    work_boundary(9, |budget| {
        check_restrictions(
            &numeric,
            &V::Number(ExactNumber::from_source("1234", 64, 64).unwrap()),
            budget,
        )
    });
}

/// Scalar materialization charges one visit, one depth check and the scalar's key bytes.
#[test]
fn scalar_materialization_reserves_each_scalar_kind() {
    for (value, ty, work) in [
        (V::Bool(true), T::Bool, 3),
        (V::String("abc".to_owned()), T::String, 5),
        (V::Url("abc".to_owned()), T::Url, 5),
        (V::Path("abc".to_owned()), T::Path, 5),
        (
            V::Number(ExactNumber::from_source("123", 64, 64).unwrap()),
            T::Num,
            5,
        ),
    ] {
        work_boundary(work, |budget| {
            materialize(&value, &ty, &Catalogue::new(), budget, 1).map(|_| ())
        });
    }
}

/// Significant digits are accepted at the configured count and rejected one digit over it.
#[test]
fn numeric_digit_limit_is_inclusive_and_independent_of_scale() {
    let cancellation = CancellationToken::new();
    let mut policy = limits();
    policy.json.numeric_digits = 3;
    for (digits, expected) in [("123", Ok(())), ("1234", Err(E::Limit))] {
        let value = V::Number(ExactNumber::from_source(digits, 64, 64).unwrap());
        assert_eq!(
            materialize(
                &value,
                &T::Num,
                &Catalogue::new(),
                &mut Budget::new(policy, &cancellation),
                1
            )
            .map(|_| ()),
            expected
        );
    }
}

/// Nullable interpretation and list children retain standalone depth boundaries.
#[test]
fn nullable_and_list_materialization_charge_child_depth() {
    let cancellation = CancellationToken::new();
    for (value, ty) in [
        (V::Bool(true), T::Nullable(Box::new(T::Bool))),
        (V::List(vec![V::Bool(true)]), T::List(Box::new(T::Bool))),
    ] {
        for (depth, expected) in [(2, Ok(())), (1, Err(E::Limit))] {
            let mut policy = limits();
            policy.value_depth = depth;
            assert_eq!(
                materialize(
                    &value,
                    &ty,
                    &Catalogue::new(),
                    &mut Budget::new(policy, &cancellation),
                    1
                )
                .map(|_| ()),
                expected
            );
        }
    }
}

/// Variant tag limits accept exactly the allowed bytes and reject one additional byte.
#[test]
fn variant_tag_byte_limit_is_inclusive() {
    let variant = definition(CompositionBody::Variant(vec![
        CompositionAlternative {
            tag: "Tag".to_owned(),
            ty: T::Bool,
        },
        CompositionAlternative {
            tag: "Tags".to_owned(),
            ty: T::Bool,
        },
    ]));
    let mut catalogue = Catalogue::new();
    catalogue
        .insert(("Owner", "1.0", "Thing"), &variant)
        .unwrap();
    let cancellation = CancellationToken::new();
    let mut policy = limits();
    policy.json.string_bytes = 3;
    for (tag, expected) in [("Tag", Ok(())), ("Tags", Err(E::Limit))] {
        let value = V::Variant {
            tag: tag.to_owned(),
            payload: Box::new(V::Bool(true)),
        };
        assert_eq!(
            materialize(
                &value,
                &nominal(),
                &catalogue,
                &mut Budget::new(policy, &cancellation),
                1
            )
            .map(|_| ()),
            expected
        );
    }
}

/// Nominal record interpretation and variant payloads each reserve their child depth.
#[test]
fn nominal_record_and_variant_materialization_charge_child_depth() {
    let cancellation = CancellationToken::new();
    for (body, value, exact_depth) in [
        (
            CompositionBody::Record(vec![field(T::Bool)]),
            V::Record(vec![("x".to_owned(), Some(V::Bool(true)))]),
            3,
        ),
        (
            CompositionBody::Variant(vec![CompositionAlternative {
                tag: "Tag".to_owned(),
                ty: T::Bool,
            }]),
            V::Variant {
                tag: "Tag".to_owned(),
                payload: Box::new(V::Bool(true)),
            },
            2,
        ),
    ] {
        let definition = definition(body);
        let mut catalogue = Catalogue::new();
        catalogue
            .insert(("Owner", "1.0", "Thing"), &definition)
            .unwrap();
        for (depth, expected) in [(exact_depth, Ok(())), (exact_depth - 1, Err(E::Limit))] {
            let mut policy = limits();
            policy.value_depth = depth;
            assert_eq!(
                materialize(
                    &value,
                    &nominal(),
                    &catalogue,
                    &mut Budget::new(policy, &cancellation),
                    1
                )
                .map(|_| ()),
                expected
            );
        }
    }
}

/// Record traversal reserves both supplied input slots and declared output slots.
#[test]
fn record_materialization_reserves_input_and_output_work() {
    let values = vec![("x".to_owned(), Some(V::Bool(true)))];
    let fields = vec![field(T::Bool)];
    // Depth 1 + slots 2 + supplied visit 1 + key 4 + field visit 1 + scalar 3 + restrictions 1.
    work_boundary(13, |budget| {
        materialize_record(
            &values,
            &fields,
            &Catalogue::new(),
            budget,
            1,
            &|never, _, _| match *never {},
        )
        .map(|_| ())
    });
}

/// Default materialization applies interpretation depth after lifting the closed payload.
#[test]
fn record_default_materialization_charges_nullable_interpretation_depth() {
    let mut defaulted = field(T::Nullable(Box::new(T::Bool)));
    defaulted.presence = FieldPresence::Defaulted;
    defaulted.default = Some(V::Bool(true));
    let cancellation = CancellationToken::new();
    for (depth, expected) in [(3, Ok(())), (2, Err(E::Limit))] {
        let mut policy = limits();
        policy.value_depth = depth;
        assert_eq!(
            materialize_record::<std::convert::Infallible>(
                &[],
                std::slice::from_ref(&defaulted),
                &Catalogue::new(),
                &mut Budget::new(policy, &cancellation),
                1,
                &|never, _, _| match *never {}
            )
            .map(|_| ()),
            expected
        );
    }
}

/// Lifting checks list elements, present record values and variant payloads independently.
#[test]
fn lifting_closed_structures_charges_each_child_depth() {
    let cancellation = CancellationToken::new();
    for value in [
        V::List(vec![V::Bool(true)]),
        V::Record(vec![("x".to_owned(), Some(V::Bool(true)))]),
        V::Variant {
            tag: "Tag".to_owned(),
            payload: Box::new(V::Bool(true)),
        },
    ] {
        for (depth, expected) in [(2, Ok(())), (1, Err(E::Limit))] {
            let mut policy = limits();
            policy.value_depth = depth;
            assert_eq!(
                lift_closed::<ModuleSymbolIdentity>(
                    &value,
                    &mut Budget::new(policy, &cancellation),
                    1
                )
                .map(|_| ()),
                expected
            );
        }
    }
}
