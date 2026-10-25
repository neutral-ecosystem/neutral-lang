// SPDX-License-Identifier: Apache-2.0
//! Coverage-guided target for current strict captured vocabulary bundle schemas.
#![no_main]
use libfuzzer_sys::fuzz_target;
use neutral_core::{StructuralLimits, VocabularyContentDigest};
use neutral_vocabulary::{
    PROJECT_VOCABULARY_ENCODING_VERSION, PROJECT_VOCABULARY_SCHEMA_VERSION,
    VOCABULARY_ENCODING_VERSION, VOCABULARY_SCHEMA_VERSION, VocabularyLimits, VocabularyLock,
    validate_captured_bundle, validate_project_bundle,
};
const FUZZ_IDENTITY: &str = "Fuzz";
const FUZZ_VERSION: &str = "0.1.0";
fuzz_target!(|bytes: &[u8]| {
    let limits = VocabularyLimits::from_structural(
        StructuralLimits::new(1_048_576, 32).expect("nonzero limits"),
    );
    let digest = VocabularyContentDigest::from_bytes(bytes);
    let lock = VocabularyLock::new(
        FUZZ_IDENTITY,
        FUZZ_VERSION,
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        digest,
        Vec::new(),
    )
    .expect("constant vocabulary lock");
    let _ = validate_captured_bundle(bytes, &lock, limits);
    let project_lock = VocabularyLock::new(
        FUZZ_IDENTITY,
        FUZZ_VERSION,
        PROJECT_VOCABULARY_ENCODING_VERSION,
        PROJECT_VOCABULARY_SCHEMA_VERSION,
        digest,
        Vec::new(),
    )
    .expect("constant project vocabulary lock");
    let _ = validate_project_bundle(bytes, &project_lock, limits);
});
