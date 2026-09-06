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
