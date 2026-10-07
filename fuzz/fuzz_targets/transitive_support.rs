// SPDX-License-Identifier: Apache-2.0

//! Mutated exact multi-bundle diamond captures and their complete independent consumer pipeline.

use super::composition_support;
use neutral_compiler::CapturedVocabularyInput;
use neutral_core::{CancellationToken, VocabularyContentDigest};
use neutral_encoding::{DecodeLimits, project::hard_project_limits};
use neutral_vocabulary::{VocabularyLock, composition};

/// Captured domain names paired with reviewed independently owned seed bytes.
const BUNDLES: &[(&str, &[u8])] = &[
    (
        "Root",
        include_bytes!("../seeds/vocabulary/diamond-root.json"),
    ),
    (
        "Left",
        include_bytes!("../seeds/vocabulary/diamond-left.json"),
    ),
    (
        "Right",
        include_bytes!("../seeds/vocabulary/diamond-right.json"),
    ),
    (
        "Leaf",
        include_bytes!("../seeds/vocabulary/diamond-leaf.json"),
    ),
];

/// Mutates one exact captured bundle while preserving the other transitive locks.
pub fn run(input: &[u8]) {
    let selected = input.first().map_or(0, |n| usize::from(*n)) % BUNDLES.len();
    let mut vocabularies = Vec::new();
    for (index, (identity, seed)) in BUNDLES.iter().enumerate() {
        let mut bytes = seed.to_vec();
        if index == selected && input.len() > 2 {
            let offset = usize::from(input[1]) % bytes.len();
            for (destination, replacement) in bytes[offset..].iter_mut().zip(&input[2..]) {
                *destination = *replacement;
            }
        }
        let lock = VocabularyLock::new(
            *identity,
            super::FUZZ_VERSION,
            composition::ENCODING_VERSION,
            composition::SCHEMA_VERSION,
            VocabularyContentDigest::from_bytes(&bytes),
            vec![composition::REQUIRED_FEATURE.to_owned()],
        )
        .unwrap();
        vocabularies.push(CapturedVocabularyInput::new(bytes, lock));
    }
    let captured = composition_support::capture_with_vocabularies(
        include_bytes!("../seeds/composition-diamond.neu"),
        vocabularies,
    );
    if input.len() <= 2 {
        assert!(
            captured.is_some(),
            "unmutated reviewed diamond must capture"
        );
    }
    if let Some(captured) = captured {
        let encoded = composition_support::encode(&captured);
        if input.len() <= 2 {
            assert!(
                encoded.is_some(),
                "unmutated reviewed diamond must compile and encode"
            );
        }
        let Some(bytes) = encoded else {
            return;
        };
        neutral_probe::composition::inspect_composition_encoded(&bytes, DecodeLimits::hard(),
            hard_project_limits(), composition_support::limits(), None, &CancellationToken::new())
            .expect("successful transitive compiler output must independently decode and close its public view");
    }
}
