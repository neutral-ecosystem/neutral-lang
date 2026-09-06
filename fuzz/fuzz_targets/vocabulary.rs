// SPDX-License-Identifier: Apache-2.0

//! Coverage-guided target for strict captured vocabulary JSON and schema validation.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neutral_core::{StructuralLimits, VocabularyContentDigest};
use neutral_vocabulary::{
    VOCABULARY_ENCODING_VERSION, VOCABULARY_SCHEMA_VERSION, VocabularyLimits, VocabularyLock,
    validate_captured_bundle,
};

/// Frozen inert identity used to construct an exact host lock around fuzz bytes.
const FUZZ_IDENTITY: &str = "Fuzz";
/// Frozen syntactically valid release used only by the fuzz lock.
const FUZZ_VERSION: &str = "0.1.0";

fuzz_target!(|bytes: &[u8]| {
    let structural = StructuralLimits::new(1_048_576, 32).expect("fuzz limits must be nonzero");
    let lock = VocabularyLock::new(
        FUZZ_IDENTITY,
        FUZZ_VERSION,
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        Vec::new(),
    )
    .expect("fuzz lock constants must remain valid");
    let _ = validate_captured_bundle(bytes, &lock, VocabularyLimits::from_structural(structural));
});
