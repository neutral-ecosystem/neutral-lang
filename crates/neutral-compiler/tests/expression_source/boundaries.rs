// SPDX-License-Identifier: Apache-2.0

//! Observable source contracts for parsing, typing, finite expansion and lowering.

use super::*;
use neutral_ir::composition::{ValueOriginKind, ValuePathSegment};

/// Guard operators belong to the conditional primary while surrounding operators keep precedence.
#[test]
fn unparenthesized_compound_guards_preserve_surrounding_operator_precedence() {
    for (expression, expected) in [
        ("if 1 > 0 { 2 } else { 3 }", "2"),
        ("if 1 > 0 && true { 2 } else { 3 }", "2"),
        ("if false || 1 > 0 { 2 } else { 3 }", "2"),
        ("if 1 == 1 { 2 } else { 3 }", "2"),
        ("1 + if 1 > 0 { 2 } else { 3 }", "3"),
        ("if 1 > 0 { 2 } else { 3 } + 3", "5"),
        ("if 1 > 0 { 2 } else { 3 } * 4", "8"),
        ("if 1 < 0 { 1 / 0 } else { 3 }", "3"),
    ] {
        let ir = compile(&format!("num result = {expression}"))
            .unwrap_or_else(|e| panic!("{expression}: {e:?}"));
        assert_eq!(value(&ir, "result"), &number(expected), "{expression}");
    }
}

/// A compound guard never bypasses complete branch or guard typing.
#[test]
fn compound_guards_reject_unknown_names_wrong_branch_types_and_chained_relations() {
    for (expression, code) in [
        ("if 1 > 0 { 2 } else { missing }", "NEU-EXP-003"),
        ("if 1 > 0 { 2 } else { false }", "NEU-EXP-004"),
        ("if 1 > false { 2 } else { 3 }", "NEU-EXP-004"),
        ("if 1 < 2 < 3 { 2 } else { 3 }", "NEU-EXP-002"),
    ] {
        assert_eq!(
            compile(&format!("num result = {expression}"))
                .unwrap_err()
                .code,
            code,
            "{expression}"
        );
    }
}

/// Nullable literals and scalar string contexts retain their declared value categories.
#[test]
fn nullable_lists_and_inert_url_path_literals_keep_exact_values() {
    let ir = compile("num? absent = null\nnum? present = 3\nList<num> empty = []\nurl endpoint = \"not a URL %zz\"\npath file = \"../opaque\"\nstring rendered = text(true) ++ text(2.50) ++ text(\"!\")").unwrap();
    assert_eq!(value(&ir, "absent"), &V::Null);
    assert_eq!(value(&ir, "present"), &number("3"));
    assert_eq!(value(&ir, "empty"), &V::List(Vec::new()));
    assert_eq!(value(&ir, "endpoint"), &V::Url("not a URL %zz".into()));
    assert_eq!(value(&ir, "file"), &V::Path("../opaque".into()));
    assert_eq!(value(&ir, "rendered"), &V::String("true2.5!".into()));
}

/// Nullable list construction uses its explicit element context even for an empty list.
#[test]
fn nullable_list_constructor_retains_declared_nullable_type() {
    let ir = compile("List<num>? empty = []\nList<num>? populated = [1, 2]").unwrap();
    assert_eq!(value(&ir, "empty"), &V::List(Vec::new()));
    assert_eq!(
        value(&ir, "populated"),
        &V::List(vec![number("1"), number("2")])
    );
    assert!(
        ir.declarations
            .iter()
            .all(|d| matches!(d.signature, S::Binding(T::Nullable(_))))
    );
}

/// Contextual nominal construction may inhabit the declared outer nullable type.
#[test]
fn nullable_record_constructor_preserves_supplied_fields_and_null() {
    let ir =
        compile("record Item { num count }\nItem? absent = null\nItem? present = { count: 3 }")
            .unwrap();
    assert_eq!(value(&ir, "absent"), &V::Null);
    assert_eq!(
        value(&ir, "present"),
        &V::Record(vec![("count".into(), Some(number("3")))])
    );
    for declaration in ir.declarations.iter().filter(|d| d.body.is_some()) {
        assert!(
            matches!(&declaration.signature, S::Binding(T::Nullable(inner)) if matches!(inner.as_ref(), T::Nominal(owner) if owner.declaration_name() == "Item"))
        );
    }
    assert_eq!(
        compile("record Item { num count }\nItem? present = { count: false }")
            .unwrap_err()
            .code,
        "NEU-EXP-004"
    );
}

