// SPDX-License-Identifier: Apache-2.0

//! Literal catalogue cases and the shared supplied-value/origin boundary.

use neutral_core::{CancellationToken, StructuralLimits, VocabularyContentDigest};
use neutral_ir::{
    ExactNumber,
    composition::{ClosedValue as V, ValueOriginKind as K, ValuePathSegment as P},
};
use neutral_vocabulary::{
    VocabularyLimits, VocabularyLock,
    composition::{
        self, CapturedCompositionBundle, CompositionError as E, CompositionLimits,
        ValidatedComposition, ValidatedCompositionValue, materialize_composition_value,
        validate_composition_closure,
    },
};

/// Captured runtime-owned presence/default fixture, independent of portable plan location.
const PRESENCE: &[u8] = include_bytes!("composition/fixtures/positive/presence.json");
/// Captured runtime-owned variant/heterogeneous-payload fixture.
const VARIANTS: &[u8] = include_bytes!("composition/fixtures/positive/variants.json");
/// Canonical owner and exact semantic revision in literal inputs.
const OWNER: (&str, &str, &str) = ("Fixture", "1.0.0", "Item");

/// Supplies generous finite policy for semantic cases; tests narrow individual budgets separately.
fn limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ))
}

/// Validates exact captured bytes under an explicit new or legacy schema lock.
fn catalogue(
    bytes: &[u8],
    legacy: bool,
    policy: CompositionLimits,
) -> Result<ValidatedComposition, E> {
    let schema = if legacy {
        neutral_vocabulary::PROJECT_VOCABULARY_SCHEMA_VERSION
    } else {
        composition::SCHEMA_VERSION
    };
    let features = if legacy {
        Vec::new()
    } else {
        vec![composition::REQUIRED_FEATURE.to_owned()]
    };
    let lock = VocabularyLock::new(
        OWNER.0,
        OWNER.1,
        composition::ENCODING_VERSION,
        schema,
        VocabularyContentDigest::from_bytes(bytes),
        features,
    )
    .unwrap();
    validate_composition_closure(
        &[CapturedCompositionBundle { bytes, lock: &lock }],
        &[(OWNER.0, OWNER.1)],
        policy,
        &CancellationToken::new(),
    )
}

/// Materializes a supplied record under the complete presence fixture.
fn presence(value: &V, policy: CompositionLimits) -> Result<ValidatedCompositionValue, E> {
    materialize_composition_value(
        &catalogue(PRESENCE, false, limits()).unwrap(),
        OWNER,
        value,
        policy,
        &CancellationToken::new(),
    )
}

/// Builds one exact number without tying semantic literals to package `SemVer`.
fn number(value: &str) -> V {
    V::Number(ExactNumber::from_source(value, 64, 64).unwrap())
}

