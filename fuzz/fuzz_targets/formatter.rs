// SPDX-License-Identifier: Apache-2.0
//! Coverage-guided current grammar formatting after complete capture validation.
#![no_main]
use libfuzzer_sys::fuzz_target;
use neutral_core::CancellationToken;
mod expression_support;
/// Capture and format arbitrary or header-preserving current source mutations.
fn exercise(bytes: &[u8]) {
    if let Some(captured) = expression_support::capture(bytes) {
        let _ = neutral_compiler::format_expression_source(
            &captured,
            expression_support::SOURCE_MODULE,
            &CancellationToken::new(),
        );
    }
}
fuzz_target!(|bytes: &[u8]| {
    exercise(bytes);
    exercise(&expression_support::mutated_source(bytes));
});
