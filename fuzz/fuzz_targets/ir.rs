// SPDX-License-Identifier: Apache-2.0
//! Coverage-guided target for the complete current expression-project decoder.
#![no_main]
use libfuzzer_sys::fuzz_target;
mod expression_support;
fuzz_target!(|bytes: &[u8]| {
    for input in [bytes, &expression_support::mutated_seed(bytes)] {
        if let Some(project) = expression_support::decode(input) {
            expression_support::inspect_public(&project);
            let _ = expression_support::consumer_support::consume(&project, bytes);
        }
    }
});
