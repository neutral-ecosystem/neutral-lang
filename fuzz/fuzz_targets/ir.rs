// SPDX-License-Identifier: Apache-2.0

//! Coverage-guided target for the external framed-IR decoder and validator.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neutral_core::CancellationToken;
use neutral_encoding::{DecodeLimits, decode};
mod composition_support;

fuzz_target!(|bytes: &[u8]| {
    let _ = decode(bytes, DecodeLimits::hard(), &CancellationToken::new());
    let _ = neutral_encoding::project::decode_project(
        bytes,
        DecodeLimits::hard(),
        neutral_encoding::project::hard_project_limits(),
        &CancellationToken::new(),
    );
    for input in [bytes, &composition_support::mutated_seed(bytes)] {
        let _ = neutral_encoding::composition::decode_composition_project(
            input,
            DecodeLimits::hard(),
            neutral_encoding::project::hard_project_limits(),
            composition_support::limits(),
            &CancellationToken::new(),
        );
    }
});
