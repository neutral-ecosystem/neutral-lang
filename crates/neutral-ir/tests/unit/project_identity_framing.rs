// SPDX-License-Identifier: Apache-2.0

//! Direct budget and cancellation boundaries for unpublished identity frames.

use super::{IdentityError, IdentityLimits, Writer, ordered};
use neutral_core::CancellationToken;

/// Cancellation after construction is observed by every writer boundary.
#[test]
fn framing_checks_cancellation_after_writer_construction() {
    let cancel = CancellationToken::new();
    let mut writer = Writer::new(
        IdentityLimits {
            bytes: 64,
            nodes: 4,
        },
        &cancel,
    )
    .unwrap();
    assert_eq!(writer.check(), Ok(()));
    cancel.cancel();
    assert_eq!(writer.check(), Err(IdentityError::Cancelled));
    assert_eq!(writer.items(0), Err(IdentityError::Cancelled));
    assert_eq!(writer.text(""), Err(IdentityError::Cancelled));
    assert_eq!(writer.leaf("value", b""), Err(IdentityError::Cancelled));
}

/// Collection bounds accept their exact ceiling and reject the first excess item.
#[test]
fn framing_collection_work_limit_is_inclusive() {
    let cancel = CancellationToken::new();
    let writer = Writer::new(
        IdentityLimits {
            bytes: 64,
            nodes: 2,
        },
        &cancel,
    )
    .unwrap();
    assert_eq!(writer.items(0), Ok(()));
    assert_eq!(writer.items(1), Ok(()));
    assert_eq!(writer.items(2), Ok(()));
    assert_eq!(writer.items(3), Err(IdentityError::Limit));
}

/// Key comparison work counts cumulative UTF-8 bytes even before any output is written.
#[test]
fn framing_key_work_budget_accepts_equality_and_rejects_overshoot() {
    let cancel = CancellationToken::new();
    let mut writer = Writer::new(IdentityLimits { bytes: 3, nodes: 2 }, &cancel).unwrap();
    assert_eq!(writer.text("é"), Ok(()));
    assert_eq!(writer.text("x"), Ok(()));
    assert_eq!(writer.text(""), Ok(()));
    assert_eq!(writer.text("zz"), Err(IdentityError::Limit));
    assert!(writer.finish().is_empty());
}

/// Nested frames backpatch payload lengths and write integers in network byte order.
#[test]
fn framing_nested_numbers_preserve_exact_wire_bytes() {
    let cancel = CancellationToken::new();
    let mut writer = Writer::new(
        IdentityLimits {
            bytes: 30,
            nodes: 2,
        },
        &cancel,
    )
    .unwrap();
    writer
        .frame("r", |writer| writer.number("n", 0x0102_0304_0506_0708))
        .unwrap();
    assert_eq!(
        writer.finish(),
        [
            0, 1, b'r', 0, 0, 0, 0, 0, 0, 0, 19, 0, 1, b'n', 0, 0, 0, 0, 0, 0, 0, 8, 1, 2, 3, 4, 5,
            6, 7, 8,
        ]
    );
}

/// Output bytes and frame counts accept the ceiling and reject the first excess independently.
#[test]
fn framing_output_and_node_budgets_include_complete_frame_overhead() {
    let cancel = CancellationToken::new();
    let mut writer = Writer::new(
        IdentityLimits {
            bytes: 12,
            nodes: 1,
        },
        &cancel,
    )
    .unwrap();
    assert_eq!(writer.leaf("x", b"a"), Ok(()));
    assert_eq!(writer.leaf("", b""), Err(IdentityError::Limit));

    let mut writer = Writer::new(
        IdentityLimits {
            bytes: 12,
            nodes: 2,
        },
        &cancel,
    )
    .unwrap();
    assert_eq!(writer.leaf("x", b"ab"), Err(IdentityError::Limit));

    let mut writer = Writer::new(
        IdentityLimits {
            bytes: 64,
            nodes: 1,
        },
        &cancel,
    )
    .unwrap();
    assert_eq!(
        writer.frame("r", |writer| writer.leaf("x", b"")),
        Err(IdentityError::Limit)
    );
}

/// Invalid tags and body failures cannot become successful frames, including late cancellation.
#[test]
fn framing_rejects_invalid_tags_and_propagates_body_failures() {
    let cancel = CancellationToken::new();
    let limits = IdentityLimits {
        bytes: 64,
        nodes: 4,
    };
    let mut writer = Writer::new(limits, &cancel).unwrap();
    assert_eq!(writer.leaf("é", b""), Err(IdentityError::InvalidInput));
    let mut writer = Writer::new(limits, &cancel).unwrap();
    assert_eq!(
        writer.frame("x", |_| Err(IdentityError::InvalidInput)),
        Err(IdentityError::InvalidInput)
    );
    let mut writer = Writer::new(limits, &cancel).unwrap();
    assert_eq!(
        writer.frame("x", |_| {
            cancel.cancel();
            Ok(())
        }),
        Err(IdentityError::Cancelled)
    );
}

/// Canonical ordering permits empty and increasing inputs but rejects duplicates and reversals.
#[test]
fn framing_ordering_requires_strictly_increasing_values() {
    assert_eq!(ordered::<u8>([]), Ok(()));
    assert_eq!(ordered([1]), Ok(()));
    assert_eq!(ordered([1, 2, 3]), Ok(()));
    assert_eq!(ordered([1, 1]), Err(IdentityError::InvalidInput));
    assert_eq!(ordered([2, 1]), Err(IdentityError::InvalidInput));
}
