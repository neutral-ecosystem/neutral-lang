// SPDX-License-Identifier: Apache-2.0

//! Coverage-guided target for all strict captured vocabulary bundle schemas.

#![no_main]

use libfuzzer_sys::fuzz_target;
mod composition_support;
mod transitive_support;
use neutral_core::{CancellationToken, StructuralLimits, VocabularyContentDigest};
use neutral_ir::composition::{ClosedValue, CompositionBody};
use neutral_vocabulary::{
    PROJECT_VOCABULARY_ENCODING_VERSION, PROJECT_VOCABULARY_SCHEMA_VERSION,
    VOCABULARY_ENCODING_VERSION, VOCABULARY_SCHEMA_VERSION, VocabularyLimits, VocabularyLock,
    composition::{self, CapturedCompositionBundle, CompositionLimits},
    validate_captured_bundle, validate_project_bundle,
};

/// Frozen inert identity used to construct an exact host lock around fuzz bytes.
const FUZZ_IDENTITY: &str = "Fuzz";
/// Frozen syntactically valid release used only by the fuzz lock.
const FUZZ_VERSION: &str = "0.1.0";
/// Bounds extra supplied-value probes per accepted catalogue, independently of its type count.
const MAX_MATERIALIZATION_REQUESTS: usize = 4;

fuzz_target!(|bytes: &[u8]| {
    transitive_support::run(bytes);
    let _ = neutral_encoding::composition::decode_composition_project(
        &composition_support::mutated_seed(bytes),
        neutral_encoding::DecodeLimits::hard(),
        neutral_encoding::project::hard_project_limits(),
        composition_support::limits(),
        &CancellationToken::new(),
    );
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
    let project_lock = VocabularyLock::new(
        FUZZ_IDENTITY,
        FUZZ_VERSION,
        PROJECT_VOCABULARY_ENCODING_VERSION,
        PROJECT_VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        Vec::new(),
    )
    .expect("fuzz project lock constants must remain valid");
    let _ = validate_project_bundle(
        bytes,
        &project_lock,
        VocabularyLimits::from_structural(structural),
    );
    let composition_lock = VocabularyLock::new(
        FUZZ_IDENTITY,
        FUZZ_VERSION,
        composition::ENCODING_VERSION,
        composition::SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        vec![composition::REQUIRED_FEATURE.to_owned()],
    )
    .expect("fuzz composition lock constants must remain valid");
    let policy = CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(structural));
    if let Ok(catalogue) = composition::validate_composition_closure(
        &[CapturedCompositionBundle {
            bytes,
            lock: &composition_lock,
        }],
        &[(FUZZ_IDENTITY, FUZZ_VERSION)],
        policy,
        &CancellationToken::new(),
    ) {
        // Exercise bounded expansion/origin paths as soon as a mutated captured
        // schema remains valid. This is closed-data checking, never compilation.
        for (bundle, definition) in catalogue
            .bundles()
            .iter()
            .flat_map(|bundle| {
                bundle
                    .definitions
                    .iter()
                    .filter(|definition| definition.public)
                    .map(move |definition| (bundle, definition))
            })
            .take(MAX_MATERIALIZATION_REQUESTS)
        {
            let value = match &definition.body {
                CompositionBody::Record(_) => ClosedValue::Record(Vec::new()),
                CompositionBody::Variant(alternatives) => ClosedValue::Variant {
                    tag: alternatives[0].tag.clone(),
                    payload: Box::new(ClosedValue::Null),
                },
            };
            let _ = composition::materialize_composition_value(
                &catalogue,
                (
                    bundle.identity.identity(),
                    bundle.identity.version(),
                    &definition.name,
                ),
                &value,
                policy,
                &CancellationToken::new(),
            );
        }
    }
});
