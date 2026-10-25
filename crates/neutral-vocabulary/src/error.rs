// SPDX-License-Identifier: Apache-2.0

//! Closed safe vocabulary failure classes and deterministic diagnostics.

/// Bounded strict bundle validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VocabularyError {
    /// Cooperative cancellation of explicitly cancellable composition JSON decoding.
    Cancelled,
    /// A limit builder received zero.
    InvalidLimits,
    /// Captured bytes exceeded the configured bound.
    BundleByteLimitExceeded,
    /// Exact captured bytes did not match the lock digest.
    DigestMismatch,
    /// A UTF-8 BOM appeared before JSON.
    BomForbidden,
    /// Captured bytes were not strict UTF-8.
    InvalidUtf8,
    /// JSON syntax was malformed, truncated, or extended.
    MalformedJson,
    /// A duplicate object member appeared before map collapse.
    DuplicateJsonMember,
    /// JSON allocation/depth/node limits were exceeded.
    JsonLimitExceeded,
    /// A raw JSON numeric token appeared.
    RawJsonNumberForbidden,
    /// The top-level JSON value was not an object.
    InvalidEnvelope,
    /// A closed-schema object contained an unknown member.
    UnknownMember,
    /// An object omitted a required member.
    MissingMember,
    /// A member used the wrong JSON shape.
    InvalidMemberType,
    /// A member attempted to introduce executable behavior.
    ExecutableShapeForbidden,
    /// The format discriminator was unsupported.
    UnsupportedFormat,
    /// The encoding version was unsupported or mismatched.
    UnsupportedEncodingVersion,
    /// The logical schema version was unsupported or mismatched.
    UnsupportedSchemaVersion,
    /// Bundle identity, release, or feature facts disagreed with the lock.
    LockMismatch,
    /// A logical vocabulary identity was invalid.
    InvalidIdentity,
    /// A vocabulary release identifier was invalid.
    InvalidVersion,
    /// A structural feature ID was malformed.
    InvalidFeatureId,
    /// A required feature appeared more than once.
    DuplicateFeature,
    /// A required feature was not permitted by exact lock facts.
    UnknownRequiredFeature,
    /// A nominal type appeared more than once.
    DuplicateType,
    /// A nominal type name was invalid or protected.
    InvalidTypeName,
    /// A field appeared more than once.
    DuplicateField,
    /// A field name was invalid or protected.
    InvalidFieldName,
    /// A tagged type was unknown or malformed.
    InvalidType,
    /// A nominal type target did not exist.
    UnknownTypeTarget,
    /// Embedded nominal records formed a cycle outside `ref`.
    InvalidTypeRecursion,
    /// A public v1 vocabulary signature transitively names a private type.
    PrivateTypeExposed,
    /// A closed default was malformed, incompatible, incomplete, or contained `ref`.
    InvalidDefault,
}
