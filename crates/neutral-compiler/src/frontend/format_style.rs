// SPDX-License-Identifier: Apache-2.0

//! Canonical source-formatting spellings shared by the private formatter.

/// One canonical indentation level.
pub(super) const INDENT: &str = "    ";
/// Canonical logical newline spelling.
pub(super) const NEWLINE: char = '\n';
/// Canonical separator between root source blocks.
pub(super) const ROOT_SEPARATOR: &str = "\n\n";
/// Canonical separator between the language and module headers.
pub(super) const HEADER_SEPARATOR: char = '\n';
/// Canonical language header prefix.
pub(super) const LANGUAGE_HEADER_PREFIX: &str = "neu \"";
/// Canonical language header suffix.
pub(super) const LANGUAGE_HEADER_SUFFIX: char = '"';
/// Canonical module header prefix.
pub(super) const MODULE_PREFIX: &str = "module ";
/// Canonical vocabulary requirement prefix.
pub(super) const USE_PREFIX: &str = "use ";
/// Canonical nominal-record declaration prefix.
pub(super) const RECORD_PREFIX: &str = "record ";
/// Canonical binding initializer separator.
pub(super) const INITIALIZER_SEPARATOR: &str = " = ";
/// Canonical field type/name separator.
pub(super) const TYPE_NAME_SEPARATOR: char = ' ';
/// Canonical field name/value separator.
pub(super) const FIELD_VALUE_SEPARATOR: &str = ": ";
/// Canonical qualified-name separator.
pub(super) const QUALIFIER_SEPARATOR: &str = "::";
/// Canonical comma suffix for multiline members.
pub(super) const MEMBER_SUFFIX: char = ',';
