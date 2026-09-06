// SPDX-License-Identifier: Apache-2.0

//! Coverage-guided target for the external framed-IR decoder and validator.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neutral_core::CancellationToken;
use neutral_encoding::{DecodeLimits, decode};

fuzz_target!(|bytes: &[u8]| {
    let _ = decode(bytes, DecodeLimits::hard(), &CancellationToken::new());
});