/// Literal positive, negative, boundary and migration bytes match their registered outcomes.
#[test]
fn composition_literal_fixture_oracles_and_hashes_are_fixed() {
    let cases: &[(&[u8], &str, Option<E>, bool)] = &[
        (
            PRESENCE,
            "f79072dee04a81d34d915daa93040753a28ee63121d07c81298b8b905d8a8450",
            None,
            false,
        ),
        (
            VARIANTS,
            "369ecaa4cd385bcdffa12b46886ca2e2d0dcd955732cebd49242b0c3c3d031e6",
            None,
            false,
        ),
        (
            include_bytes!("composition/fixtures/negative/duplicate-choice.json"),
            "e895501697de85ab40524d5da83e01662aa801a5446218f73e8d63c60a592e8c",
            Some(E::DuplicateChoice),
            false,
        ),
        (
            include_bytes!("composition/fixtures/negative/default-outside-bounds.json"),
            "e3f3883673ee4c9c390dfd33fcc9ed8659e25087f3b29747562d572f54514f37",
            Some(E::InvalidDefault),
            false,
        ),
        (
            include_bytes!("composition/fixtures/negative/unknown-tag.json"),
            "25f155995993f532f404916ce3e93ba050b206bb64260eae476f7af6ded2d3d2",
            Some(E::InvalidDefault),
            false,
        ),
        (
            include_bytes!("composition/fixtures/negative/public-private-closure.json"),
            "b597efe992a603a0a86a4b783d157c6db47005105164951a4822d32032e15204",
            Some(E::PrivateType),
            false,
        ),
        (
            include_bytes!("composition/fixtures/boundary/alternatives.json"),
            "437e8b767037bd8c2eae58885f3dcf26ead4485d6f501b16bbbd9d67597dc7dc",
            None,
            false,
        ),
        (
            include_bytes!("composition/fixtures/migration/legacy-leaf.json"),
            "cd8a2bb3e4244461270ff2fc418cf1bb1898a295379fbd8d7d576676404c23f6",
            None,
            true,
        ),
    ];
    for (bytes, hash, expected, legacy) in cases {
        assert_eq!(
            VocabularyContentDigest::from_bytes(bytes),
            VocabularyContentDigest::parse_text(&format!("sha256:{hash}")).unwrap()
        );
        let result = catalogue(bytes, *legacy, limits());
        if let Some(expected) = expected {
            assert_eq!(result, Err(expected.clone()));
        } else {
            assert!(result.is_ok());
        }
    }
}

/// A declared independent variant alternative limit accepts exact count and rejects one over.
#[test]
fn composition_literal_alternative_boundary_is_exact() {
    let bytes = include_bytes!("composition/fixtures/boundary/alternatives.json");
    assert!(
        catalogue(
            bytes,
            false,
            CompositionLimits {
                alternatives_per_type: 2,
                ..limits()
            }
        )
        .is_ok()
    );
    assert_eq!(
        catalogue(
            bytes,
            false,
            CompositionLimits {
                alternatives_per_type: 1,
                ..limits()
            }
        ),
        Err(E::Limit)
    );
}

/// Equal supplied/defaulted meaning retains separate occurrence evidence and optional absence.
#[test]
fn composition_supplied_and_defaulted_values_share_meaning_not_origins() {
    let omitted = presence(&V::Record(vec![]), limits()).unwrap();
    let explicit = presence(
        &V::Record(vec![("count".into(), Some(number("3.0")))]),
        limits(),
    )
    .unwrap();
    assert_eq!(omitted.value(), explicit.value());
    assert_ne!(omitted.origins(), explicit.origins());
    assert_eq!(
        omitted.value(),
        &V::Record(vec![
            ("count".into(), Some(number("3"))),
            ("label".into(), None)
        ])
    );
    assert_eq!(
        omitted
            .origins()
            .iter()
            .map(|origin| origin.kind)
            .collect::<Vec<_>>(),
        [K::Supplied, K::Defaulted, K::OmittedOptional]
    );
    assert_eq!(explicit.origins()[1].kind, K::Supplied);
    assert_eq!(omitted.origins()[1].path, [P::Field("count".into())]);
}

/// Explicit null remains a present value rather than requesting a default or optional absence.
#[test]
fn composition_supplied_null_and_omission_are_distinct() {
    let null = presence(&V::Record(vec![("label".into(), Some(V::Null))]), limits()).unwrap();
    let absent = presence(&V::Record(vec![]), limits()).unwrap();
    assert_ne!(null.value(), absent.value());
    assert_eq!(null.origins()[2].kind, K::ExplicitNull);
    assert_eq!(
        presence(&V::Record(vec![("count".into(), Some(V::Null))]), limits()),
        Err(E::InvalidValue)
    );
}

