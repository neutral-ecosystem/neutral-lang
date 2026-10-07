// SPDX-License-Identifier: Apache-2.0

//! Coverage-guided target for capture, source decoding, frontend, and semantics.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neutral_compiler::{CompilationRequest, compile};
use neutral_core::{CancellationToken, StructuralLimits};
mod composition_support;

fuzz_target!(|bytes: &[u8]| {
    let limits = StructuralLimits::new(1_048_576, 32).expect("fuzz limits must be nonzero");
    let _ = compile(CompilationRequest::new(
        bytes.to_vec(),
        limits,
        CancellationToken::new(),
    ));
    if bytes.len() <= 65_536
        && let Some(captured) = composition_support::capture(bytes)
    {
        let _ = composition_support::encode(&captured);
    }
    // Also mutate inside a valid header so a random campaign can reach the new
    // grammar instead of spending its entire budget rejecting the envelope.
    let mut source = include_bytes!("../seeds/composition-source.neu").to_vec();
    let header_bytes = b"neu \"1.0\"\nmodule example\n".len();
    for (original, replacement) in source[header_bytes..].iter_mut().zip(bytes) {
        *original = *replacement;
    }
    if let Some(captured) = composition_support::capture(&source)
        && let Some(encoded) = composition_support::encode(&captured)
    {
        neutral_encoding::composition::decode_composition_project(
            &encoded,
            neutral_encoding::DecodeLimits::hard(),
            neutral_encoding::project::hard_project_limits(),
            composition_support::limits(),
            &CancellationToken::new(),
        )
        .expect("encoded compiler output must decode independently");
    }
    let _ = neutral_encoding::composition::decode_composition_project(
        &composition_support::mutated_seed(bytes),
        neutral_encoding::DecodeLimits::hard(),
        neutral_encoding::project::hard_project_limits(),
        composition_support::limits(),
        &CancellationToken::new(),
    );
});
