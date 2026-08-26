// SPDX-License-Identifier: Apache-2.0

//! Central private spellings for the frozen Neutral language surface.
//!
//! This module is the single compiler-internal source for language words. The
//! lexer, parser, and semantics consume these constants so their recognition
//! and protected-name behavior cannot drift independently.

/// Frozen language-word constants grouped by source-language responsibility.
pub(crate) mod names {
    /// Canonical document-header keyword.
    pub(crate) const NEU: &str = "neu";
    /// Canonical module-header keyword.
    pub(crate) const MODULE: &str = "module";
    /// Exact numeric core-type spelling.
    pub(crate) const NUM: &str = "num";
    /// Exact string core-type spelling.
    pub(crate) const STRING: &str = "string";
    /// Exact Boolean core-type spelling.
    pub(crate) const BOOL: &str = "bool";
    /// Exact list core-type spelling.
    pub(crate) const LIST: &str = "List";
    /// Exact identity-reference core-type spelling.
    pub(crate) const REF_TYPE: &str = "Ref";
    /// Vocabulary-import declaration keyword.
    pub(crate) const USE: &str = "use";
    /// Record declaration keyword.
    pub(crate) const RECORD: &str = "record";
    /// Boolean true literal spelling.
    pub(crate) const TRUE: &str = "true";
    /// Boolean false literal spelling.
    pub(crate) const FALSE: &str = "false";
    /// Null literal spelling.
    pub(crate) const NULL: &str = "null";
    /// Identity-reference constructor spelling.
    pub(crate) const REF: &str = "ref";

    /// All frozen core spellings that source declarations cannot redeclare.
    pub(crate) const PROTECTED_CORE_NAMES: [&str; 13] = [
        NUM, STRING, BOOL, LIST, REF_TYPE, NEU, MODULE, USE, RECORD, TRUE, FALSE, NULL, REF,
    ];

    /// Returns whether a spelling belongs to the frozen protected core set.
    pub(crate) fn is_protected_name(value: &str) -> bool {
        PROTECTED_CORE_NAMES.contains(&value)
    }
}
