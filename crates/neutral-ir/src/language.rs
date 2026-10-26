// SPDX-License-Identifier: Apache-2.0

//! Frozen source and logical-name spellings shared by Neutral producers and readers.
//!
//! These helpers answer narrow lexical questions, not complete source validity.
//! A valid name can still be reserved, private, duplicated or unresolved in its
//! owning scope. Contextual expression/composition keywords and generated binding
//! suffixes are governed by their own modules rather than added implicitly to
//! the frozen protected-name set here.
//!
//! Exact release versions accept only three canonical unsigned decimal components;
//! they are deliberately narrower than general `SemVer` ranges or prerelease syntax.
//! Feature identifiers are inert spellings, not dynamically loaded capabilities:
//! their permitted alphabet alone establishes neither support nor a size budget.

/// Canonical source-level language version spelling.
pub use neutral_core::profile::V0_SOURCE_PROFILE as SOURCE_LANGUAGE_VERSION;
/// Canonical document-header keyword.
pub const NEU: &str = "neu";
/// Canonical module-header keyword.
pub const MODULE: &str = "module";
/// Exact numeric core-type spelling.
pub const NUM: &str = "num";
/// Exact string core-type spelling.
pub const STRING: &str = "string";
/// Exact Boolean core-type spelling.
pub const BOOL: &str = "bool";
/// Inert URL scalar spelling in the v1 project profile.
pub const URL: &str = "url";
/// Inert path scalar spelling in the v1 project profile.
pub const PATH: &str = "path";
/// Exact list core-type spelling.
pub const LIST: &str = "List";
/// Exact identity-reference core-type spelling.
pub const REF_TYPE: &str = "Ref";
/// Vocabulary-import declaration keyword.
pub const USE: &str = "use";
/// Record declaration keyword.
pub const RECORD: &str = "record";
/// Boolean true literal spelling.
pub const TRUE: &str = "true";
/// Boolean false literal spelling.
pub const FALSE: &str = "false";
/// Null literal spelling.
pub const NULL: &str = "null";
/// Identity-reference constructor spelling.
pub const REF: &str = "ref";

/// All frozen core spellings that source and external declarations cannot redeclare.
pub const PROTECTED_CORE_NAMES: [&str; 13] = [
    NUM, STRING, BOOL, LIST, REF_TYPE, NEU, MODULE, USE, RECORD, TRUE, FALSE, NULL, REF,
];

/// Returns whether a spelling belongs to the frozen protected core set.
#[must_use]
pub fn is_protected_name(value: &str) -> bool {
    PROTECTED_CORE_NAMES.contains(&value)
}

/// Returns whether a vocabulary or nominal type name is uppercase-leading ASCII.
#[must_use]
pub fn is_upper_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z')) && bytes.all(|byte| byte.is_ascii_alphanumeric())
}

/// Returns whether a module, binding, or field name is canonical lower snake case ASCII.
/// Requires a lowercase first byte and alphanumeric final byte, with no doubled
/// underscores. This does not recognize expression-generated matrix suffixes.
#[must_use]
pub fn is_snake_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
        && !bytes.windows(2).any(|pair| pair == b"__")
}

/// Returns whether text is one exact canonical three-part release version.
#[must_use]
pub fn is_exact_release_version(value: &str) -> bool {
    let mut parts = value.split('.');
    let valid = parts.by_ref().take(3).all(is_canonical_unsigned_component);
    // all() also accepts a short iterator; the separator count rules out missing components.
    valid && parts.next().is_none() && value.matches('.').count() == 2
}

/// Returns whether one release-version component is canonical ASCII decimal.
fn is_canonical_unsigned_component(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

/// Returns whether a structural feature ID is bounded to inert qualified ASCII.
#[must_use]
pub fn is_feature_id(value: &str) -> bool {
    !value.is_empty()
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'/' | b':' | b'@')
        })
}
