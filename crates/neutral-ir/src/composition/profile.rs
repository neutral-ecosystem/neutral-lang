// SPDX-License-Identifier: Apache-2.0

//! Explicit successor selectors shared by capture, vocabulary and consumer boundaries.
//!
//! These are contract versions, not package versions. Defining a selector does
//! not activate a compiler or codec; each boundary must explicitly implement it.
//!
//! The vocabulary composition schema and shared record/variant feature spellings
//! remain useful to current expression projects. Complete `/2` capture/project/view
//! selectors here retain their historical definitions; current `/3` transport
//! discovery uses [`crate::expression::profile`] instead. Shared model reuse is not
//! a request to accept an older complete artifact or negotiate a fallback.
//!
//! Policy tag order is part of derivation/wire compatibility. Equal current version
//! strings do not merge schema, encoding and identity contracts, and hard item/view
//! ceilings are independent of the more restrictive request policy a caller supplies.

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
/// Successor processing context domain, independent of complete logical meaning.
pub const DERIVATION_DOMAIN: &str = "neutral/project-derivation/v2";
/// Successor artifact format and post-compilation selection domain.
pub const ARTIFACT_DOMAIN: &str = "neutral/project-artifact/v2";
/// Composition control tags in the immutable policy tuple order.
pub const COMPOSITION_LIMIT_TAGS: [&str; 15] = [
    "bundles",
    "captured-bytes",
    "dependencies-per-bundle",
    "dependency-edges",
    "dependency-depth",
    "total-types",
    "total-fields",
    "alternatives-per-type",
    "total-alternatives",
    "choices-per-field",
    "total-choices",
    "type-depth",
    "value-depth",
    "value-nodes",
    "work",
];
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
/// Independent public projection work ceiling, separate from artifact bytes.
pub const MAX_VIEW_WORK: u64 = 100_000_000;
/// Independent retained public projection bytes, not a transport serialization limit.
pub const MAX_VIEW_OUTPUT_BYTES: u64 = 100_000_000;

/// Exact artifact/capture tuple, distinct from package releases and standalone source availability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionProfileDescriptor<'a> {
    /// Source behavior within an explicitly selected capture.
    pub source_profile: &'a str,
    /// Required captured-project envelope.
    pub capture_profile: &'a str,
    /// Complete logical project schema.
    pub project_ir: &'a str,
    /// Post-compilation public selection schema.
    pub view: &'a str,
    /// Whole-request outcome schema.
    pub result: &'a str,
    /// Independently decoded binary format.
    pub encoding: &'a str,
    /// Governed identity/transcript profile.
    pub identity: &'a str,
    /// Exact sorted required project features.
    pub required_features: &'a [&'a str],
}

/// Facts shared by explicit producers, independent readers and public views.
/// Bundle schemas/revisions remain per-bundle facts; a project may contain adapted and native leaves.
pub const COMPOSITION_PROFILE: CompositionProfileDescriptor<'static> =
    CompositionProfileDescriptor {
        source_profile: neutral_core::profile::V1_SOURCE_PROFILE,
        capture_profile: CAPTURE_REQUEST_VERSION,
        project_ir: PROJECT_IR_SCHEMA,
        view: PROJECT_VIEW_SCHEMA,
        result: PROJECT_RESULT_SCHEMA,
        encoding: ENCODING,
        identity: IDENTITY_PROFILE,
        required_features: REQUIRED_FEATURES,
    };
