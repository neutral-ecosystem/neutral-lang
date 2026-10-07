// SPDX-License-Identifier: Apache-2.0

//! Resolved binding values share closed defaults, constraints and invariant identity-only references.

use neutral_core::{
    CancellationToken, StructuralLimits, VocabularyContentDigest, profile::V1_SOURCE_PROFILE,
};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        BindingValue as V, ClosedValue as C, CompositionAlternative, CompositionBinding,
        CompositionBody, CompositionDefinition, CompositionField, FieldPresence, FieldRestrictions,
        SourceCompositionDefinition, ValueOriginKind as K, ValuePathSegment as P,
    },
    project_interface::ProjectPublicType as T,
};
use neutral_vocabulary::{
    VocabularyLimits, VocabularyLock,
    composition::{
        CapturedCompositionBundle, CompositionError as E, CompositionLimits,
        ValidatedCompositionBindings, ValidatedCompositionScope, validate_composition_bindings,
        validate_composition_closure, validate_composition_scope,
    },
};
use std::sync::Arc;

/// Common independent finite policy; cases narrow only the bound they exercise.
fn limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ))
}

/// Alias-independent exact source owner.
fn owner(module: &str, name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new(V1_SOURCE_PROFILE, module), name)
}

/// One raw record/variant contract without a compiler or byte decoder dependency.
fn source(name: &str, public: bool, body: CompositionBody) -> SourceCompositionDefinition {
    SourceCompositionDefinition {
        owner: owner("example", name),
        definition: CompositionDefinition {
            name: name.to_owned(),
            public,
            body,
        },
    }
}

/// Closed constrained default; reference-capable values cannot be assigned here.
fn attempts() -> CompositionField {
    let number = ExactNumber::from_source("3", 64, 64).unwrap();
    CompositionField {
        name: "attempts".to_owned(),
        ty: T::Num,
        presence: FieldPresence::Defaulted,
        restrictions: FieldRestrictions {
            choices: Some(vec![C::Number(number.clone())]),
            minimum: Some(number.clone()),
            maximum: Some(number.clone()),
            ..FieldRestrictions::default()
        },
        default: Some(C::Number(number)),
    }
}

/// A public recursive identity-only type and a separate private unused type.
fn scope() -> Arc<ValidatedCompositionScope> {
    let catalogue = Arc::new(
        validate_composition_closure(&[], &[], limits(), &CancellationToken::new()).unwrap(),
    );
    let definitions = vec![
        source(
            "Node",
            true,
            CompositionBody::Record(vec![
                attempts(),
                CompositionField {
                    name: "next".to_owned(),
                    ty: T::Nullable(Box::new(T::Ref(Box::new(T::Nominal(owner(
                        "example", "Node",
                    )))))),
                    presence: FieldPresence::Required,
                    restrictions: FieldRestrictions::default(),
                    default: None,
                },
            ]),
        ),
        source(
            "Outcome",
            true,
            CompositionBody::Variant(vec![CompositionAlternative {
                tag: "next".to_owned(),
                ty: T::List(Box::new(T::Nullable(Box::new(T::Ref(Box::new(
                    T::Nominal(owner("example", "Node")),
                )))))),
            }]),
        ),
        source("Private", false, CompositionBody::Record(vec![])),
    ];
    Arc::new(
        validate_composition_scope(catalogue, definitions, limits(), &CancellationToken::new())
            .unwrap(),
    )
}

/// Raw resolved immutable binding with explicitly selected public visibility.
fn binding(name: &str, public: bool, ty: T, value: V) -> CompositionBinding {
    CompositionBinding {
        owner: owner("example", name),
        public,
        ty,
        value,
    }
}

/// Missing attempts materialize a closed constrained default while next remains explicit.
fn node(next: V) -> V {
    V::Record(vec![("next".to_owned(), Some(next))])
}

/// Two forward/cyclic reference targets are checked without evaluating either target value.
fn cycle() -> Vec<CompositionBinding> {
    vec![
        binding(
            "right",
            true,
            T::Nominal(owner("example", "Node")),
            node(V::Reference(owner("example", "left"))),
        ),
        binding(
            "left",
            true,
            T::Nominal(owner("example", "Node")),
            node(V::Reference(owner("example", "right"))),
        ),
    ]
}