/// A nullable variant constructor retains its exact tag and payload contract.
#[test]
fn nullable_variant_constructor_preserves_tag_payload_and_null() {
    let ir = compile("variant Choice { num ready, string failed }\nChoice? absent = null\nChoice? present = { tag: \"ready\", payload: 3 }").unwrap();
    assert_eq!(value(&ir, "absent"), &V::Null);
    assert_eq!(
        value(&ir, "present"),
        &V::Variant {
            tag: "ready".into(),
            payload: Box::new(number("3"))
        }
    );
    for declaration in ir.declarations.iter().filter(|d| d.body.is_some()) {
        assert!(
            matches!(&declaration.signature, S::Binding(T::Nullable(inner)) if matches!(inner.as_ref(), T::Nominal(owner) if owner.declaration_name() == "Choice"))
        );
    }
    assert_eq!(compile("variant Choice { num ready, string failed }\nChoice? present = { tag: \"ready\", payload: false }").unwrap_err().code, "NEU-EXP-004");
}

/// Empty lists need an expected element type before comparison or matrix expansion.
#[test]
fn inferred_empty_list_requires_element_type() {
    assert_eq!(
        compile("bool item = [] == [1]").unwrap_err().code,
        "NEU-EXP-004"
    );
    assert_eq!(
        compile("template Item(static num value) -> num { value }\nList<num> empty = []\nexpand items = Item(value: axis) for (axis in empty)")
            .unwrap()
            .declarations
            .len(),
        1
    );
    assert!(compile("List<num> item = [] ++ [1]").is_ok());
}

/// Record field names alone cannot choose a nominal constructor context.
#[test]
fn context_free_record_constructor_requires_nominal_type() {
    let body = "record Item { num count }\nbool item = { count: 1 } == { count: 1 }";
    assert_eq!(compile(body).unwrap_err().code, "NEU-EXP-004");
    assert!(compile("record Item { num count }\nItem item = { count: 1 }").is_ok());
}

/// Variant constructors require exactly the tag and payload fields.
#[test]
fn two_field_variant_constructor_requires_tag_and_payload() {
    for value in [
        "{ other: \"ready\", payload: \"ok\" }",
        "{ tag: \"ready\", other: \"ok\" }",
    ] {
        let body = format!("variant Choice {{ string ready, num failed }}\nChoice item = {value}");
        let error = compile(&body).expect_err("invalid variant field pair must fail");
        assert_eq!(error.code, "NEU-EXP-004", "{value}");
        assert!(error.location.is_some(), "{value}");
    }
    let valid = compile(
        "variant Choice { string ready, num failed }\nChoice item = { tag: \"ready\", payload: \"ok\" }",
    )
    .expect("valid tag/payload pair");
    assert_eq!(
        value(&valid, "item"),
        &V::Variant {
            tag: "ready".into(),
            payload: Box::new(V::String("ok".into())),
        }
    );
}

/// Closed nominal defaults recursively preserve scalar, null and composite values.
#[test]
fn nested_record_defaults_and_origins_preserve_null_and_composite_values() {
    let ir = compile("record Child { num amount = 2 }\nvariant Choice { string ready, num failed }\nrecord Config { num count = 3, string label = \"ok\", bool enabled = true, url endpoint = \"opaque\", path file = \"../x\", num? absent = null, List<num> values = [1, 2], Child child = {}, Choice choice = { tag: \"ready\", payload: \"yes\" } }\nConfig config = {}").unwrap();
    assert_eq!(
        value(&ir, "config"),
        &V::Record(vec![
            ("absent".into(), Some(V::Null)),
            (
                "child".into(),
                Some(V::Record(vec![("amount".into(), Some(number("2")))]))
            ),
            (
                "choice".into(),
                Some(V::Variant {
                    tag: "ready".into(),
                    payload: Box::new(V::String("yes".into()))
                })
            ),
            ("count".into(), Some(number("3"))),
            ("enabled".into(), Some(V::Bool(true))),
            ("endpoint".into(), Some(V::Url("opaque".into()))),
            ("file".into(), Some(V::Path("../x".into()))),
            ("label".into(), Some(V::String("ok".into()))),
            (
                "values".into(),
                Some(V::List(vec![number("1"), number("2")]))
            ),
        ])
    );
    let origins: Vec<_> = ir
        .origins
        .iter()
        .filter(|o| o.binding.declaration_name() == "config")
        .collect();
    assert!(
        origins
            .iter()
            .any(|o| o.path == [ValuePathSegment::Field("absent".into())]
                && o.kind == ValueOriginKind::ExplicitNull)
    );
    assert!(origins.iter().any(|o| o.path
        == [
            ValuePathSegment::Field("choice".into()),
            ValuePathSegment::Payload
        ]
        && o.kind == ValueOriginKind::Supplied));
}

