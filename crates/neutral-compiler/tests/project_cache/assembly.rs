// SPDX-License-Identifier: Apache-2.0

//! Current `/3` captured-companion facts remain separate from syntax semantics.

use super::super::{BTreeMap, Resolver, graph};
use super::*;
use crate::project_cache_support::capture_expression_source;
use neutral_core::CancellationToken;

#[test]
fn current_companion_projection_preserves_canonical_sources_and_modules() {
    let captured = capture_expression_source("num answer = 42");
    let context = &captured.composition;
    let cancel = CancellationToken::new();
    let resolver = Resolver {
        captured: context,
        graph: graph::build_expression(context, &cancel).unwrap(),
        roots: BTreeMap::new(),
        types: BTreeMap::new(),
        definitions: Vec::new(),
        work: 0,
        cancel: &cancel,
    };
    let companions = captured_companions(&resolver).unwrap();
    assert_eq!(companions.modules.len(), 1);
    assert_eq!(companions.modules[0].identity.module_name(), "example");
    assert_eq!(companions.modules[0].imports, Vec::<String>::new());
    assert_eq!(companions.sources.len(), 1);
    assert_eq!(companions.sources[0].module, "example");
    assert_eq!(companions.sources[0].source_id, "unit");
    assert_eq!(companions.sources[0].digest, context.sources()[0].digest());
    assert_eq!(
        companions.sources[0].byte_len,
        context.sources()[0].bytes().len() as u64
    );
    assert_eq!(
        companions.vocabulary_sources,
        [] as [neutral_ir::project::ProjectVocabularySource; 0]
    );
}
