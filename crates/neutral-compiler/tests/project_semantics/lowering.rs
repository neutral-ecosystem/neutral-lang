// SPDX-License-Identifier: Apache-2.0

//! Private contextual-lowering boundary tests within the project semantic criterion.

use super::*;
use neutral_core::SourceContentDigest;

/// Token splitting rejects incomplete lists and extra closing delimiters independently.
#[test]
fn split_items_rejects_unbalanced_and_empty_children() {
    for source in [")", "[", ",1", "1,,2", "1,]"] {
        let mut tokens = super::super::lexer::lex(source.as_bytes()).unwrap().tokens;
        tokens.pop();
        assert!(
            matches!(
                split_items(&tokens),
                Err(ProjectCompileFailure::Lowering {
                    code: codes::INVALID_VALUE,
                    location: None,
                })
            ),
            "{source}"
        );
    }
}

/// Name resolution in contextual lowering fails with the original occurrence coordinates.
#[test]
fn lowering_rejects_unresolved_name_at_original_occurrence() {
    let (captured, _) = super::super::tests::project(&[("api", "num value = 1")]);
    let vocabularies = validate_project_vocabularies(&captured).unwrap();
    let roots = BTreeMap::new();
    let digest = SourceContentDigest::from_bytes(b"missing");
    let modules = BTreeMap::from([(
        "api".to_owned(),
        ModuleContext {
            digest,
            aliases: BTreeMap::new(),
        },
    )]);
    let cancellation = CancellationToken::new();
    let mut lowering = Lowering {
        roots: &roots,
        modules: &modules,
        vocabularies: &vocabularies,
        values: BTreeMap::new(),
        defaults: BTreeMap::new(),
        cancellation: &cancellation,
        remaining: 4096,
        numeric_limit: 4096,
    };
    let mut tokens = super::super::lexer::lex(b"missing").unwrap().tokens;
    tokens.pop();
    let ProjectCompileFailure::Lowering {
        code,
        location: Some(location),
    } = lowering
        .value(&tokens, &ProjectPublicType::Num, "api", false, 0)
        .unwrap_err()
    else {
        panic!("unresolved name must retain its source location");
    };
    assert_eq!(code, codes::INVALID_VALUE);
    assert_eq!(location, SourceLocation::new(digest, tokens[0].span));
}

/// A default without its declared field type cannot publish partially materialized defaults.
#[test]
fn lowering_rejects_default_without_field_type() {
    let (captured, _) = super::super::tests::project(&[("api", "record Item { num count = 1 }")]);
    let source = &captured.sources()[0];
    let mut root = parse_roots("api", source.digest(), source.bytes())
        .unwrap()
        .remove(0);
    let identity = root.symbol.identity().clone();
    root.fields.clear();
    let roots = BTreeMap::from([(key(&identity), root)]);
    let vocabularies = validate_project_vocabularies(&captured).unwrap();
    let modules = BTreeMap::from([(
        "api".to_owned(),
        ModuleContext {
            digest: source.digest(),
            aliases: BTreeMap::new(),
        },
    )]);
    let cancellation = CancellationToken::new();
    let mut lowering = Lowering {
        roots: &roots,
        modules: &modules,
        vocabularies: &vocabularies,
        values: BTreeMap::new(),
        defaults: BTreeMap::new(),
        cancellation: &cancellation,
        remaining: 4096,
        numeric_limit: 4096,
    };
    assert!(matches!(
        lowering.record_defaults(&identity, 0),
        Err(ProjectCompileFailure::Lowering {
            code: codes::INVALID_VALUE,
            location: None,
        })
    ));
    assert!(lowering.defaults.is_empty());
}