/// Closed supplied data obeys the same finite choices, inclusive bounds and Unicode length rules.
#[test]
fn composition_supplied_values_cannot_bypass_restrictions() {
    for value in [number("0"), number("2"), number("4"), V::String("3".into())] {
        assert_eq!(
            presence(&V::Record(vec![("count".into(), Some(value))]), limits()),
            Err(E::InvalidValue)
        );
    }
    for value in [number("1"), number("3")] {
        assert!(presence(&V::Record(vec![("count".into(), Some(value))]), limits()).is_ok());
    }
    assert!(
        presence(
            &V::Record(vec![("label".into(), Some(V::String("é🙂".into())))]),
            limits()
        )
        .is_ok()
    );
    assert_eq!(
        presence(
            &V::Record(vec![("label".into(), Some(V::String("é🙂x".into())))]),
            limits()
        ),
        Err(E::InvalidValue)
    );
}

/// Unknown and duplicate fields reject before any validated value/origin result is published.
#[test]
fn security_composition_supplied_fields_are_closed_and_unique() {
    for fields in [
        vec![("unknown".into(), Some(number("3")))],
        vec![
            ("count".into(), Some(number("1"))),
            ("count".into(), Some(number("3"))),
        ],
    ] {
        assert_eq!(presence(&V::Record(fields), limits()), Err(E::InvalidValue));
    }
}

