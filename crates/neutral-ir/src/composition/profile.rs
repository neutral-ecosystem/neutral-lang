// SPDX-License-Identifier: Apache-2.0

//! Explicit successor selectors shared by capture, vocabulary and consumer boundaries.
//!
//! These are contract versions, not package versions. Defining a selector does
//! not activate a compiler or codec; each boundary must explicitly implement it.

/// Closed captured-input envelope for vocabulary composition and tagged variants.
pub const CAPTURE_REQUEST_VERSION: &str = "neutral.capture/v2";
/// Shared source/vocabulary closed tagged-variant capability.
pub const TAGGED_VARIANTS_FEATURE: &str = "tagged-variants-v1";
/// Composed vocabulary types, defaults and declarative restrictions capability.
pub const VOCABULARY_COMPOSITION_FEATURE: &str = "vocabulary-composition-v2";
/// Exact sorted feature set required by the successor capture boundary.
pub const REQUIRED_FEATURES: &[&str] = &[TAGGED_VARIANTS_FEATURE, VOCABULARY_COMPOSITION_FEATURE];
/// Successor vocabulary logical schema, independent of its unchanged JSON encoding.
pub const VOCABULARY_SCHEMA_VERSION: &str = "2.0";
/// Successor identity profile; old-profile transcript bytes remain immutable.
pub const IDENTITY_PROFILE: &str = "neutral.project-identity/2";
/// Exact captured source/vocabulary domain, distinct from meaning and artifact identity.
pub const CAPTURED_DOMAIN: &str = "neutral/project-captured/v2";
