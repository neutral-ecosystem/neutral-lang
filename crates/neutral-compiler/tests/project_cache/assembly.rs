// SPDX-License-Identifier: Apache-2.0

//! Assembly rejects incomplete checked inputs at the original declaration.

use super::super::{
    BTreeMap, BTreeSet, CancellationToken, graph, parse, validate_composition_bindings,
    validate_composition_scope,
};
use super::*;
use crate::tests::project_cache_support::capture_source;

/// Neither a missing materialized binding nor a missing nominal definition can publish declarations.
#[test]
fn assembly_rejects_missing_checked_declarations_with_source_location() {
    for body in ["public num answer = 42", "public record Item { num value }"] {
        let captured = capture_source(body);
        let cancel = CancellationToken::new();
        let mut resolver = Resolver {
            captured: &captured,
            graph: graph::build(&captured, &cancel).unwrap(),
            roots: BTreeMap::new(),
            types: BTreeMap::new(),
            definitions: Vec::new(),
            values: BTreeMap::new(),
            active: BTreeSet::new(),
            provenance: Vec::new(),
            work: 0,
            cancel: &cancel,
        };
        let roots = parse(&captured.sources()[0], &captured, &cancel).unwrap();
        assert_eq!(roots.len(), 1);
        let location = roots[0].location;
        for root in roots {
            resolver
                .roots
                .insert(root.owner.try_clone().unwrap(), root)
                .unwrap();
        }
        let scope = Arc::try_new(
            validate_composition_scope(
                captured.catalogue(),
                Vec::new(),
                captured.composition_limits(),
                &cancel,
            )
            .unwrap(),
        )
        .unwrap();
        let bindings = validate_composition_bindings(
            scope.clone(),
            &[],
            captured.composition_limits(),
            &cancel,
        )
        .unwrap();
        let Err(failure) = declarations(&mut resolver, &scope, &bindings) else {
            panic!("assembly accepted missing checked declaration: {body}");
        };
        assert_eq!(failure.code, codes::INCOMPATIBLE_VALUE, "{body}");
        assert_eq!(failure.location, Some(location), "{body}");
        assert_eq!(resolver.provenance, Vec::new());
    }
}
