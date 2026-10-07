// SPDX-License-Identifier: Apache-2.0

//! Both nominal origins share type closure, closed defaults, restrictions and value semantics.

use neutral_core::allocation::Shared as Arc;
use neutral_core::{
    CancellationToken, StructuralLimits, VocabularyContentDigest, profile::V1_SOURCE_PROFILE,
};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        ClosedValue as V, CompositionAlternative, CompositionBody, CompositionDefinition,
        CompositionField, FieldPresence, FieldRestrictions, SourceCompositionDefinition,
        ValueOriginKind as K,
    },
    project_interface::ProjectPublicType as T,
};
use neutral_vocabulary::{
    VocabularyLimits, VocabularyLock,
    composition::{
        self, CapturedCompositionBundle, CompositionError as E, CompositionLimits,
        ValidatedComposition, ValidatedCompositionScope, validate_composition_closure,
        validate_composition_scope,
    },
};

/// Creates fallible successor shared ownership for fixtures; allocation failure fails the test.
fn shared<T>(value: T) -> Arc<T> {
    Arc::try_new(value).unwrap()
}

/// Finite default test policy; individual tests narrow only the bound under review.
fn limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ))
}

/// Empty but validated vocabulary closure for source-only type scopes.
fn empty() -> Arc<ValidatedComposition> {
    shared(validate_composition_closure(&[], &[], limits(), &CancellationToken::new()).unwrap())
}

/// Exact source owner independent of package versions, paths or aliases.
fn owner(module: &str, name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new(V1_SOURCE_PROFILE, module), name)
}

/// Raw source definition using the same record/variant body as a vocabulary.
fn source(
    module: &str,
    name: &str,
    public: bool,
    body: CompositionBody,
) -> SourceCompositionDefinition {
    SourceCompositionDefinition {
        owner: owner(module, name),
        definition: CompositionDefinition {
            name: name.to_owned(),
            public,
            body,
        },
    }
}

/// Required typed field with no default or semantic restrictions.
fn field(name: &str, ty: T) -> CompositionField {
    CompositionField {
        name: name.to_owned(),
        ty,
        presence: FieldPresence::Required,
        restrictions: FieldRestrictions::default(),
        default: None,
    }
}

/// Unsorted heterogeneous source variant, deliberately requiring canonicalization.
fn outcome(module: &str, name: &str, public: bool) -> SourceCompositionDefinition {
    source(
        module,
        name,
        public,
        CompositionBody::Variant(vec![
            CompositionAlternative {
                tag: "text".to_owned(),
                ty: T::String,
            },
            CompositionAlternative {
                tag: "number".to_owned(),
                ty: T::Num,
            },
        ]),
    )
}

/// Validates source-owned declarations without any captured-source or compiler bypass.
fn scope(
    sources: Vec<SourceCompositionDefinition>,
    policy: CompositionLimits,
) -> Result<ValidatedCompositionScope, E> {
    validate_composition_scope(empty(), sources, policy, &CancellationToken::new())
}

/// Exact finite test number, never a host float or package revision.
fn number(value: &str) -> V {
    V::Number(ExactNumber::from_source(value, 64, 64).unwrap())
}

/// Contextual tagged payload independent of its nominal owner.
fn variant(tag: &str, value: V) -> V {
    V::Variant {
        tag: tag.to_owned(),
        payload: Box::new(value),
    }
}

/// Opens the already byte-pinned vocabulary variant fixture under exact /2 locks.
fn vocabulary() -> Arc<ValidatedComposition> {
    let bytes = include_bytes!("composition/fixtures/positive/variants.json");
    let lock = VocabularyLock::new(
        "Fixture",
        "1.0.0",
        composition::ENCODING_VERSION,
        composition::SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        vec![composition::REQUIRED_FEATURE.to_owned()],
    )
    .unwrap();
    shared(
        validate_composition_closure(
            &[CapturedCompositionBundle { bytes, lock: &lock }],
            &[("Fixture", "1.0.0")],
            limits(),
            &CancellationToken::new(),
        )
        .unwrap(),
    )
}