/// Missing, duplicate and unknown constructor members reject instead of silently dropping data.
#[test]
fn nominal_constructors_reject_invalid_fields_and_tags() {
    for (body, code) in [
        ("record Item { num count }\nItem item = {}", "NEU-EXP-004"),
        (
            "record Item { num count }\nItem item = { count: 1, extra: 2 }",
            "NEU-EXP-004",
        ),
        (
            "record Item { num count }\nItem item = { count: 1, count: 2 }",
            "NEU-EXP-002",
        ),
        (
            "record Item { num count }\nItem item = { count 1 }",
            "NEU-EXP-002",
        ),
        (
            "variant Choice { string ready, num failed }\nChoice item = { tag: \"other\", payload: 2 }",
            "NEU-EXP-004",
        ),
        (
            "variant Choice { string ready, num failed }\nChoice item = { tag: \"ready\" }",
            "NEU-EXP-004",
        ),
        (
            "variant Choice { string ready, num failed }\nChoice item = { tag: \"ready\" ++ \"\", payload: \"ok\" }",
            "NEU-EXP-004",
        ),
        ("num item = null", "NEU-EXP-004"),
        ("string item = text([1])", "NEU-EXP-004"),
        ("List<num> item = [1, false]", "NEU-EXP-004"),
    ] {
        assert_eq!(compile(body).unwrap_err().code, code, "{body}");
    }
}

/// Nested interpolation balances braces and strings while decoding literal Unicode and escapes.
#[test]
fn nested_interpolation_and_comments_preserve_literal_result() {
    let ir = compile("/* header */\r\nstring result = $\"é:${$\"inner=${if true { \"yes\" } else { \"}\" }}\"}:\\n\\${literal}\" // trailing\r\nnum quotient = 8 /* operator boundary */ / 2\nstring empty = $\"\"").unwrap();
    assert_eq!(
        value(&ir, "result"),
        &V::String("é:inner=yes:\n${literal}".into())
    );
    assert_eq!(value(&ir, "quotient"), &number("4"));
    assert_eq!(value(&ir, "empty"), &V::String(String::new()));
    for body in ["string item = $\"${2}\"", "string item = $\"${text([1])}\""] {
        assert_eq!(compile(body).unwrap_err().code, "NEU-EXP-004", "{body}");
    }
}

/// Broken delimiter, escape and declaration syntax must fail atomically with safe source codes.
#[test]
fn contextual_parser_rejects_malformed_source() {
    for body in [
        "num item = 1 /* unfinished",
        "string item = \"unfinished",
        "string item = \"\\q\"",
        "string item = $\"${\"unfinished}\"",
        "string item = $\"${text(1)\"",
        "num item = (1",
        "num item = 1)",
        "List<num> item = [,1]",
        "List<num> item = [1,,2]",
        "num item = @",
        "num item = 1e+",
        "num item = 1..2",
        "num item =",
        "num item = if true { 1 }",
        "num item = text(1) extra",
        "num item = (1) extra",
        "num item = unknown::",
        "num item = 1\nimport other as alias",
    ] {
        assert_eq!(compile(body).unwrap_err().code, "NEU-EXP-002", "{body}");
    }
}

