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

#[test]
fn structural_equality_preserves_order_presence_tags_and_reference_identity() {
    use neutral_ir::{LogicalModuleIdentity, ModuleSymbolIdentity};
    let reference = |name| {
        V::Reference(ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new("1.0", "example"),
            name,
        ))
    };
    for (left, right, equal) in [
        (
            V::List(vec![V::Bool(true), V::Bool(false)]),
            V::List(vec![V::Bool(true), V::Bool(false)]),
            true,
        ),
        (
            V::List(vec![V::Bool(true), V::Bool(false)]),
            V::List(vec![V::Bool(false), V::Bool(true)]),
            false,
        ),
        (
            V::Record(vec![("item".into(), None)]),
            V::Record(vec![("item".into(), Some(V::Null))]),
            false,
        ),
        (
            V::Record(vec![("item".into(), Some(V::List(vec![V::Bool(true)])))]),
            V::Record(vec![("item".into(), Some(V::List(vec![V::Bool(true)])))]),
            true,
        ),
        (
            V::Variant {
                tag: "ready".into(),
                payload: Box::new(V::Bool(true)),
            },
            V::Variant {
                tag: "ready".into(),
                payload: Box::new(V::Bool(false)),
            },
            false,
        ),
        (
            V::Variant {
                tag: "ready".into(),
                payload: Box::new(V::Bool(true)),
            },
            V::Variant {
                tag: "failed".into(),
                payload: Box::new(V::Bool(true)),
            },
            false,
        ),
        (reference("a"), reference("a"), true),
        (reference("a"), reference("b"), false),
    ] {
        for (operator, expected) in [(O::Equal, equal), (O::NotEqual, !equal)] {
            assert_eq!(
                values::binary(
                    operator,
                    &left,
                    &right,
                    &mut Budget::new(ExpressionLimits::hard(), &CancellationToken::new(),).unwrap()
                ),
                Ok(V::Bool(expected))
            );
        }
    }
}

#[test]
fn utf8_text_and_list_output_limits_reject_before_reservation() {
    use neutral_core::allocation::testing::observe;
    let text = V::String("éa".into());
    let left = V::List(vec![V::String("é".into())]);
    let right = V::List(vec![V::String("a".into())]);
    let cancel = CancellationToken::new();
    for output_bytes in [2, 3] {
        let limits = ExpressionLimits {
            output_bytes,
            ..ExpressionLimits::hard()
        };
        let (result, count) = observe(None, || {
            values::text(&text, &mut Budget::new(limits, &cancel).unwrap())
        });
        if output_bytes == 2 {
            assert_eq!(result, Err(E::Limit));
            assert_eq!(count, 0, "text rejects before copying decoded UTF-8");
        } else {
            assert_eq!(result, Ok(V::String("éa".into())));
        }
        let (result, count) = observe(None, || {
            values::binary(
                O::Concat,
                &left,
                &right,
                &mut Budget::new(limits, &cancel).unwrap(),
            )
        });
        if output_bytes == 2 {
            assert_eq!(result, Err(E::Limit));
            assert_eq!(
                count, 0,
                "list concatenation rejects before allocating children"
            );
        } else {
            assert_eq!(
                result,
                Ok(V::List(vec![V::String("é".into()), V::String("a".into())]))
            );
        }
    }
}

#[test]
fn construction_reservations_cancel_and_recover_without_mutating_inputs() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    for (left, right, expected) in [
        (V::Bool(true), None, V::String("true".into())),
        (
            V::Number(ExactNumber::from_source("-12.5", 3, 1).unwrap()),
            None,
            V::String("-12.5".into()),
        ),
        (V::String("é".into()), None, V::String("é".into())),
        (
            V::String("é".into()),
            Some(V::String("a".into())),
            V::String("éa".into()),
        ),
        (
            V::List(vec![V::String("é".into())]),
            Some(V::List(vec![V::String("a".into())])),
            V::List(vec![V::String("é".into()), V::String("a".into())]),
        ),
    ] {
        let run = |cancel: &CancellationToken| {
            let mut budget = Budget::new(ExpressionLimits::hard(), cancel)?;
            match &right {
                Some(right) => values::binary(O::Concat, &left, right, &mut budget),
                None => values::text(&left, &mut budget),
            }
        };
        let (result, count) = observe(None, || run(&CancellationToken::new()));
        assert_eq!(result, Ok(expected.clone()));
        assert_ne!(count, 0);
        for index in 0..count {
            let (failed, _) = observe(Some(index), || run(&CancellationToken::new()));
            assert_eq!(failed, Err(E::Limit), "reservation {index}");
            let signal = CancellationToken::new();
            let (failed, _) = observe_cancellation(index, &signal, || run(&signal));
            assert_eq!(failed, Err(E::Cancelled), "cancellation {index}");
            assert_eq!(run(&CancellationToken::new()), Ok(expected.clone()));
        }
    }
}

#[test]
fn scalar_text_and_concatenation_use_decoded_utf8_output_boundaries() {
    for (left, right, exact, expected) in [
        (V::Bool(false), None, 5, V::String("false".into())),
        (
            V::Number(ExactNumber::from_source("-0.125", 4, 3).unwrap()),
            None,
            6,
            V::String("-0.125".into()),
        ),
        (
            V::String("é".into()),
            Some(V::String("a".into())),
            3,
            V::String("éa".into()),
        ),
    ] {
        for bytes in [exact - 1, exact] {
            let signal = CancellationToken::new();
            let mut budget = Budget::new(
                ExpressionLimits {
                    output_bytes: bytes,
                    ..ExpressionLimits::hard()
                },
                &signal,
            )
            .unwrap();
            let actual = match &right {
                Some(right) => values::binary(O::Concat, &left, right, &mut budget),
                None => values::text(&left, &mut budget),
            };
            assert_eq!(
                actual,
                if bytes == exact {
                    Ok(expected.clone())
                } else {
                    Err(E::Limit)
                }
            );
        }
    }
}
