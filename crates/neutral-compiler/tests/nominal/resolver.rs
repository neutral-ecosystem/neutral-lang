// SPDX-License-Identifier: Apache-2.0

//! Shared `/3` nominal resolver boundaries against current captured inputs.

use super::*;
use crate::test_support::capture_expression_source;
use neutral_ir::composition::{CompositionBody as B, CompositionDefinition, CompositionField};

fn resolver<'a>(
    captured: &'a crate::CapturedExpressionProject,
    cancel: &'a CancellationToken,
) -> Resolver<'a> {
    let context = &captured.composition;
    Resolver {
        captured: context,
        graph: graph::build_expression(context, cancel).unwrap(),
        roots: BTreeMap::new(),
        types: BTreeMap::new(),
        definitions: Vec::new(),
        work: 0,
        cancel,
    }
}

fn root(captured: &crate::CapturedExpressionProject, name: &str, ty: Option<TypeExpr>) -> Root {
    let source = &captured.sources()[0];
    Root {
        owner: ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(V1_SOURCE_PROFILE.to_owned(), source.module_id().to_owned()),
            name.to_owned(),
        ),
        public: true,
        location: SourceLocation::new(source.digest(), ByteSpan::new(0, 1).unwrap()),
        ty,
        fields: Vec::new(),
        tokens: Vec::new(),
        variant: false,
    }
}

#[test]
fn current_nominal_lookup_reports_the_original_missing_alias_occurrence() {
    let captured = capture_expression_source("num answer = 42");
    let cancel = CancellationToken::new();
    let mut resolver = resolver(&captured, &cancel);
    let root = root(&captured, "answer", Some(TypeExpr::Num));
    let span = ByteSpan::new(12, 19).unwrap();
    resolver
        .roots
        .insert(root.owner.clone(), root.clone())
        .unwrap();
    assert_eq!(
        resolver.name(&root, Some("missing"), "answer", span, false),
        Err(fail(
            codes::INCOMPATIBLE_VALUE,
            Some(SourceLocation::new(root.location.source(), span))
        )),
    );
}

#[test]
fn current_nominal_record_lookup_reports_unknown_member_at_the_object() {
    let captured = capture_expression_source("public record Item { num amount }");
    let cancel = CancellationToken::new();
    let mut resolver = resolver(&captured, &cancel);
    let item = root(&captured, "Item", None);
    let ty = T::Nominal(item.owner.clone());
    resolver.definitions.push(SourceCompositionDefinition {
        owner: item.owner.clone(),
        definition: CompositionDefinition {
            name: "Item".to_owned(),
            public: true,
            body: B::Record(vec![CompositionField {
                name: "amount".to_owned(),
                ty: T::Num,
                presence: FieldPresence::Required,
                restrictions: FieldRestrictions::default(),
                default: None,
            }]),
        },
    });
    let mut tokens =
        crate::frontend::lexer::lex_bounded(b"{ amount: 1, extra: 2 }", usize::MAX, None)
            .unwrap()
            .tokens;
    tokens.retain(|token| !matches!(token.kind, TokenKind::EndOfFile));
    assert_eq!(
        resolver.record_value(&item, &ty, &tokens, 0),
        Err(fail(codes::INCOMPATIBLE_VALUE, Some(item.location))),
    );
}

#[test]
fn current_nominal_work_overflow_preserves_the_previous_total_and_location() {
    let captured = capture_expression_source("num answer = 42");
    let cancel = CancellationToken::new();
    let mut resolver = resolver(&captured, &cancel);
    let location = root(&captured, "answer", Some(TypeExpr::Num)).location;
    resolver.work = u64::MAX;
    assert_eq!(
        resolver.step(1, Some(location)),
        Err(fail(codes::LIMIT, Some(location)))
    );
    assert_eq!(resolver.work, u64::MAX);
}

#[test]
fn nominal_default_copy_work_counts_nested_values_restrictions_and_cancellation() {
    let cancel = CancellationToken::new();
    let number = ExactNumber::from_source("12", 16, 16).unwrap();
    let value: V = V::Record(vec![
        ("missing".to_owned(), None),
        (
            "present".to_owned(),
            Some(V::List(vec![V::Number(number.clone()), V::Bool(true)])),
        ),
    ]);
    assert_eq!(value_clone_work(&value, &cancel, symbol_work).unwrap(), 24);
    let body = B::Record(vec![CompositionField {
        name: "amount".to_owned(),
        ty: T::Nullable(Box::new(T::List(Box::new(T::Num)))),
        presence: FieldPresence::Defaulted,
        restrictions: FieldRestrictions {
            minimum: Some(number.clone()),
            maximum: Some(number.clone()),
            choices: Some(vec![CompositionValue::Number(number.clone())]),
            ..FieldRestrictions::default()
        },
        default: Some(CompositionValue::Number(number)),
    }]);
    assert_eq!(body_clone_work(&body, &cancel).unwrap(), 21);
    cancel.cancel();
    assert_eq!(
        body_clone_work(&body, &cancel),
        Err(fail(codes::CANCELLED, None))
    );
}