/// Exact vocabulary type tuple; source-local aliases cannot be supplied here.
fn vocabulary_type(name: &str) -> T {
    T::VocabularyNominal {
        identity: "Fixture".to_owned(),
        version: "1.0.0".to_owned(),
        name: name.to_owned(),
    }
}

/// Both origins use the identical payload validator and preserve heterogeneous list order.
#[test]
fn composition_scope_both_origins_share_variant_list_typing() {
    let list = V::List(vec![
        variant("text", V::String("ok".to_owned())),
        variant("number", number("3")),
    ]);
    let sources = vec![
        outcome("example", "Outcome", true),
        source(
            "example",
            "Item",
            true,
            CompositionBody::Record(vec![field(
                "values",
                T::List(Box::new(T::Nominal(owner("example", "Outcome")))),
            )]),
        ),
    ];
    let scope =
        validate_composition_scope(vocabulary(), sources, limits(), &CancellationToken::new())
            .unwrap();
    let value = V::Record(vec![("values".to_owned(), Some(list))]);
    let source = scope
        .materialize(
            &T::Nominal(owner("example", "Item")),
            &value,
            limits(),
            &CancellationToken::new(),
        )
        .unwrap();
    let vocabulary = scope
        .materialize(
            &vocabulary_type("Item"),
            &value,
            limits(),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(source, vocabulary);
    for invalid in [
        variant("unknown", number("3")),
        variant("text", number("3")),
        V::Record(vec![]),
    ] {
        for ty in [
            T::Nominal(owner("example", "Outcome")),
            vocabulary_type("Outcome"),
        ] {
            assert_eq!(
                scope.materialize(&ty, &invalid, limits(), &CancellationToken::new()),
                Err(E::InvalidValue)
            );
        }
    }
}

/// A source record can embed exact public vocabulary variants without changing owner identity.
#[test]
fn composition_scope_source_defaults_embed_vocabulary_variants() {
    let mut value = field("choice", vocabulary_type("Outcome"));
    value.presence = FieldPresence::Defaulted;
    value.default = Some(variant("text", V::String("ok".to_owned())));
    let scope = validate_composition_scope(
        vocabulary(),
        vec![source(
            "example",
            "Request",
            true,
            CompositionBody::Record(vec![value]),
        )],
        limits(),
        &CancellationToken::new(),
    )
    .unwrap();
    let ty = T::Nominal(owner("example", "Request"));
    let omitted = scope
        .materialize(&ty, &V::Record(vec![]), limits(), &CancellationToken::new())
        .unwrap();
    let supplied = scope
        .materialize(
            &ty,
            &V::Record(vec![(
                "choice".to_owned(),
                Some(variant("text", V::String("ok".to_owned()))),
            )]),
            limits(),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(omitted.value(), supplied.value());
    assert_ne!(omitted.origins(), supplied.origins());
    assert_eq!(omitted.origins()[1].kind, K::Defaulted);
    assert_eq!(omitted.origins()[2].kind, K::Defaulted);
    assert_eq!(supplied.origins()[1].kind, K::Supplied);
}

/// Definition, field and tag order cannot influence shared resolved-scope semantics.
#[test]
fn composition_scope_canonical_order_is_nonsemantic() {
    let sources = vec![
        outcome("zeta", "Outcome", true),
        source(
            "alpha",
            "Request",
            true,
            CompositionBody::Record(vec![field("zeta", T::Num), field("alpha", T::Bool)]),
        ),
    ];
    let expected = scope(sources.clone(), limits()).unwrap();
    let mut shuffled = sources;
    shuffled.reverse();
    for source in &mut shuffled {
        match &mut source.definition.body {
            CompositionBody::Record(fields) => fields.reverse(),
            CompositionBody::Variant(alternatives) => alternatives.reverse(),
        }
    }
    assert_eq!(scope(shuffled, limits()).unwrap(), expected);
    assert_eq!(expected.sources()[0].owner.module().module_name(), "alpha");
}

/// Empty/duplicate/protected alternatives reject before contextual value checking.
#[test]
fn composition_scope_rejects_invalid_variant_names_and_tags() {
    let baseline = outcome("example", "Outcome", true);
    for tags in [
        vec![],
        vec!["duplicate", "duplicate"],
        vec!["num"],
        vec!["Upper"],
        vec!["bad__tag"],
    ] {
        let mut invalid = baseline.clone();
        invalid.definition.body = CompositionBody::Variant(
            tags.into_iter()
                .map(|tag| CompositionAlternative {
                    tag: tag.to_owned(),
                    ty: T::Num,
                })
                .collect(),
        );
        assert_eq!(scope(vec![invalid], limits()), Err(E::InvalidContract));
    }
    assert_eq!(
        scope(vec![baseline.clone(), baseline], limits()),
        Err(E::InvalidContract)
    );
}

/// Owner components are checked, never trusted merely because they were assembled by constructors.
#[test]
fn composition_scope_rejects_forged_source_owners() {
    for invalid in [
        source(
            "bad::",
            "Outcome",
            true,
            CompositionBody::Variant(vec![CompositionAlternative {
                tag: "ok".to_owned(),
                ty: T::Bool,
            }]),
        ),
        SourceCompositionDefinition {
            owner: ModuleSymbolIdentity::new(
                LogicalModuleIdentity::new("unsupported", "example"),
                "Outcome",
            ),
            ..outcome("example", "Outcome", true)
        },
        SourceCompositionDefinition {
            owner: owner("example", "Other"),
            ..outcome("example", "Outcome", true)
        },
    ] {
        assert_eq!(scope(vec![invalid], limits()), Err(E::InvalidContract));
    }
}

/// All alternative payload branches participate in public closure, even if never selected.
#[test]
fn composition_scope_unselected_private_payload_is_not_public() {
    let hidden = outcome("example", "Hidden", false);
    for payload in [
        T::Nominal(hidden.owner.clone()),
        T::List(Box::new(T::Nominal(hidden.owner.clone()))),
        T::Nullable(Box::new(T::Ref(Box::new(T::Nominal(hidden.owner.clone()))))),
    ] {
        let exposed = source(
            "example",
            "Exposed",
            true,
            CompositionBody::Variant(vec![CompositionAlternative {
                tag: "hidden".to_owned(),
                ty: payload,
            }]),
        );
        assert_eq!(
            scope(vec![hidden.clone(), exposed], limits()),
            Err(E::PrivateType)
        );
    }
}

/// Private local types are legal internally but never as cross-module payload contracts.
#[test]
fn composition_scope_private_cross_module_types_fail() {
    let hidden = outcome("provider", "Hidden", false);
    for module in ["provider", "consumer"] {
        let outer = source(
            module,
            "Outer",
            false,
            CompositionBody::Record(vec![field("hidden", T::Nominal(hidden.owner.clone()))]),
        );
        let result = scope(vec![hidden.clone(), outer], limits());
        if module == "provider" {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(E::PrivateType));
        }
    }
}

/// Lists, nullable wrappers and unselected tags cannot conceal embedded source cycles.
#[test]
fn composition_scope_rejects_embedded_variant_record_cycles() {
    for ty in [
        T::Nominal(owner("example", "Item")),
        T::List(Box::new(T::Nominal(owner("example", "Item")))),
        T::Nullable(Box::new(T::Nominal(owner("example", "Item")))),
    ] {
        let variant = source(
            "example",
            "Outcome",
            true,
            CompositionBody::Variant(vec![CompositionAlternative {
                tag: "recursive".to_owned(),
                ty,
            }]),
        );
        let record = source(
            "example",
            "Item",
            true,
            CompositionBody::Record(vec![field("value", T::Nominal(variant.owner.clone()))]),
        );
        assert_eq!(
            scope(vec![variant, record], limits()),
            Err(E::EmbeddedCycle)
        );
    }
}

/// Nominal reference cycles do not embed values or trigger recursive default expansion.
#[test]
fn composition_scope_reference_cycles_are_nonembedding() {
    let target = T::Nominal(owner("example", "Outcome"));
    let source = source(
        "example",
        "Outcome",
        true,
        CompositionBody::Variant(vec![CompositionAlternative {
            tag: "next".to_owned(),
            ty: T::Nullable(Box::new(T::Ref(Box::new(target.clone())))),
        }]),
    );
    let scope = scope(vec![source], limits()).unwrap();
    assert!(
        scope
            .materialize(
                &target,
                &variant("next", V::Null),
                limits(),
                &CancellationToken::new()
            )
            .is_ok()
    );
    assert_eq!(
        scope.materialize(
            &target,
            &variant("next", variant("next", V::Null)),
            limits(),
            &CancellationToken::new()
        ),
        Err(E::InvalidValue)
    );
}

/// Raw malformed wrapper shapes and unresolved source/vocabulary owners cannot enter a scope.
#[test]
fn composition_scope_rejects_invalid_and_dangling_types() {
    for (ty, error) in [
        (T::Ref(Box::new(T::Num)), E::InvalidContract),
        (
            T::Nullable(Box::new(T::Nullable(Box::new(T::Bool)))),
            E::InvalidContract,
        ),
        (T::Nominal(owner("missing", "Absent")), E::UnknownType),
        (vocabulary_type("Outcome"), E::UnknownType),
    ] {
        let source = source(
            "example",
            "Request",
            true,
            CompositionBody::Record(vec![field("value", ty)]),
        );
        assert_eq!(scope(vec![source], limits()), Err(error));
    }
}

/// Source fields cannot manufacture vocabulary-only optional presence or contradictory defaults.
#[test]
fn composition_scope_rejects_invalid_presence_and_unused_defaults() {
    for (presence, default, error) in [
        (FieldPresence::Optional, None, E::InvalidContract),
        (
            FieldPresence::Required,
            Some(number("1")),
            E::InvalidContract,
        ),
        (FieldPresence::Defaulted, None, E::InvalidContract),
        (
            FieldPresence::Defaulted,
            Some(V::Bool(true)),
            E::InvalidDefault,
        ),
    ] {
        let mut value = field("value", T::Num);
        value.presence = presence;
        value.default = default;
        let source = source(
            "example",
            "Unused",
            false,
            CompositionBody::Record(vec![value]),
        );
        assert_eq!(scope(vec![source], limits()), Err(error));
    }
}

/// Inclusive exact restrictions and normalized finite choices use the vocabulary implementation.
#[test]
fn composition_scope_defaults_and_restrictions_share_exact_arithmetic() {
    let mut value = field("value", T::Num);
    value.presence = FieldPresence::Defaulted;
    value.default = Some(number("3"));
    value.restrictions.choices = Some(vec![number("3"), number("1")]);
    value.restrictions.minimum = Some(ExactNumber::from_source("1", 64, 64).unwrap());
    value.restrictions.maximum = Some(ExactNumber::from_source("3", 64, 64).unwrap());
    let valid = source(
        "example",
        "Request",
        true,
        CompositionBody::Record(vec![value.clone()]),
    );
    let scope = scope(vec![valid.clone()], limits()).unwrap();
    let ty = T::Nominal(valid.owner);
    assert!(
        scope
            .materialize(&ty, &V::Record(vec![]), limits(), &CancellationToken::new())
            .is_ok()
    );
    for literal in ["0", "2", "4"] {
        assert_eq!(
            scope.materialize(
                &ty,
                &V::Record(vec![("value".to_owned(), Some(number(literal)))]),
                limits(),
                &CancellationToken::new()
            ),
            Err(E::InvalidValue)
        );
    }
    value.default = Some(number("2"));
    assert_eq!(
        validate_composition_scope(
            empty(),
            vec![source(
                "example",
                "Unused",
                false,
                CompositionBody::Record(vec![value])
            )],
            limits(),
            &CancellationToken::new()
        ),
        Err(E::InvalidDefault)
    );
}

/// Project depth counts nominal record children once, and nullable adds visits but not depth.
#[test]
fn composition_scope_project_depth_exact_and_one_over() {
    let sources = vec![
        source(
            "example",
            "Inner",
            true,
            CompositionBody::Record(vec![field("value", T::Nullable(Box::new(T::Bool)))]),
        ),
        source(
            "example",
            "Outer",
            true,
            CompositionBody::Record(vec![field("inner", T::Nominal(owner("example", "Inner")))]),
        ),
    ];
    let scope = scope(sources, limits()).unwrap();
    let value = V::Record(vec![(
        "inner".to_owned(),
        Some(V::Record(vec![("value".to_owned(), Some(V::Bool(true)))])),
    )]);
    let mut policy = limits();
    policy.value_depth = 2;
    assert!(
        scope
            .materialize(
                &T::Nominal(owner("example", "Outer")),
                &value,
                policy,
                &CancellationToken::new()
            )
            .is_ok()
    );
    policy.value_depth = 1;
    assert_eq!(
        scope.materialize(
            &T::Nominal(owner("example", "Outer")),
            &value,
            policy,
            &CancellationToken::new()
        ),
        Err(E::Limit)
    );
    policy = limits();
    policy.value_nodes = 4;
    assert!(
        scope
            .materialize(
                &T::Nominal(owner("example", "Outer")),
                &value,
                policy,
                &CancellationToken::new()
            )
            .is_ok()
    );
    policy.value_nodes = 3;
    assert_eq!(
        scope.materialize(
            &T::Nominal(owner("example", "Outer")),
            &value,
            policy,
            &CancellationToken::new()
        ),
        Err(E::Limit)
    );
}

/// Source and vocabulary declarations share one aggregate count without alias double charging.
#[test]
fn composition_scope_mixed_aggregate_exact_and_one_over() {
    let mut policy = limits();
    policy.total_types = 3;
    policy.total_fields = 1;
    policy.total_alternatives = 4;
    assert!(
        validate_composition_scope(
            vocabulary(),
            vec![outcome("example", "Outcome", true)],
            policy,
            &CancellationToken::new()
        )
        .is_ok()
    );
    policy.total_types = 2;
    assert_eq!(
        validate_composition_scope(
            vocabulary(),
            vec![outcome("example", "Outcome", true)],
            policy,
            &CancellationToken::new()
        ),
        Err(E::Limit)
    );
    policy = limits();
    policy.total_alternatives = 3;
    assert_eq!(
        validate_composition_scope(
            vocabulary(),
            vec![outcome("example", "Outcome", true)],
            policy,
            &CancellationToken::new()
        ),
        Err(E::Limit)
    );
}

/// Every source variant alternative is independently bounded at exact/one-over limits.
#[test]
fn composition_scope_alternative_limit_exact_and_one_over() {
    let mut policy = limits();
    policy.alternatives_per_type = 2;
    assert!(scope(vec![outcome("example", "Outcome", true)], policy).is_ok());
    policy.alternatives_per_type = 1;
    assert_eq!(
        scope(vec![outcome("example", "Outcome", true)], policy),
        Err(E::Limit)
    );
}

/// Choice count bounds apply before canonicalization and reject duplicate normalized meaning.
#[test]
fn composition_scope_choice_bounds_and_duplicates() {
    let mut value = field("value", T::Num);
    value.restrictions.choices = Some(vec![number("1"), number("3")]);
    let mut policy = limits();
    policy.choices_per_field = 2;
    policy.total_choices = 2;
    let sources = vec![source(
        "example",
        "Request",
        true,
        CompositionBody::Record(vec![value.clone()]),
    )];
    assert!(scope(sources.clone(), policy).is_ok());
    policy.total_choices = 1;
    assert_eq!(scope(sources, policy), Err(E::Limit));
    value.restrictions.choices = Some(vec![number("1"), number("1.0")]);
    assert_eq!(
        scope(
            vec![source(
                "example",
                "Request",
                true,
                CompositionBody::Record(vec![value])
            )],
            limits()
        ),
        Err(E::DuplicateChoice)
    );
}

/// Deep raw types and defaults fail before recursive consumers can expand them.
#[test]
fn composition_scope_raw_depth_and_work_bounds() {
    let mut ty = T::Bool;
    let mut value = V::Bool(true);
    for _ in 0..8 {
        ty = T::List(Box::new(ty));
        value = V::List(vec![value]);
    }
    let mut field = field("value", ty);
    field.presence = FieldPresence::Defaulted;
    field.default = Some(value);
    let sources = vec![source(
        "example",
        "Unused",
        false,
        CompositionBody::Record(vec![field]),
    )];
    let mut policy = limits();
    policy.type_depth = 7;
    assert_eq!(scope(sources.clone(), policy), Err(E::Limit));
    policy = limits();
    policy.value_depth = 7;
    assert_eq!(scope(sources.clone(), policy), Err(E::Limit));
    policy = limits();
    policy.work = 1;
    assert_eq!(scope(sources, policy), Err(E::Limit));
}

/// Zero controls precede cancellation; cancellation never returns a partial scope or value.
#[test]
fn composition_scope_cancellation_precedence_and_publication() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut policy = limits();
    policy.total_types = 0;
    assert_eq!(
        validate_composition_scope(empty(), vec![], policy, &cancellation),
        Err(E::InvalidLimits)
    );
    assert_eq!(
        validate_composition_scope(empty(), vec![], limits(), &cancellation),
        Err(E::Cancelled)
    );
    let scope = scope(vec![outcome("example", "Outcome", true)], limits()).unwrap();
    assert_eq!(
        scope.materialize(
            &T::Nominal(owner("example", "Outcome")),
            &variant("text", V::String("ok".to_owned())),
            limits(),
            &cancellation
        ),
        Err(E::Cancelled)
    );
}

