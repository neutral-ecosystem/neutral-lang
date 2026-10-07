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
/// Complete successor logical artifact; never accepted at the frozen /1 boundary.
pub const PROJECT_IR_SCHEMA: &str = "neutral.project-ir/2";
/// Successor post-compilation public selection envelope.
pub const PROJECT_VIEW_SCHEMA: &str = "neutral.project-view/2";
/// Successor whole-request outcome envelope.
pub const PROJECT_RESULT_SCHEMA: &str = "neutral.project-result/2";
/// Successor complete logical identity domain.
pub const LOGICAL_DOMAIN: &str = "neutral/project-logical/v2";
/// Successor independently checked public interface identity domain.
pub const INTERFACE_DOMAIN: &str = "neutral/project-interface/v2";
/// Distinct restricted-CBOR successor transport.
pub const ENCODING: &str = "NIR-PROJECT-CBOR/2";
/// Successor magic cannot be mistaken for a frozen project or document frame.
pub const MAGIC: [u8; 8] = *b"NEUP2\r\n\x1a";
/// Contextual declaration spelling, deliberately outside the frozen protected set.
pub const VARIANT: &str = "variant";
/// Closed contextual discriminator member.
pub const TAG: &str = "tag";
/// Closed contextual payload member.
pub const PAYLOAD: &str = "payload";
/// Hard retained-item and traversal ceiling shared by raw successor consumers.
pub const MAX_ITEMS: u64 = 1_000_000;
