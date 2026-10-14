// SPDX-License-Identifier: Apache-2.0

//! Pure construction and ordering have exact typed results and decoded-byte bounds.

use super::{
    budget::{Budget, OperationError as E},
    values,
};
use neutral_core::CancellationToken;
use neutral_ir::{
    ExactNumber,
    composition::BindingValue as V,
    expression::{BinaryOperator as O, profile::ExpressionLimits},
};

#[test]
fn scalar_text_is_finite_decimal_not_ir_display_or_host_formatting() {
    for (spelling, expected) in [
        ("0", "0"),
        ("-12.5", "-12.5"),
        ("1e3", "1000"),
        ("1e-4", "0.0001"),
        ("1.23", "1.23"),
    ] {
        let number = V::Number(ExactNumber::from_source(spelling, 100, 100).unwrap());
        assert_eq!(
            values::text(
                &number,
                &mut Budget::new(ExpressionLimits::hard(), &CancellationToken::new()).unwrap()
            )
            .unwrap(),
            V::String(expected.into())
        );
    }
    assert_eq!(
        values::text(
            &V::Bool(true),
            &mut Budget::new(ExpressionLimits::hard(), &CancellationToken::new()).unwrap()
        )
        .unwrap(),
        V::String("true".into())
    );
    assert_eq!(
        values::text(
            &V::Null,
            &mut Budget::new(ExpressionLimits::hard(), &CancellationToken::new()).unwrap()
        ),
        Err(E::TypeMismatch)
    );
}

#[test]
fn utf8_ordering_and_concatenation_do_not_normalize_or_coerce() {
    let cancel = CancellationToken::new();
    let mut budget = Budget::new(ExpressionLimits::hard(), &cancel).unwrap();
    assert_eq!(
        values::binary(
            O::Less,
            &V::String("z".into()),
            &V::String("é".into()),
            &mut budget
        )
        .unwrap(),
        V::Bool(true)
    );
    assert_eq!(
        values::binary(
            O::Concat,
            &V::String("e".into()),
            &V::String("\u{301}".into()),
            &mut budget
        )
        .unwrap(),
        V::String("e\u{301}".into())
    );
    assert_eq!(
        values::binary(
            O::Equal,
            &V::String("é".into()),
            &V::String("e\u{301}".into()),
            &mut budget
        )
        .unwrap(),
        V::Bool(false)
    );
    assert_eq!(
        values::binary(
            O::Concat,
            &V::List(vec![V::Bool(true)]),
            &V::List(vec![V::Bool(false)]),
            &mut budget
        )
        .unwrap(),
        V::List(vec![V::Bool(true), V::Bool(false)])
    );
}

#[test]
fn text_growth_is_rejected_before_reservation_and_keeps_exact_precision() {
    use neutral_core::allocation::testing::observe;
    let number = V::Number(ExactNumber::from_source("1e100", 100, 100).unwrap());
    let limits = ExpressionLimits {
        output_bytes: 100,
        ..ExpressionLimits::hard()
    };
    let (result, reservations) = observe(None, || {
        values::text(
            &number,
            &mut Budget::new(limits, &CancellationToken::new()).unwrap(),
        )
    });
    assert_eq!(result, Err(E::Limit));
    assert_eq!(reservations, 0);
}
