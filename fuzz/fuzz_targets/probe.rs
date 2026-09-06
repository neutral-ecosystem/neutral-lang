// SPDX-License-Identifier: Apache-2.0

//! Coverage-guided target for decoded artifact traversal through the public probe.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neutral_core::CancellationToken;
use neutral_encoding::{DecodeLimits, decode};
use neutral_probe::summarize;

fuzz_target!(|bytes: &[u8]| {
    if let Ok(document) = decode(bytes, DecodeLimits::hard(), &CancellationToken::new()) {
        let _ = summarize(&document);
    }
});
