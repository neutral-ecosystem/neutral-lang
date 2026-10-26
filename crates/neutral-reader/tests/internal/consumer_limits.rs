// SPDX-License-Identifier: Apache-2.0

//! Literal hard-limit and caller-intersection regressions for the consumer contract.

use crate::consumer::{ConsumerError, ConsumerLimits};

fn values(limits: ConsumerLimits) -> [u64; 11] {
    [
        limits.roots,
        limits.candidates,
        limits.closure_edges,
        limits.bytes,
        limits.text_bytes,
        limits.features,
        limits.diagnostics,
        limits.locations,
        limits.path_segments,
        limits.work,
        limits.output_bytes,
    ]
}

#[test]
fn consumer_hard_limits_keep_the_frozen_default_values() {
    assert_eq!(
        values(ConsumerLimits::default()),
        [
            1_000_000,
            1_000_000,
            1_000_000,
            67_108_864,
            1_048_576,
            1_000_000,
            1_000_000,
            1_000_000,
            64,
            100_000_000,
            100_000_000,
        ]
    );
}

#[test]
fn caller_limits_intersect_hard_ceilings_and_zero_controls_are_rejected() {
    let caller = ConsumerLimits {
        roots: u64::MAX,
        candidates: u64::MAX,
        closure_edges: u64::MAX,
        bytes: u64::MAX,
        text_bytes: u64::MAX,
        features: u64::MAX,
        diagnostics: u64::MAX,
        locations: u64::MAX,
        path_segments: u64::MAX,
        work: u64::MAX,
        output_bytes: u64::MAX,
    };
    assert_eq!(
        values(caller.effective(u64::MAX).unwrap()),
        values(ConsumerLimits::default())
    );
    assert_eq!(caller.effective(7).unwrap().work, 7);

    for index in 0..11 {
        let mut limits = ConsumerLimits::default();
        match index {
            0 => limits.roots = 0,
            1 => limits.candidates = 0,
            2 => limits.closure_edges = 0,
            3 => limits.bytes = 0,
            4 => limits.text_bytes = 0,
            5 => limits.features = 0,
            6 => limits.diagnostics = 0,
            7 => limits.locations = 0,
            8 => limits.path_segments = 0,
            9 => limits.work = 0,
            _ => limits.output_bytes = 0,
        }
        assert!(matches!(
            limits.effective(u64::MAX),
            Err(ConsumerError::Limit)
        ));
    }
    assert!(matches!(
        ConsumerLimits::default().effective(0),
        Err(ConsumerError::Limit)
    ));
}
