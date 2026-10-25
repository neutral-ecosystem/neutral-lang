// SPDX-License-Identifier: Apache-2.0
//! Coverage-guided target for the complete current expression-project decoder.
#![no_main]
use libfuzzer_sys::fuzz_target;
mod composition_support;
fuzz_target!(|bytes: &[u8]| {
    let _ = composition_support::decode(bytes);
    let _ = composition_support::decode(&composition_support::mutated_seed(bytes));
});
