// SPDX-License-Identifier: Apache-2.0

//! Raw original-byte expression/expansion evidence, separate from logical meaning.

use crate::ModuleSymbolIdentity;
use neutral_core::SourceLocation;

/// One canonical tree occurrence; readers independently check path and location ownership.
#[derive(Debug, Eq, PartialEq)]
pub struct ExpressionAttribution {
    /// Declaration or public-template owner.
    pub owner: ModuleSymbolIdentity,
    /// Canonical child-index path, not a dotted source string.
    pub node_path: Vec<u64>,
    /// Original-byte occurrence, or explicit absence of source evidence.
    pub occurrence: Option<SourceLocation>,
    /// Related checked public occurrences; private origins are redacted in views.
    pub related_public_locations: Vec<SourceLocation>,
}

/// One generated declaration's real source lineage.
#[derive(Debug, Eq, PartialEq)]
pub struct ExpansionAttribution {
    /// Stable generated declaration owner.
    pub owner: ModuleSymbolIdentity,
    /// Original application occurrence.
    pub application: SourceLocation,
    /// Original template definition occurrence.
    pub definition: SourceLocation,
    /// Original argument occurrences in parameter order.
    pub arguments: Vec<SourceLocation>,
    /// Original axis positions, empty for plain instantiation.
    pub original_axis_indices: Vec<u64>,
}
