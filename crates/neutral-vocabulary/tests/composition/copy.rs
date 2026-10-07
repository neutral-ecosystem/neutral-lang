// SPDX-License-Identifier: Apache-2.0

//! Allocation-boundary failure propagation for retained origin field keys.

use super::*;

/// Every vector/text reservation boundary returns an allocation error without a partial path.
#[test]
fn security_composition_origin_copy_faults_are_atomic() {
    let input = vec![
        P::Field("first".to_owned()),
        P::Element(3),
        P::Payload,
        P::Field("second".to_owned()),
    ];
    let mut allocations = 0;
    assert_eq!(
        path_observed(&input, &mut |_| {
            allocations += 1;
            Ok(())
        })
        .unwrap(),
        input
    );
    assert_eq!(allocations, 3);
    for failure in 0..allocations {
        let mut current = 0;
        let result = path_observed(&input, &mut |_| {
            let fail = current == failure;
            current += 1;
            if fail { Err(E::Allocation) } else { Ok(()) }
        });
        assert_eq!(result, Err(E::Allocation));
        assert_eq!(current, failure + 1);
    }
    assert_eq!(path(&input).unwrap(), input);
    assert_eq!(path(&[]).unwrap(), []);
}
/// UTF-8 scalar bytes are copied exactly; reservation failures are not reported as success or bad input.
#[test]
fn security_composition_text_copy_preserves_bytes_and_reports_allocation_failure() {
    assert_eq!(text("λ💡").unwrap(), "λ💡");
    assert_eq!(text("").unwrap(), "");
    assert_eq!(
        text_observed("λ💡", &mut |bytes| {
            assert_eq!(bytes, "λ💡".len());
            Err(E::Allocation)
        }),
        Err(E::Allocation)
    );
}
