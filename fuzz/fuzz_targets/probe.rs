// SPDX-License-Identifier: Apache-2.0

//! Coverage-guided target for decoded artifact traversal through the public probe.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neutral_core::CancellationToken;
use neutral_encoding::{DecodeLimits, decode};
use neutral_probe::summarize;
mod composition_support;

fuzz_target!(|bytes: &[u8]| {
    if let Ok(document) = decode(bytes, DecodeLimits::hard(), &CancellationToken::new()) {
        let _ = summarize(&document);
    }
    let _ = neutral_probe::project::inspect_project_encoded(
        bytes,
        DecodeLimits::hard(),
        neutral_encoding::project::hard_project_limits(),
        None,
        &CancellationToken::new(),
    );
    for input in [bytes, &composition_support::mutated_seed(bytes)] {
        let cancel = CancellationToken::new();
        if let Ok(summary) = neutral_probe::composition::inspect_composition_encoded(
            input,
            DecodeLimits::hard(),
            neutral_encoding::project::hard_project_limits(),
            composition_support::limits(),
            None,
            &cancel,
        ) {
            let _ = neutral_probe::composition::render_composition_summary_json(&summary, &cancel);
        }
    }
});
