// SPDX-License-Identifier: Apache-2.0
//! Coverage-guided independent entry discovery, public closure and explicit consumer requests.
#![no_main]
use libfuzzer_sys::fuzz_target;
mod expression_support;
fuzz_target!(|bytes: &[u8]| {
    for input in [bytes, &expression_support::mutated_seed(bytes)] {
        let _ = expression_support::probe(input);
        if let Some(project) = expression_support::decode(input) {
            let _ = expression_support::consumer_support::consume(&project, bytes);
        }
    }
});
