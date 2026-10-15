// SPDX-License-Identifier: Apache-2.0

//! Crate-local successor probe formatting checks.

use super::*;

#[test]
fn fingerprint_formatting_error_is_a_bounded_view_failure() {
    assert_eq!(
        fingerprint_error(std::fmt::Error),
        CompositionProbeError::View(CompositionReadError::Limit)
    );
}

#[test]
fn missing_root_uses_the_redacted_semantic_class() {
    assert_eq!(
        missing_root(0),
        CompositionProbeError::View(CompositionReadError::Semantic)
    );
}
