// SPDX-License-Identifier: Apache-2.0

//! Canonical source-formatting spellings shared by the private formatter.

/// One canonical indentation level.
pub(super) const INDENT: &str = "    ";
/// Canonical logical newline spelling.
pub(super) const NEWLINE: char = '\n';
/// Text form required by string replacement operations.
pub(super) const NEWLINE_TEXT: &str = "\n";
/// Accepted two-byte physical newline, normalized before rendering comments.
pub(super) const CARRIAGE_RETURN_LINE_FEED: &str = "\r\n";
/// Accepted carriage-return character and string escape source.
pub(super) const CARRIAGE_RETURN: char = '\r';
/// Canonical separator between root source blocks.
pub(super) const ROOT_SEPARATOR: &str = "\n\n";
/// Canonical separator between the language and module headers.
pub(super) const HEADER_SEPARATOR: char = NEWLINE;
/// Canonical binding initializer separator.
pub(super) const INITIALIZER_SEPARATOR: &str = " = ";
/// Canonical separator between source words and declaration delimiters.
pub(super) const WORD_SEPARATOR: char = ' ';
/// Canonical field name/value separator.
pub(super) const FIELD_VALUE_SEPARATOR: &str = ": ";
/// Canonical qualified-name separator.
pub(super) const QUALIFIER_SEPARATOR: &str = "::";
/// Canonical comma suffix for multiline members.
pub(super) const MEMBER_SUFFIX: char = ',';
/// Opens a record declaration or record value.
pub(super) const RECORD_OPEN: char = '{';
/// Closes a record declaration or record value.
pub(super) const RECORD_CLOSE: char = '}';
/// Opens an ordered list value.
pub(super) const LIST_OPEN: char = '[';
/// Closes an ordered list value.
pub(super) const LIST_CLOSE: char = ']';
/// Marks an outer nullable type.
pub(super) const NULLABLE_SUFFIX: char = '?';
/// Opens a list/reference type argument.
pub(super) const TYPE_ARGUMENT_OPEN: char = '<';
/// Closes a list/reference type argument.
pub(super) const TYPE_ARGUMENT_CLOSE: char = '>';
/// Opens an identity-reference constructor argument.
pub(super) const REFERENCE_OPEN: char = '(';
/// Closes an identity-reference constructor argument.
pub(super) const REFERENCE_CLOSE: char = ')';
/// Surrounds a source string literal.
pub(super) const STRING_DELIMITER: char = '"';
/// Introduces a string escape.
pub(super) const ESCAPE_MARKER: char = '\\';
/// Decoded tab character with a canonical short escape.
pub(super) const TAB: char = '\t';
/// Decoded null character with a canonical short escape.
pub(super) const NULL_CHARACTER: char = '\0';
/// Canonical escaped string delimiter.
pub(super) const ESCAPED_QUOTE: &str = "\\\"";
/// Canonical escaped backslash.
pub(super) const ESCAPED_BACKSLASH: &str = "\\\\";
/// Canonical escaped line feed.
pub(super) const ESCAPED_NEWLINE: &str = "\\n";
/// Canonical escaped carriage return.
pub(super) const ESCAPED_CARRIAGE_RETURN: &str = "\\r";
/// Canonical escaped tab.
pub(super) const ESCAPED_TAB: &str = "\\t";
/// Canonical escaped null.
pub(super) const ESCAPED_NULL: &str = "\\0";
/// Introduces a lowercase hexadecimal Unicode scalar escape.
pub(super) const UNICODE_ESCAPE_PREFIX: &str = "\\u{";
/// Closes the Unicode scalar's hexadecimal digits.
pub(super) const UNICODE_ESCAPE_SUFFIX: char = RECORD_CLOSE;
