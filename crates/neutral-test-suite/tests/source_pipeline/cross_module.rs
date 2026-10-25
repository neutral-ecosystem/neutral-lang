// SPDX-License-Identifier: Apache-2.0

//! Literal cross-module semantics through complete encoding and independent reading.

use super::*;
use neutral_ir::project_identity::{IdentityLimits, canonical_expression_project};
use neutral_reader::expression::ExpressionViewRequest;

/// An import SCC is valid when its actual value dependencies are acyclic.
#[test]
fn integration_import_cycle_has_literal_values_after_independent_decode() {
    let units = [
        source!("positive", "cycle-left", "cycle::left"),
        source!("positive", "cycle-right", "cycle::right"),
    ];
    let (_, bytes, decoded) = pipeline(&units);
    assert_eq!(materialized(&decoded, "cycle::left", "seed"), &number("42"));
    assert_eq!(
        materialized(&decoded, "cycle::right", "answer"),
        &number("42")
    );
    assert_eq!(
        decoded
            .ir()
            .modules
            .iter()
            .map(|m| (m.identity.module_name(), m.imports.as_slice()))
            .collect::<Vec<_>>(),
        [
            ("cycle::left", &["cycle::right".to_owned()][..]),
            ("cycle::right", &["cycle::left".to_owned()][..]),
        ]
    );
    let reversed = units.into_iter().rev().collect::<Vec<_>>();
    assert_eq!(pipeline(&reversed).1, bytes);
}

/// Two import paths retain one canonical provider and exact reference targets, not alias identities.
#[test]
fn integration_diamond_imports_preserve_values_types_references_and_public_closure() {
    let units = [
        source!("positive", "diamond-main", "diamond::main"),
        source!("positive", "diamond-left", "diamond::left"),
        source!("positive", "diamond-right", "diamond::right"),
        source!("positive", "diamond-base", "diamond::base"),
    ];
    let (_, bytes, decoded) = pipeline(&units);
    assert_eq!(
        materialized(&decoded, "diamond::main", "total"),
        &number("85")
    );
    let expected = V::Record(vec![("count".into(), Some(number("40")))]);
    for module in ["diamond::left", "diamond::right"] {
        assert_eq!(materialized(&decoded, module, "reused"), &expected);
        assert_eq!(
            materialized(&decoded, module, "pointer"),
            &V::Reference(symbol("diamond::base", "seed"))
        );
    }
    assert_eq!(
        decoded
            .ir()
            .declarations
            .iter()
            .filter(|d| d.identity == symbol("diamond::base", "seed"))
            .count(),
        1
    );
    let roots = [
        symbol("diamond::left", "pointer"),
        symbol("diamond::right", "pointer"),
    ];
    let view = decoded
        .derive_view(
            &ExpressionViewRequest {
                schema: profile::PROJECT_VIEW_SCHEMA,
                semantic_profiles: &decoded.ir().semantic_profiles,
                roots: &roots,
            },
            &CancellationToken::new(),
        )
        .unwrap();
    let mut owners = view
        .declarations()
        .map(|d| d.identity.clone())
        .collect::<Vec<_>>();
    owners.sort();
    assert_eq!(
        owners,
        [
            symbol("diamond::base", "Item"),
            symbol("diamond::base", "seed"),
            symbol("diamond::left", "pointer"),
            symbol("diamond::right", "pointer")
        ]
    );
    let reversed = units.into_iter().rev().collect::<Vec<_>>();
    assert_eq!(pipeline(&reversed).1, bytes);
}

/// Alias spelling changes captured evidence but not imported meaning or reference identity.
#[test]
fn property_import_alias_renaming_keeps_literal_meaning_and_logical_identity() {
    let (_, bytes, original) = pipeline(&[
        source!("positive", "consumer", "app::consumer"),
        source!("positive", "shared", "app::shared"),
    ]);
    let (_, renamed_bytes, renamed) = pipeline(&[
        source!("positive", "consumer-renamed", "app::consumer"),
        source!("positive", "shared", "app::shared"),
    ]);
    assert_ne!(bytes, renamed_bytes);
    for project in [&original, &renamed] {
        assert_eq!(
            materialized(project, "app::consumer", "answer"),
            &number("42")
        );
        assert_eq!(
            materialized(project, "app::consumer", "pointer"),
            &V::Reference(symbol("app::shared", "api"))
        );
    }
    let limits = IdentityLimits {
        bytes: 1_048_576,
        nodes: 1_048_576,
    };
    let cancel = CancellationToken::new();
    assert_eq!(
        canonical_expression_project(original.ir(), limits, &cancel)
            .unwrap()
            .identity(),
        canonical_expression_project(renamed.ir(), limits, &cancel)
            .unwrap()
            .identity()
    );
}

/// Public nominal contracts do not permit naming a private reference target across an import.
#[test]
fn integration_cross_module_private_reference_rejects_at_the_consuming_declaration() {
    let units = [
        source!("positive", "access-provider", "access::provider"),
        source!("negative", "cross-private-reference", "access::consumer"),
    ];
    let captured = capture(&units);
    let failure = compile_expression_project(&captured, &CancellationToken::new()).unwrap_err();
    assert_eq!(failure.code, "NEU-EXP-003");
    let location = failure
        .location
        .expect("the consuming reference has a source occurrence");
    let consumer = units[1].1.as_bytes();
    assert_eq!(
        location.source(),
        neutral_core::SourceContentDigest::from_bytes(consumer)
    );
    let start = usize::try_from(location.span().start()).unwrap();
    let end = usize::try_from(location.span().end()).unwrap();
    assert_eq!(
        &consumer[start..end],
        b"public Ref<provider::Item> pointer = ref(provider::hidden)"
    );
    let reversed = units.into_iter().rev().collect::<Vec<_>>();
    let reordered =
        compile_expression_project(&capture(&reversed), &CancellationToken::new()).unwrap_err();
    assert_eq!(reordered, failure);
}

/// An imported public envelope cannot conceal a private member type in its provider.
#[test]
fn integration_cross_module_public_type_leak_never_publishes_a_project() {
    let units = [
        source!("negative", "cross-private-provider", "leak::provider"),
        source!("negative", "cross-private-consumer", "leak::consumer"),
    ];
    let failure =
        compile_expression_project(&capture(&units), &CancellationToken::new()).unwrap_err();
    assert_eq!(failure.code, "NEU-COM-010");
    let reversed = units.into_iter().rev().collect::<Vec<_>>();
    let reordered =
        compile_expression_project(&capture(&reversed), &CancellationToken::new()).unwrap_err();
    assert_eq!(reordered, failure);
}
