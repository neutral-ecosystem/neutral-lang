// SPDX-License-Identifier: Apache-2.0

//! Hand-checked arithmetic and frozen rational-oracle literals, never host floats.

use super::{
    budget::{Budget, OperationError as E},
    decimal,
};
use neutral_core::{
    CancellationToken,
    allocation::testing::{observe, observe_cancellation},
};
use neutral_ir::{
    ExactNumber,
    expression::{BinaryOperator as O, profile::ExpressionLimits},
};

fn number(text: &str) -> ExactNumber {
    ExactNumber::from_source(text, 4096, 4096).unwrap()
}
fn operation(op: O, left: &str, right: &str) -> Result<ExactNumber, E> {
    decimal::binary(
        op,
        &number(left),
        &number(right),
        &mut Budget::new(ExpressionLimits::hard(), &CancellationToken::new())?,
    )
}

#[test]
fn exact_operations_preserve_decimal_sign_scale_and_zero() {
    for (op, a, b, want) in [
        (O::Add, "0.125", "0.875", "1"),
        (O::Subtract, "1", "1", "0"),
        (O::Subtract, "-1.2", "3.4", "-4.6"),
        (O::Add, "1e3", "-999", "1"),
        (O::Multiply, "-1.25", "0.8", "-1"),
        (O::Multiply, "0", "-123", "0"),
        (O::Add, "999", "1", "1000"),
        (O::Subtract, "1000", "1", "999"),
        (O::Add, "0", "-0.001", "-0.001"),
        (O::Subtract, "0.001", "0", "0.001"),
    ] {
        assert_eq!(operation(op, a, b).unwrap(), number(want));
    }
}

#[test]
fn division_matches_frozen_independent_decimal_vectors() {
    for (a, b, want) in [
        ("1", "8", "0.125"),
        ("1", "40", "0.025"),
        ("7", "2", "3.5"),
        ("1", "3", "0.3333333333333333333333333333333333"),
        ("2", "3", "0.6666666666666666666666666666666667"),
        ("1", "7", "0.1428571428571428571428571428571429"),
        ("-1", "6", "-0.1666666666666666666666666666666667"),
        ("0", "-3", "0"),
        ("1e-40", "3", "3.333333333333333333333333333333333e-41"),
        ("1e40", "3", "3.333333333333333333333333333333333e39"),
        (
            "10000000000000000000000000000000001",
            "1",
            "10000000000000000000000000000000001",
        ),
        (
            "99999999999999999999999999999999999998",
            "99999999999999999999999999999999999999",
            "1",
        ),
        (
            "10000000000000000000000000000000001",
            "3",
            "3333333333333333333333333333333334",
        ),
        (
            "1",
            "1099511627776",
            "0.0000000000009094947017729282379150390625",
        ),
    ] {
        assert_eq!(operation(O::Divide, a, b).unwrap(), number(want), "{a}/{b}");
    }
    assert_eq!(operation(O::Divide, "1", "0"), Err(E::DivisionByZero));
}

#[test]
fn arithmetic_limits_cancellation_and_allocation_fail_atomically() {
    let cancel = CancellationToken::new();
    let a = number("1");
    let b = number("7");
    let (result, reservations) = observe(None, || {
        decimal::binary(
            O::Divide,
            &a,
            &b,
            &mut Budget::new(ExpressionLimits::hard(), &cancel).unwrap(),
        )
    });
    assert!(result.is_ok());
    for index in 0..reservations {
        let (result, _) = observe(Some(index), || {
            decimal::binary(
                O::Divide,
                &a,
                &b,
                &mut Budget::new(ExpressionLimits::hard(), &cancel).unwrap(),
            )
        });
        assert_eq!(result, Err(E::Limit));
        let signal = CancellationToken::new();
        let (result, _) = observe_cancellation(index, &signal, || {
            decimal::binary(
                O::Divide,
                &a,
                &b,
                &mut Budget::new(ExpressionLimits::hard(), &signal).unwrap(),
            )
        });
        assert_eq!(result, Err(E::Cancelled));
        assert!(operation(O::Divide, "1", "7").is_ok());
    }
    let mut limits = ExpressionLimits::hard();
    limits.coefficient_digits = 2;
    assert_eq!(
        decimal::binary(
            O::Add,
            &number("99"),
            &number("2"),
            &mut Budget::new(limits, &cancel).unwrap()
        ),
        Err(E::Limit)
    );
    assert_eq!(
        decimal::binary(
            O::Add,
            &number("98"),
            &number("1"),
            &mut Budget::new(limits, &cancel).unwrap()
        )
        .unwrap(),
        number("99")
    );
    limits.work = 1;
    assert_eq!(
        decimal::binary(
            O::Multiply,
            &a,
            &b,
            &mut Budget::new(limits, &cancel).unwrap()
        ),
        Err(E::Limit)
    );
}

#[test]
fn small_signed_arithmetic_matches_independent_integer_results() {
    for left in -25_i64..=25 {
        for right in -9_i64..=9 {
            for (operator, expected) in [
                (O::Add, left + right),
                (O::Subtract, left - right),
                (O::Multiply, left * right),
            ] {
                assert_eq!(
                    operation(operator, &left.to_string(), &right.to_string()).unwrap(),
                    number(&expected.to_string())
                );
            }
        }
    }
}

#[test]
fn normalized_scale_output_and_work_controls_accept_exactly_the_boundary() {
    let cancel = CancellationToken::new();
    let left = number("1e2");
    let right = number("1e1");
    for scale in [2, 3] {
        let limits = ExpressionLimits {
            absolute_scale: scale,
            ..ExpressionLimits::hard()
        };
        let result = decimal::binary(
            O::Multiply,
            &left,
            &right,
            &mut Budget::new(limits, &cancel).unwrap(),
        );
        assert_eq!(
            result,
            if scale == 3 {
                Ok(number("1e3"))
            } else {
                Err(E::Limit)
            }
        );
    }
    let left = number("1");
    let right = number("2");
    let required_output = std::mem::size_of::<ExactNumber>() as u64 + 1;
    for bytes in [required_output - 1, required_output] {
        let limits = ExpressionLimits {
            output_bytes: bytes,
            ..ExpressionLimits::hard()
        };
        let result = decimal::binary(
            O::Add,
            &left,
            &right,
            &mut Budget::new(limits, &cancel).unwrap(),
        );
        assert_eq!(
            result,
            if bytes == required_output {
                Ok(number("3"))
            } else {
                Err(E::Limit)
            }
        );
    }
    let mut budget = Budget::new(ExpressionLimits::hard(), &cancel).unwrap();
    assert_eq!(
        decimal::binary(O::Add, &left, &right, &mut budget),
        Ok(number("3"))
    );
    let work = ExpressionLimits::hard().work - budget.remaining().work;
    for available in [work - 1, work] {
        let limits = ExpressionLimits {
            work: available,
            ..ExpressionLimits::hard()
        };
        let result = decimal::binary(
            O::Add,
            &left,
            &right,
            &mut Budget::new(limits, &cancel).unwrap(),
        );
        assert_eq!(
            result,
            if available == work {
                Ok(number("3"))
            } else {
                Err(E::Limit)
            }
        );
    }
}
