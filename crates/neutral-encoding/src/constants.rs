// SPDX-License-Identifier: Apache-2.0

//! Shared hard bounds for complete project codecs.

/// Maximum complete artifact size.
pub const MAXIMUM_ARTIFACT_BYTES: usize = 67_108_864;
/// Maximum encoded section size.
pub const MAXIMUM_SECTION_BYTES: usize = 67_108_864;
/// Maximum nested type/value layers.
pub const MAXIMUM_NESTING_DEPTH: usize = 128;
/// Maximum entries in one array or map.
pub const MAXIMUM_CONTAINER_ITEMS: usize = 1_000_000;
/// Maximum UTF-8 bytes in one text item.
pub const MAXIMUM_TEXT_BYTES: usize = 16_777_216;
/// Maximum bytes in one byte-string item.
pub const MAXIMUM_BYTE_STRING_BYTES: usize = 16_777_216;
/// Maximum exact-number coefficient digits.
pub const MAXIMUM_EXACT_NUMBER_DIGITS: usize = 1_000_000;
/// Maximum recursively traversed logical nodes.
pub const MAXIMUM_TRAVERSAL_NODES: usize = 1_000_000;

/// Type tuple spellings shared with the project `/1` codec.
pub mod kind {
    /// Nullable type.
    pub const NULLABLE: &str = "nullable";
}
