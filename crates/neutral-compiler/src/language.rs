// SPDX-License-Identifier: Apache-2.0

//! Central private spellings for the frozen Neutral language surface.
//!
//! This module is the single compiler-internal source for language words. The
//! lexer, parser, and semantics consume these constants so their recognition
//! and protected-name behavior cannot drift independently.
//!
//! Shared core words are reexports of the IR spelling contract; graph words remain
//! contextual identifiers in the small raw lexer. Recognizing an identifier here
//! does not resolve a module, establish visibility or allow redeclaration. The
//! expression parser owns its additional contextual grammar, and exact source/
//! semantic profiles are selected by capture rather than by these keyword groups.

/// Frozen language-word constants grouped by source-language responsibility.
pub(crate) mod names {
    pub(crate) use neutral_ir::language::{
        BOOL, FALSE, LIST, MODULE, NEU, NULL, NUM, RECORD, REF, REF_TYPE, STRING, TRUE, USE,
        is_protected_name,
    };
}

/// Exact graph keywords kept separate from the shared core token vocabulary.
pub(crate) mod graph_names {
    /// Starts one logical module import.
    pub(crate) const IMPORT: &str = "import";
    /// Introduces the required local alias.
    pub(crate) const AS: &str = "as";
    /// Public declaration prefix; imports cannot be re-exported.
    pub(crate) const PUBLIC: &str = "public";
    pub(crate) use neutral_ir::language::{PATH, URL};
}