/// Private and missing roots have the same safe materialization failure and debug redaction.
#[test]
fn composition_scope_private_roots_and_debug_are_redacted() {
    let scope = scope(
        vec![outcome("private_module", "PrivateSecret", false)],
        limits(),
    )
    .unwrap();
    for name in ["PrivateSecret", "Absent"] {
        assert_eq!(
            scope.materialize(
                &T::Nominal(owner("private_module", name)),
                &variant("text", V::String("ok".to_owned())),
                limits(),
                &CancellationToken::new()
            ),
            Err(E::UnknownType)
        );
    }
    let debug = format!("{scope:?}");
    assert!(!debug.contains("PrivateSecret"));
    assert!(!debug.contains("private_module"));
}

/// Concurrent calls retain request-local work, origin and cancellation state.
#[test]
fn composition_scope_concurrent_materialization_is_isolated() {
    let scope = shared(scope(vec![outcome("example", "Outcome", true)], limits()).unwrap());
    std::thread::scope(|threads| {
        let handles: Vec<_> = (0..8)
            .map(|index| {
                let scope = Arc::clone(&scope);
                threads.spawn(move || {
                    let cancellation = CancellationToken::new();
                    if index % 2 == 0 {
                        cancellation.cancel();
                    }
                    scope.materialize(
                        &T::Nominal(owner("example", "Outcome")),
                        &variant("number", number("3")),
                        limits(),
                        &cancellation,
                    )
                })
            })
            .collect();
        for (index, handle) in handles.into_iter().enumerate() {
            let result = handle.join().unwrap();
            if index % 2 == 0 {
                assert_eq!(result, Err(E::Cancelled));
            } else {
                assert!(result.is_ok());
            }
        }
    });
}

