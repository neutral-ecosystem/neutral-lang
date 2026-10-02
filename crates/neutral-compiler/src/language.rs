// SPDX-License-Identifier: Apache-2.0

//! Central private spellings for the frozen Neutral language surface.
//!
//! This module is the single compiler-internal source for language words. The
//! lexer, parser, and semantics consume these constants so their recognition
//! and protected-name behavior cannot drift independently.

/// Frozen language-word constants grouped by source-language responsibility.
pub(crate) mod names {
    pub(crate) use neutral_ir::language::{
        BOOL, FALSE, LIST, MODULE, NEU, NULL, NUM, RECORD, REF, REF_TYPE, SOURCE_LANGUAGE_VERSION,
        STRING, TRUE, USE, is_protected_name,
    };
}

/// Exact v1 graph keywords kept separate from the frozen v0 token vocabulary.
pub(crate) mod graph_names {
    /// Starts one logical module import.
    pub(crate) const IMPORT: &str = "import";
    /// Introduces the required local alias.
    pub(crate) const AS: &str = "as";
    /// Public declaration prefix; imports cannot be re-exported.
    pub(crate) const PUBLIC: &str = "public";
    pub(crate) use neutral_ir::language::{PATH, URL};
}
