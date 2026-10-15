// SPDX-License-Identifier: Apache-2.0

//! Private request-local schema retention faults, without changing a global allocator.

use super::*;
use crate::{VocabularyLimits, VocabularyLock};
use neutral_core::{CancellationToken, StructuralLimits, VocabularyContentDigest};

/// Literal nested contracts cover type/value boxes, names, choices, defaults and lock retention.
#[test]
fn security_composition_schema_retention_faults_and_cancellation_are_atomic() {
    let bytes = include_bytes!("fixtures/bundle.json");
    let lock = VocabularyLock::new(
        "ExampleDomain",
        "1.0.0",
        ENCODING_VERSION,
        SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        vec![REQUIRED_FEATURE.to_owned()],
    )
    .unwrap();
    let input = CapturedCompositionBundle { bytes, lock: &lock };
    let limits = super::super::CompositionLimits::from_vocabulary(
        VocabularyLimits::from_structural(StructuralLimits::new(65_536, 64).unwrap()),
    );
    let cancel = CancellationToken::new();
    let mut budget = Budget::new(limits, &cancel);
    let expected = bundle(input, &mut budget).unwrap();
    let checkpoints = budget.allocations;
    assert!(checkpoints > 0);
    assert_eq!(
        expected
            .definitions
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<Vec<_>>(),
        ["Outcome", "Request"]
    );
    let CompositionBody::Record(fields) = &expected.definitions[1].body else {
        panic!("Request must exercise record field retention");
    };
    assert!(fields.iter().any(|field| field.default.is_some()));
    assert!(fields.iter().any(|field| {
        field
            .restrictions
            .choices
            .as_ref()
            .is_some_and(|choices| !choices.is_empty())
    }));
    for index in 0..checkpoints {
        for cancellation in [false, true] {
            let signal = CancellationToken::new();
            let mut attempt = Budget::new(limits, &signal);
            attempt.allocation_fault = Some((index, cancellation));
            let result = bundle(input, &mut attempt);
            assert_eq!(
                result.unwrap_err(),
                if cancellation {
                    E::Cancelled
                } else {
                    E::Allocation
                }
            );
            assert_eq!(attempt.allocations, index + 1);
            assert_eq!(signal.is_cancelled(), cancellation);
        }
    }
    assert_eq!(
        bundle(input, &mut Budget::new(limits, &CancellationToken::new())).unwrap(),
        expected
    );
}

/// Independent outer count and wrapper depth bounds fail before proportional retention.
#[test]
fn security_composition_schema_limits_precede_retention() {
    let cancel = CancellationToken::new();
    let limits = super::super::CompositionLimits::from_vocabulary(
        VocabularyLimits::from_structural(StructuralLimits::new(65_536, 64).unwrap()),
    );
    let raw = J::Array(vec![J::Null, J::Null]);
    let mut budget = Budget::new(limits, &cancel);
    budget.limits.dependencies_per_bundle = 1;
    assert_eq!(dependencies(&raw, "Example", &mut budget), Err(E::Limit));
    assert_eq!(budget.allocations, 0);
    budget.limits.type_depth = 1;
    let lock = VocabularyLock::new(
        "Example",
        "1.0.0",
        ENCODING_VERSION,
        SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(b""),
        vec![REQUIRED_FEATURE.to_owned()],
    )
    .unwrap();
    assert_eq!(ty(&J::Null, &lock, &mut budget, 2), Err(E::Limit));
    assert_eq!(budget.allocations, 0);
}

/// Closed record defaults canonicalize distinct field names without changing their values.
#[test]
fn closed_record_defaults_preserve_sorted_distinct_fields() {
    let value = J::Object(vec![
        ("kind".into(), J::String("record".into())),
        (
            "fields".into(),
            J::Array(
                ["second", "first"]
                    .into_iter()
                    .map(|name| {
                        J::Object(vec![
                            ("name".into(), J::String(name.into())),
                            (
                                "value".into(),
                                J::Object(vec![("kind".into(), J::String("null".into()))]),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]);
    let cancel = CancellationToken::new();
    let limits = super::super::CompositionLimits::from_vocabulary(
        VocabularyLimits::from_structural(StructuralLimits::new(65_536, 64).unwrap()),
    );
    assert_eq!(
        closed(&value, &mut Budget::new(limits, &cancel), 0).unwrap(),
        V::Record(vec![
            ("first".into(), Some(V::Null)),
            ("second".into(), Some(V::Null))
        ])
    );
}