/// Per-module type limits count each source owner once, separately from aggregate limits.
#[test]
fn composition_scope_per_module_type_limit_exact_and_one_over() {
    let mut policy = limits();
    policy.json = policy.json.with_types(1).unwrap();
    assert!(
        scope(
            vec![
                outcome("alpha", "Outcome", true),
                outcome("beta", "Outcome", true)
            ],
            policy
        )
        .is_ok()
    );
    assert_eq!(
        scope(
            vec![
                outcome("alpha", "Outcome", true),
                outcome("alpha", "Other", true)
            ],
            policy
        ),
        Err(E::Limit)
    );
    assert_eq!(
        validate_composition_scope(vocabulary(), vec![], policy, &CancellationToken::new()),
        Err(E::Limit)
    );
}

/// Semantic Unicode/list lengths do not replace structural byte or item limits.
#[test]
fn composition_scope_string_and_list_restrictions() {
    let mut label = field("label", T::String);
    label.restrictions.min_length = Some(2);
    label.restrictions.max_length = Some(2);
    let mut values = field("values", T::List(Box::new(T::Num)));
    values.restrictions.min_length = Some(1);
    values.restrictions.max_length = Some(1);
    let scope = scope(
        vec![source(
            "example",
            "Request",
            true,
            CompositionBody::Record(vec![label, values]),
        )],
        limits(),
    )
    .unwrap();
    let ty = T::Nominal(owner("example", "Request"));
    let valid = V::Record(vec![
        ("label".to_owned(), Some(V::String("é猫".to_owned()))),
        ("values".to_owned(), Some(V::List(vec![number("3")]))),
    ]);
    assert!(
        scope
            .materialize(&ty, &valid, limits(), &CancellationToken::new())
            .is_ok()
    );
    for (text, values) in [
        ("é", vec![number("3")]),
        ("é猫x", vec![number("3")]),
        ("é猫", vec![]),
        ("é猫", vec![number("3"), number("3")]),
    ] {
        let invalid = V::Record(vec![
            ("label".to_owned(), Some(V::String(text.to_owned()))),
            ("values".to_owned(), Some(V::List(values))),
        ]);
        assert_eq!(
            scope.materialize(&ty, &invalid, limits(), &CancellationToken::new()),
            Err(E::InvalidValue)
        );
    }
}