/// Applies whole-request validation with isolated cancellation and policy.
fn validate(
    bindings: &[CompositionBinding],
    policy: CompositionLimits,
) -> Result<ValidatedCompositionBindings, E> {
    validate_composition_bindings(scope(), bindings, policy, &CancellationToken::new())
}

/// Non-embedding cycles, forward references and input permutation produce canonical facts.
#[test]
fn composition_bindings_forward_cycles_and_canonical_order() {
    let mut input = cycle();
    let result = validate(&input, limits()).unwrap();
    input.reverse();
    assert_eq!(result, validate(&input, limits()).unwrap());
    assert_eq!(
        result.bindings()[0].binding().owner,
        owner("example", "left")
    );
    assert_eq!(
        result.bindings()[0].references()[0].path,
        [P::Field("next".to_owned())]
    );
    assert_eq!(
        result.bindings()[0].references()[0].target,
        owner("example", "right")
    );
    assert!(
        result.bindings()[0]
            .origins()
            .iter()
            .any(|origin| origin.path == [P::Field("attempts".to_owned())]
                && origin.kind == K::Defaulted)
    );
    assert!(!format!("{result:?}").contains("left"));
    assert!(!format!("{:?}", result.bindings()[0]).contains("right"));
}

/// Actual edges include only selected non-null occurrences through payload/list/nullable wrappers.
#[test]
fn composition_bindings_nested_reference_paths() {
    let mut bindings = cycle();
    bindings.push(binding(
        "selected",
        true,
        T::Nominal(owner("example", "Outcome")),
        V::Variant {
            tag: "next".to_owned(),
            payload: Box::new(V::List(vec![
                V::Null,
                V::Reference(owner("example", "right")),
                V::Reference(owner("example", "left")),
            ])),
        },
    ));
    let result = validate(&bindings, limits()).unwrap();
    let selected = &result.bindings()[2];
    assert_eq!(selected.references().len(), 2);
    assert_eq!(selected.references()[0].path, [P::Payload, P::Element(1)]);
    assert_eq!(selected.references()[1].path, [P::Payload, P::Element(2)]);
    assert!(
        selected
            .origins()
            .iter()
            .any(|origin| origin.path == [P::Payload, P::Element(0)]
                && origin.kind == K::ExplicitNull)
    );
}

/// Dangling, differently owned/kinded/typed and private public targets fail atomically.
#[test]
fn composition_bindings_reference_target_failures() {
    let mut dangling = cycle();
    dangling[0].value = node(V::Reference(owner("example", "absent")));
    assert_eq!(validate(&dangling, limits()), Err(E::InvalidValue));
    let mut mismatch = cycle();
    mismatch[0].ty = T::Nominal(owner("example", "Outcome"));
    mismatch[0].value = V::Variant {
        tag: "next".to_owned(),
        payload: Box::new(V::List(vec![])),
    };
    assert_eq!(validate(&mismatch, limits()), Err(E::InvalidValue));
    let mut private = cycle();
    private[0].public = false;
    assert_eq!(validate(&private, limits()), Err(E::PrivateType));
    private[1].public = false;
    assert!(validate(&private, limits()).is_ok());
    private[0].owner = owner("other", "right");
    private[1].value = node(V::Reference(owner("other", "right")));
    assert_eq!(validate(&private, limits()), Err(E::PrivateType));
}

/// Closed defaults, exact scalar restrictions and required fields remain enforced on binding values.
#[test]
fn composition_bindings_restrictions_and_required_fields() {
    let mut input = cycle();
    if let V::Record(fields) = &mut input[0].value {
        fields.push((
            "attempts".to_owned(),
            Some(V::Number(ExactNumber::from_source("4", 64, 64).unwrap())),
        ));
    }
    assert_eq!(validate(&input, limits()), Err(E::InvalidValue));
    input[0].value = V::Record(vec![]);
    assert_eq!(validate(&input, limits()), Err(E::InvalidValue));
    input[0].value = node(V::Null);
    let result = validate(&input, limits()).unwrap();
    assert_eq!(result.bindings()[1].references(), []);
}

