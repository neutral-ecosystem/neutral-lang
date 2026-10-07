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
    assert!(checkpoints > 30);
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
