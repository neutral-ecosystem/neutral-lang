// SPDX-License-Identifier: Apache-2.0

//! Missing checked policy arithmetic at the complete composition assembly boundary.

/// Independent capture byte ceilings cannot overflow the assembled combined text ceiling.
#[test]
#[ignore = "UNIMPLEMENTED: combined composition text policy overflow rejects assembly atomically"]
fn combined_capture_text_limits_reject_overflow_without_partial_ir() {
    // TODO: Capture a small valid /2 project with positive source/vocabulary ceilings whose sum overflows u64; assert compilation returns the frozen composition limit code and no artifact at composition/assembly.rs:43-46, then compare a nonoverflowing request with the same bytes to a successful baseline.
    todo!("UNIMPLEMENTED");
}