/// Public/private type closure cannot be bypassed through a binding or type wrapper.
#[test]
fn composition_bindings_private_type_and_bad_owner_failures() {
    for ty in [
        T::Nominal(owner("example", "Private")),
        T::List(Box::new(T::Nominal(owner("example", "Private")))),
    ] {
        assert_eq!(
            validate(&[binding("secret", true, ty, V::Record(vec![]))], limits()),
            Err(E::PrivateType)
        );
    }
    let mut input = cycle();
    input[0].owner = owner("example", "InvalidBinding");
    assert_eq!(validate(&input, limits()), Err(E::InvalidContract));
    input[0] = input[1].clone();
    assert_eq!(validate(&input, limits()), Err(E::InvalidContract));
    input[0].ty = T::Ref(Box::new(T::Num));
    assert_eq!(validate(&input, limits()), Err(E::InvalidContract));
}

/// Aggregate bounds apply across bindings, including unused defaults, before partial publication.
#[test]
fn composition_bindings_limits_and_cancellation() {
    let input = cycle();
    let mut policy = limits();
    policy.value_nodes = 1;
    assert_eq!(validate(&input, policy), Err(E::Limit));
    policy = limits();
    policy.work = 1;
    assert_eq!(validate(&input, policy), Err(E::Limit));
    policy = limits();
    policy.total_fields = 1;
    assert_eq!(validate(&input, policy), Err(E::Limit));
    policy = limits();
    policy.type_depth = 1;
    assert_eq!(validate(&input, policy), Err(E::Limit));
    policy = limits();
    policy.work = 0;
    assert_eq!(validate(&input, policy), Err(E::InvalidLimits));
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert_eq!(
        validate_composition_bindings(scope(), &input, limits(), &cancellation),
        Err(E::Cancelled)
    );
    // An invalid request cannot poison a subsequent independent request.
    assert!(validate(&input, limits()).is_ok());
}

/// Exact vocabulary nominal identity without source-local aliases or owner conversion.
fn domain(name: &str) -> T {
    T::VocabularyNominal {
        identity: "ExampleDomain".to_owned(),
        version: "1.0.0".to_owned(),
        name: name.to_owned(),
    }
}

/// Validates the literal registered vocabulary with required references and constrained defaults.
fn domain_scope() -> Arc<ValidatedCompositionScope> {
    let bytes = include_bytes!("composition/fixtures/bundle.json");
    let lock = VocabularyLock::new(
        "ExampleDomain",
        "1.0.0",
        neutral_vocabulary::composition::ENCODING_VERSION,
        neutral_vocabulary::composition::SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        vec![neutral_vocabulary::composition::REQUIRED_FEATURE.to_owned()],
    )
    .unwrap();
    let cancellation = CancellationToken::new();
    let catalogue = Arc::new(
        validate_composition_closure(
            &[CapturedCompositionBundle { bytes, lock: &lock }],
            &[("ExampleDomain", "1.0.0")],
            limits(),
            &cancellation,
        )
        .unwrap(),
    );
    Arc::new(
        validate_composition_scope(
            catalogue,
            vec![source(
                "Outcome",
                true,
                CompositionBody::Variant(vec![CompositionAlternative {
                    tag: "success".to_owned(),
                    ty: T::String,
                }]),
            )],
            limits(),
            &cancellation,
        )
        .unwrap(),
    )
}

/// Raw supplied vocabulary value with a complete forward reference into the same binding index.
fn domain_bindings() -> Vec<CompositionBinding> {
    vec![
        binding(
            "request",
            true,
            domain("Request"),
            V::Record(vec![(
                "selected".to_owned(),
                Some(V::Reference(owner("example", "choice"))),
            )]),
        ),
        binding(
            "choice",
            true,
            domain("Outcome"),
            V::Variant {
                tag: "success".to_owned(),
                payload: Box::new(V::String("complete".to_owned())),
            },
        ),
    ]
}