/// Match validation checks every tag, binder and body even for statically unselected arms.
#[test]
fn match_rejects_duplicate_unknown_and_mistyped_arms() {
    let prelude = "variant Choice { string ready, num failed }\nChoice choice = { tag: \"ready\", payload: \"ok\" }\n";
    for (body, code) in [
        (
            "string item = match choice { ready(value) => value, ready(other) => other }",
            "NEU-EXP-005",
        ),
        (
            "string item = match choice { ready(value) => value, other(value) => value }",
            "NEU-EXP-005",
        ),
        (
            "string item = match choice { ready(Value) => \"ok\", failed(code) => text(code) }",
            "NEU-EXP-002",
        ),
        (
            "string item = match choice { ready(value) => value, failed(code) => code }",
            "NEU-EXP-004",
        ),
        (
            "string item = match 1 { ready(value) => \"ok\" }",
            "NEU-EXP-002",
        ),
    ] {
        assert_eq!(
            compile(&format!("{prelude}{body}")).unwrap_err().code,
            code,
            "{body}"
        );
    }
}

/// Independently narrowed expression policies reject source work, retention, nodes and numbers.
#[test]
fn source_expression_limits_reject_before_publication() {
    for (body, expression) in [
        (
            "num item = 1",
            ExpressionLimits {
                work: 1,
                ..ExpressionLimits::hard()
            },
        ),
        (
            "string item = \"value\"",
            ExpressionLimits {
                retained_bytes: 1,
                ..ExpressionLimits::hard()
            },
        ),
        (
            "num item = 1 + 2",
            ExpressionLimits {
                nodes: 1,
                ..ExpressionLimits::hard()
            },
        ),
        (
            "num item = ((1))",
            ExpressionLimits {
                syntax_depth: 1,
                ..ExpressionLimits::hard()
            },
        ),
        (
            "num first = 1\nnum second = 2",
            ExpressionLimits {
                bindings: 1,
                ..ExpressionLimits::hard()
            },
        ),
        (
            "num item = 123",
            ExpressionLimits {
                coefficient_digits: 2,
                ..ExpressionLimits::hard()
            },
        ),
        (
            "num item = 1e3",
            ExpressionLimits {
                absolute_scale: 2,
                ..ExpressionLimits::hard()
            },
        ),
    ] {
        assert_eq!(
            bounded(body, expression, ExpansionLimits::hard())
                .unwrap_err()
                .code,
            "NEU-EXP-009",
            "{body}"
        );
    }
}

/// Safe error envelopes identify their selected result schema without retaining supplied names.
#[test]
fn compile_failures_report_selected_schema_and_safe_debug_fields() {
    let failure = compile("num result = secret_missing_symbol").unwrap_err();
    assert_eq!(failure.schema(), "neutral.project-result/3");
    assert_eq!(failure.code, "NEU-EXP-003");
    assert!(failure.location.is_some());
    let diagnostic = format!("{failure:?}");
    assert!(!diagnostic.contains("secret_missing_symbol"));
    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\nnum result = false\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let failure = super::super::super::compile_composition_project(
        &captured.composition,
        &CancellationToken::new(),
    )
    .unwrap_err();
    assert_eq!(failure.schema(), "neutral.project-result/2");
    assert!(failure.location.is_some());
}

/// Declaration syntax errors retain their phase-specific classification before resolution.
#[test]
fn malformed_bindings_templates_and_expansions_reject_at_their_grammar_boundary() {
    for (body, code) in [
        ("= 1", "NEU-EXP-002"),
        ("num Upper = 1", "NEU-EXP-002"),
        ("template Item(static) -> num { 1 }", "NEU-TPL-002"),
        (
            "template Item(static num value) num { value }",
            "NEU-EXP-002",
        ),
        ("template Item() -> num { 1 } trailing", "NEU-TPL-002"),
        ("expand Upper = Item() for (item in [1])", "NEU-TPL-003"),
        ("expand items Item() for (item in [1])", "NEU-EXP-002"),
        ("expand items = 7() for (item in [1])", "NEU-EXP-002"),
        ("expand items = Item() (item in [1])", "NEU-EXP-002"),
        ("expand items = Item() for item", "NEU-EXP-002"),
        (
            "expand items = Item() for (item in [1]) trailing",
            "NEU-TPL-002",
        ),
    ] {
        assert_eq!(compile(body).unwrap_err().code, code, "{body}");
    }
}
