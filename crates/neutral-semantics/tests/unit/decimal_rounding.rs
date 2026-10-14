// SPDX-License-Identifier: Apache-2.0

//! The exact quotient rounding primitive resolves midpoint parity without floats.

use super::*;
use neutral_core::CancellationToken;
use neutral_ir::expression::profile::ExpressionLimits;

#[test]
fn nearest_rounding_uses_even_parity_only_at_exact_midpoints() {
    for (remainder, denominator, last, want) in [
        (&[5][..], &[1, 0][..], 2, false),
        (&[5][..], &[1, 0][..], 3, true),
        (&[4][..], &[1, 0][..], 3, false),
        (&[6][..], &[1, 0][..], 2, true),
    ] {
        assert_eq!(
            round_up(
                remainder,
                denominator,
                last,
                &mut Budget::new(ExpressionLimits::hard(), &CancellationToken::new()).unwrap()
            )
            .unwrap(),
            want
        );
    }
}