/// Every unused closed default consumes the cumulative visit budget before publication.
#[test]
fn composition_scope_unused_default_visit_limit_exact_and_one_over() {
    let mut value = field("value", T::Nullable(Box::new(T::Bool)));
    value.presence = FieldPresence::Defaulted;
    value.default = Some(V::Bool(true));
    let sources = vec![
        source(
            "example",
            "First",
            false,
            CompositionBody::Record(vec![value.clone()]),
        ),
        source(
            "example",
            "Second",
            false,
            CompositionBody::Record(vec![value]),
        ),
    ];
    let mut policy = limits();
    policy.value_nodes = 4;
    assert!(scope(sources.clone(), policy).is_ok());
    policy.value_nodes = 3;
    assert_eq!(scope(sources, policy), Err(E::Limit));
}

/// Duplicate field keys and wrong nominal defaults reject even on private unused records.
#[test]
fn composition_scope_duplicate_fields_and_unknown_default_tags() {
    assert_eq!(
        scope(
            vec![source(
                "example",
                "Unused",
                false,
                CompositionBody::Record(vec![field("value", T::Bool), field("value", T::Bool)])
            )],
            limits()
        ),
        Err(E::InvalidContract)
    );
    let mut value = field("value", T::Nominal(owner("example", "Outcome")));
    value.presence = FieldPresence::Defaulted;
    value.default = Some(variant("unknown", V::Bool(true)));
    assert_eq!(
        scope(
            vec![
                outcome("example", "Outcome", true),
                source(
                    "example",
                    "Unused",
                    false,
                    CompositionBody::Record(vec![value])
                )
            ],
            limits()
        ),
        Err(E::InvalidDefault)
    );
}