/// A typed variant list permits distinct declared payload types but no unknown tag or wrong payload.
#[test]
fn composition_supplied_variant_collections_use_one_shared_semantic_model() {
    let catalogue = catalogue(VARIANTS, false, limits()).unwrap();
    let omitted = materialize_composition_value(
        &catalogue,
        OWNER,
        &V::Record(vec![]),
        limits(),
        &CancellationToken::new(),
    )
    .unwrap();
    let explicit = materialize_composition_value(
        &catalogue,
        OWNER,
        omitted.value(),
        limits(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(omitted.value(), explicit.value());
    assert_ne!(omitted.origins(), explicit.origins());
    assert!(
        omitted
            .origins()
            .iter()
            .skip(1)
            .all(|origin| origin.kind == K::Defaulted)
    );
    assert_eq!(
        omitted.origins()[3].path,
        [P::Field("values".into()), P::Element(0), P::Payload]
    );
    for (tag, payload) in [
        ("wrong", number("3")),
        ("text", number("3")),
        ("number", V::Null),
    ] {
        let value = V::Record(vec![(
            "values".into(),
            Some(V::List(vec![V::Variant {
                tag: tag.into(),
                payload: Box::new(payload),
            }])),
        )]);
        assert_eq!(
            materialize_composition_value(
                &catalogue,
                OWNER,
                &value,
                limits(),
                &CancellationToken::new()
            ),
            Err(E::InvalidValue)
        );
    }
}

/// Materialization nodes and recursive depth are independently bounded at exact and one-over values.
#[test]
fn security_composition_supplied_nodes_and_depth_are_independently_bounded() {
    let value = V::Record(vec![]);
    let exact = CompositionLimits {
        value_nodes: 3,
        value_depth: 3,
        ..limits()
    };
    assert!(presence(&value, exact).is_ok());
    assert_eq!(
        presence(
            &value,
            CompositionLimits {
                value_nodes: 2,
                ..exact
            }
        ),
        Err(E::Limit)
    );
    assert_eq!(
        presence(
            &value,
            CompositionLimits {
                value_depth: 2,
                ..exact
            }
        ),
        Err(E::Limit)
    );
    assert_eq!(
        presence(
            &value,
            CompositionLimits {
                value_nodes: 0,
                ..exact
            }
        ),
        Err(E::InvalidLimits)
    );
    assert_eq!(
        presence(&value, CompositionLimits { work: 1, ..exact }),
        Err(E::Limit)
    );
}

/// Supplied variant lists enforce element count and semantic restrictions independently.
#[test]
fn security_composition_supplied_list_count_has_no_unbounded_fallback() {
    let catalogue = catalogue(VARIANTS, false, limits()).unwrap();
    let value = V::Record(vec![]);
    let mut exact = limits();
    exact.json = exact.json.with_array_items(2).unwrap();
    assert!(
        materialize_composition_value(&catalogue, OWNER, &value, exact, &CancellationToken::new())
            .is_ok()
    );
    let short = CompositionLimits {
        json: exact.json.with_array_items(1).unwrap(),
        ..exact
    };
    assert_eq!(
        materialize_composition_value(&catalogue, OWNER, &value, short, &CancellationToken::new()),
        Err(E::Limit)
    );
}

/// Catalogue-wide materialized default nodes have their own exact bound before value publication.
#[test]
fn security_composition_default_expansion_nodes_are_independent() {
    assert!(
        catalogue(
            VARIANTS,
            false,
            CompositionLimits {
                value_nodes: 5,
                ..limits()
            }
        )
        .is_ok()
    );
    assert_eq!(
        catalogue(
            VARIANTS,
            false,
            CompositionLimits {
                value_nodes: 4,
                ..limits()
            }
        ),
        Err(E::Limit)
    );
}

/// Caller-built scalars cannot bypass captured JSON string, digit or scale policy.
#[test]
fn security_composition_supplied_scalar_budgets_are_enforced() {
    let value = V::Record(vec![("label".into(), Some(V::String("é🙂".into())))]);
    let mut exact = limits();
    let structural = StructuralLimits::new(65_536, 64)
        .unwrap()
        .with_string_bytes(6)
        .unwrap();
    exact.json = VocabularyLimits::from_structural(structural);
    assert!(presence(&value, exact).is_ok());
    let short = structural.with_string_bytes(5).unwrap();
    assert_eq!(
        presence(
            &value,
            CompositionLimits {
                json: VocabularyLimits::from_structural(short),
                ..exact
            }
        ),
        Err(E::Limit)
    );
    let digits = StructuralLimits::new(65_536, 64)
        .unwrap()
        .with_numeric_digits(1)
        .unwrap();
    let policy = CompositionLimits {
        json: VocabularyLimits::from_structural(digits),
        ..limits()
    };
    assert_eq!(
        presence(
            &V::Record(vec![("count".into(), Some(number("11")))]),
            policy
        ),
        Err(E::Limit)
    );
    assert!(
        presence(
            &V::Record(vec![("count".into(), Some(number("1")))]),
            policy
        )
        .is_ok()
    );
    let scale = digits.with_numeric_scale(1).unwrap();
    assert_eq!(
        presence(
            &V::Record(vec![("count".into(), Some(number("1e2")))]),
            CompositionLimits {
                json: VocabularyLimits::from_structural(scale),
                ..limits()
            }
        ),
        Err(E::Limit)
    );
}

/// Already-cancelled requests publish neither partial values nor origin classifications.
#[test]
fn security_composition_supplied_cancellation_is_atomic() {
    let catalogue = catalogue(PRESENCE, false, limits()).unwrap();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert_eq!(
        materialize_composition_value(
            &catalogue,
            OWNER,
            &V::Record(vec![]),
            limits(),
            &cancellation
        ),
        Err(E::Cancelled)
    );
}

/// Source-local aliases, capture order and request counters cannot alter standalone materialization.
#[test]
fn composition_supplied_order_and_concurrency_preserve_meaning_and_origins() {
    let first = V::Record(vec![
        ("label".into(), Some(V::Null)),
        ("count".into(), Some(number("1"))),
    ]);
    let mut reversed = first.clone();
    if let V::Record(fields) = &mut reversed {
        fields.reverse();
    }
    let expected = presence(&first, limits()).unwrap();
    assert_eq!(presence(&reversed, limits()).unwrap(), expected);
    let workers = (0..8)
        .map(|_| std::thread::spawn(|| presence(&V::Record(vec![]), limits()).unwrap()))
        .collect::<Vec<_>>();
    let omitted = presence(&V::Record(vec![]), limits()).unwrap();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), omitted);
    }
}
