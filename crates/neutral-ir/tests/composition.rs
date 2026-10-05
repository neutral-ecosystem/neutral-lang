// SPDX-License-Identifier: Apache-2.0

//! Exact-number ordering tests for the shared composition restriction model.

use neutral_ir::{ExactNumber, composition::compare_exact_numbers};
use std::cmp::Ordering;

/// Normalizes bounded decimal input without host floating-point conversion.
fn number(value: &str) -> ExactNumber {
    ExactNumber::from_source(value, 128, 1_000_000).unwrap()
}

/// Exact bounds handle zero/sign, normalized equality, scale and virtual coefficient padding.
#[test]
fn composition_numeric_comparison_is_exact_and_signed() {
    for (left, right, expected) in [
        ("-1", "1", Ordering::Less),
        ("0", "-0", Ordering::Equal),
        ("0", "1e-999999", Ordering::Less),
        ("-1e-999999", "0", Ordering::Less),
        ("1.00", "1e0", Ordering::Equal),
        ("1234", "1230", Ordering::Greater),
        ("-1234", "-1230", Ordering::Less),
        ("9e999999", "1e1000000", Ordering::Less),
    ] {
        let (left, right) = (number(left), number(right));
        assert_eq!(compare_exact_numbers(&left, &right), expected);
        assert_eq!(compare_exact_numbers(&right, &left), expected.reverse());
    }
}

/// Extreme scales compare in constant space and cannot overflow decimal positions.
#[test]
fn composition_numeric_comparison_does_not_expand_extreme_scales() {
    let tiny = ExactNumber::from_normalized_parts(false, "1", i64::MIN, 1, u64::MAX).unwrap();
    let large = ExactNumber::from_normalized_parts(false, "1", i64::MAX, 1, u64::MAX).unwrap();
    assert_eq!(compare_exact_numbers(&tiny, &large), Ordering::Less);
    assert_eq!(compare_exact_numbers(&number("0"), &tiny), Ordering::Less);
    assert_eq!(compare_exact_numbers(&number("1"), &large), Ordering::Less);
}

/// The comparator is antisymmetric/transitive across a mixed signed exact-decimal catalogue.
#[test]
fn composition_numeric_order_is_total_for_normalized_values() {
    let numbers = [
        "-1000", "-1", "-0.2", "0", "0.0001", "0.2", "1", "1.001", "1000",
    ]
    .map(number);
    for (a, left) in numbers.iter().enumerate() {
        for (b, right) in numbers.iter().enumerate() {
            assert_eq!(compare_exact_numbers(left, right), a.cmp(&b));
        }
    }
}
