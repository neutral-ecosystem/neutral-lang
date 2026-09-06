// SPDX-License-Identifier: Apache-2.0

//! Coverage-guided target for capture, source decoding, frontend, and semantics.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neutral_compiler::{CompilationRequest, compile};
use neutral_core::{CancellationToken, StructuralLimits};

fuzz_target!(|bytes: &[u8]| {
    let limits = StructuralLimits::new(1_048_576, 32).expect("fuzz limits must be nonzero");
    let _ = compile(CompilationRequest::new(
        bytes.to_vec(),
        limits,
        CancellationToken::new(),
    ));
});