/// Reference-capable vocabulary records still materialize closed defaults and preserve optional absence.
#[test]
fn composition_bindings_vocabulary_defaults_omission_and_reference() {
    let input = domain_bindings();
    let result =
        validate_composition_bindings(domain_scope(), &input, limits(), &CancellationToken::new())
            .unwrap();
    let request = &result.bindings()[1];
    assert_eq!(
        request.references()[0].path,
        [P::Field("selected".to_owned())]
    );
    assert_eq!(request.references()[0].target, owner("example", "choice"));
    assert!(
        request
            .origins()
            .iter()
            .any(|origin| origin.path == [P::Field("label".to_owned())]
                && origin.kind == K::OmittedOptional)
    );
    assert!(request.origins().iter().any(|origin| origin.path
        == [P::Field("outcomes".to_owned()), P::Element(0), P::Payload]
        && origin.kind == K::Defaulted));
    let mut explicit = input;
    if let V::Record(fields) = &mut explicit[0].value {
        fields.push(("label".to_owned(), Some(V::Null)));
    }
    let explicit = validate_composition_bindings(
        domain_scope(),
        &explicit,
        limits(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert!(
        explicit.bindings()[1]
            .origins()
            .iter()
            .any(|origin| origin.path == [P::Field("label".to_owned())]
                && origin.kind == K::ExplicitNull)
    );
    assert_ne!(
        request.binding().value,
        explicit.bindings()[1].binding().value
    );
}

/// Similar source and vocabulary shapes cannot coerce nominal reference ownership.
#[test]
fn composition_bindings_cross_origin_reference_is_invariant() {
    let mut input = domain_bindings();
    input[1].ty = T::Nominal(owner("example", "Outcome"));
    assert_eq!(
        validate_composition_bindings(domain_scope(), &input, limits(), &CancellationToken::new()),
        Err(E::InvalidValue)
    );
}

/// Exact numeric choices and Unicode-scalar/list length restrictions reject one-over values.
#[test]
fn composition_bindings_vocabulary_restriction_boundaries() {
    for (field, accepted, rejected) in [
        (
            "label",
            V::String("é".repeat(32)),
            V::String("é".repeat(33)),
        ),
        (
            "outcomes",
            V::List(vec![
                V::Variant {
                    tag: "success".to_owned(),
                    payload: Box::new(V::String("ok".to_owned()))
                };
                16
            ]),
            V::List(vec![
                V::Variant {
                    tag: "success".to_owned(),
                    payload: Box::new(V::String("ok".to_owned()))
                };
                17
            ]),
        ),
        (
            "attempts",
            V::Number(ExactNumber::from_source("3", 64, 64).unwrap()),
            V::Number(ExactNumber::from_source("2", 64, 64).unwrap()),
        ),
    ] {
        for (value, expected) in [(accepted, true), (rejected, false)] {
            let mut input = domain_bindings();
            if let V::Record(fields) = &mut input[0].value {
                fields.push((field.to_owned(), Some(value)));
            }
            let result = validate_composition_bindings(
                domain_scope(),
                &input,
                limits(),
                &CancellationToken::new(),
            );
            if expected {
                assert!(result.is_ok(), "{field}: {result:?}");
            } else {
                assert_eq!(result, Err(E::InvalidValue), "{field}");
            }
        }
    }
}

/// Root-zero occurrence depth and cumulative materialization visits accept exactly their required budget.
#[test]
fn composition_bindings_exact_and_one_under_limits() {
    let mut input = cycle();
    input.push(binding(
        "selected",
        true,
        T::Nominal(owner("example", "Outcome")),
        V::Variant {
            tag: "next".to_owned(),
            payload: Box::new(V::List(vec![V::Reference(owner("example", "left"))])),
        },
    ));
    let mut policy = limits();
    policy.value_depth = 2;
    assert!(validate(&input, policy).is_ok());
    policy.value_depth = 1;
    assert_eq!(validate(&input, policy), Err(E::Limit));
    // Find the smallest passing visit budget independently of implementation counters.
    let minimum = (1..128)
        .find(|nodes| {
            let mut policy = limits();
            policy.value_nodes = *nodes;
            validate(&input, policy).is_ok()
        })
        .unwrap();
    let mut policy = limits();
    policy.value_nodes = minimum;
    assert!(validate(&input, policy).is_ok());
    policy.value_nodes = minimum - 1;
    assert_eq!(validate(&input, policy), Err(E::Limit));
}

/// Immutable catalogues and isolated request counters remain deterministic under concurrent consumers.
#[test]
fn composition_bindings_concurrent_isolation() {
    let scope = scope();
    let input = Arc::new(cycle());
    let expected = validate_composition_bindings(
        Arc::clone(&scope),
        &input,
        limits(),
        &CancellationToken::new(),
    )
    .unwrap();
    let workers = (0..4)
        .map(|_| {
            let scope = Arc::clone(&scope);
            let input = Arc::clone(&input);
            std::thread::spawn(move || {
                validate_composition_bindings(scope, &input, limits(), &CancellationToken::new())
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), expected);
    }
}
