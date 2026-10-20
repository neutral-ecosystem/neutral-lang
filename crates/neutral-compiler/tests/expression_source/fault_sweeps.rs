// SPDX-License-Identifier: Apache-2.0

//! Reservation faults fail atomically and leave captured requests reusable.

use super::*;
use neutral_core::allocation::testing::{observe, observe_cancellation};

/// Every observed reservation must reject injected failure and cancellation without authority.
#[test]
fn expression_source_allocation_and_cancellation_sweeps_preserve_request_reuse() {
    for (source, binding, count) in [
        (
            include_str!("fixtures/positive/fault-output.neu"),
            "choice",
            "2",
        ),
        (
            include_str!("fixtures/positive/fault-matrix.neu"),
            "choices__0",
            "3",
        ),
        (
            include_str!("fixtures/positive/fault-reuse.neu"),
            "choice",
            "4",
        ),
    ] {
        let captured = capture(source, ExpressionLimits::hard(), ExpansionLimits::hard());
        let cancel = CancellationToken::new();
        let (baseline, checkpoints) =
            observe(None, || compile_expression_project(&captured, &cancel));
        let baseline = baseline.unwrap();
        assert!(checkpoints > 0);
        assert_eq!(
            value(&baseline, binding),
            &V::Variant {
                tag: "ready".into(),
                payload: Box::new(V::Record(vec![(
                    "count".into(),
                    Some(V::Number(
                        ExactNumber::from_source(
                            count,
                            ExpressionLimits::hard().coefficient_digits,
                            ExpressionLimits::hard().absolute_scale
                        )
                        .unwrap()
                    ))
                )]))
            }
        );
        for index in 0..checkpoints {
            let (failed, observed) = observe(Some(index), || {
                compile_expression_project(&captured, &cancel)
            });
            assert!(
                failed.is_err(),
                "allocation checkpoint {index}/{checkpoints}"
            );
            let failed = failed.unwrap_err();
            assert!(
                observed > index,
                "allocation checkpoint {index}/{checkpoints} was not reached"
            );
            assert!(
                matches!(
                    failed.code,
                    "NEU-EXP-002"
                        | "NEU-EXP-003"
                        | "NEU-EXP-009"
                        | "NEU-TPL-008"
                        | "NEU-COM-018"
                        | "NEU-EVAL-006"
                ),
                "checkpoint {index}: {failed:?}"
            );
            let signal = CancellationToken::new();
            let (cancelled, _) = observe_cancellation(index, &signal, || {
                compile_expression_project(&captured, &signal)
            });
            assert!(
                signal.is_cancelled(),
                "cancellation checkpoint {index}/{checkpoints}"
            );
            assert!(
                cancelled.is_err(),
                "cancelled checkpoint {index}/{checkpoints}"
            );
        }
        let recovery_signal = CancellationToken::new();
        let (recovered, recovered_checkpoints) = observe(None, || {
            compile_expression_project(&captured, &recovery_signal)
        });
        let recovered = recovered.unwrap();
        assert_eq!(recovered, baseline);
        assert_eq!(recovered_checkpoints, checkpoints);
        eprintln!(
            "{binding}: rejected {checkpoints} allocation faults and {checkpoints} cancellations; recovery passed"
        );
    }
}
