// SPDX-License-Identifier: Apache-2.0

//! Exact selected-value paths and independent resource failures.

use super::*;
use neutral_core::CancellationToken;
use neutral_ir::expression::profile::ExpressionLimits;

/// Nested paths preserve the selected value and never unwrap null or omitted members.
#[test]
fn selectors_distinguish_missing_fields_indices_alternatives_and_nulls() {
    let value = V::Record(vec![
        ("absent".into(), None),
        (
            "items".into(),
            Some(V::List(vec![V::Variant {
                tag: "ready".into(),
                payload: Box::new(V::Bool(true)),
            }])),
        ),
    ]);
    let cancel = CancellationToken::new();
    let path = [
        SelectorSegment::Field("items".into()),
        SelectorSegment::Index(0),
        SelectorSegment::Alternative("ready".into()),
    ];
    assert_eq!(
        select(
            &value,
            &path,
            &mut Budget::new(ExpressionLimits::hard(), &cancel).unwrap()
        ),
        Ok(&V::Bool(true))
    );
    assert_eq!(
        select(
            &value,
            &[],
            &mut Budget::new(ExpressionLimits::hard(), &cancel).unwrap()
        ),
        Ok(&value)
    );
    for path in [
        vec![SelectorSegment::Field("missing".into())],
        vec![SelectorSegment::Field("absent".into())],
        vec![
            SelectorSegment::Field("items".into()),
            SelectorSegment::Index(1),
        ],
        vec![
            SelectorSegment::Field("items".into()),
            SelectorSegment::Index(0),
            SelectorSegment::Alternative("failed".into()),
        ],
        vec![SelectorSegment::Index(0)],
    ] {
        assert_eq!(
            select(
                &value,
                &path,
                &mut Budget::new(ExpressionLimits::hard(), &cancel).unwrap()
            ),
            Err(E::Restriction),
            "{path:?}"
        );
    }
    assert_eq!(
        select(
            &V::Null,
            &[SelectorSegment::Field("field".into())],
            &mut Budget::new(ExpressionLimits::hard(), &cancel).unwrap()
        ),
        Err(E::Restriction)
    );
}

/// Field comparisons and tagged-payload steps debit the same caller work and cancellation state.
#[test]
fn selectors_reject_insufficient_work_and_mid_request_cancellation() {
    let value = V::Record(vec![("field".into(), Some(V::Bool(true)))]);
    let path = [SelectorSegment::Field("field".into())];
    let cancel = CancellationToken::new();
    let limits = ExpressionLimits {
        work: 1,
        ..ExpressionLimits::hard()
    };
    assert_eq!(
        select(&value, &path, &mut Budget::new(limits, &cancel).unwrap()),
        Err(E::Limit)
    );
    let variant = V::Variant {
        tag: "ready".into(),
        payload: Box::new(V::Bool(true)),
    };
    assert_eq!(
        select(
            &variant,
            &[SelectorSegment::Alternative("ready".into())],
            &mut Budget::new(limits, &cancel).unwrap()
        ),
        Err(E::Limit)
    );
    let mut budget = Budget::new(ExpressionLimits::hard(), &cancel).unwrap();
    cancel.cancel();
    assert_eq!(select(&value, &path, &mut budget), Err(E::Cancelled));
}
