// SPDX-License-Identifier: Apache-2.0

//! Observable source contracts for parsing, typing, finite expansion and lowering.

use super::*;
use neutral_ir::composition::{ValueOriginKind, ValuePathSegment};

/// Compiles real captured source with one deliberately narrowed policy dimension.
fn bounded(
    body: &str,
    expression: ExpressionLimits,
    expansion: ExpansionLimits,
) -> Result<Arc<ExpressionProjectIr>, E> {
    let source = format!(
        "neu \"{}\"\nmodule example::expressions\n{body}\n",
        LanguageProfile::V1_0.source_version()
    );
    compile_expression_project(
        &capture(&source, expression, expansion),
        &CancellationToken::new(),
    )
}

/// Checks hand-derived decimal values without relying on the evaluator for expectations.
fn number(literal: &str) -> V {
    V::Number(
        ExactNumber::from_source(
            literal,
            ExpressionLimits::hard().coefficient_digits,
            ExpressionLimits::hard().absolute_scale,
        )
        .unwrap(),
    )
}

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

/// Exported templates retain ordered defaults and definition contracts while applications fold.
#[test]
fn exported_template_defaults_use_preceding_parameters() {
    let ir = compile("public template Scale(static num base, static num factor = base + 1, deferred num result = factor * 2) -> num { result }\nnum answer = instantiate Scale(base: 3)").unwrap();
    assert_eq!(value(&ir, "answer"), &number("8"));
    assert_eq!(ir.public_templates.len(), 1);
    let template = &ir.public_templates[0];
    assert_eq!(template.identity.declaration_name(), "Scale");
    assert_eq!(
        template
            .parameters
            .iter()
            .map(|p| (p.name.as_str(), p.phase, p.default.is_some()))
            .collect::<Vec<_>>(),
        [
            ("base", ParameterPhase::Static, false),
            ("factor", ParameterPhase::Static, true),
            ("result", ParameterPhase::Deferred, true)
        ]
    );
    assert!(matches!(template.body.node, N::Bound { depth: 0, slot: 2 }));
    assert_eq!(ir.expansion_attributions.len(), 1);
    assert_eq!(ir.expansion_attributions[0].arguments.len(), 3);
}

/// Static nested applications may use static lexical parameters and reject deferred ones.
#[test]
fn nested_template_arguments_keep_static_phase_and_hygienic_match_binders() {
    let ir = compile("public variant Choice { num ready, string failed }\ntemplate Inner(static num value) -> num { value + 1 }\npublic template Outer(static num value) -> num { instantiate Inner(value: -value + 4) }\ntemplate Match(deferred Choice choice, deferred num offset) -> num { match choice { ready(offset) => offset, failed(message) => offset } }\nnum nested = instantiate Outer(value: 2)\nnum matched = instantiate Match(choice: { tag: \"ready\", payload: 7 }, offset: 100)").unwrap();
    assert_eq!(value(&ir, "nested"), &number("3"));
    assert_eq!(value(&ir, "matched"), &number("7"));
    assert_eq!(compile("template Inner(static num value) -> num { value }\ntemplate Outer(deferred num value) -> num { instantiate Inner(value: value) }").unwrap_err().code, "NEU-TPL-004");
    assert_eq!(compile("input num external\ntemplate Inner(static num value) -> num { 0 }\nnum result = instantiate Inner(value: external)").unwrap_err().code, "NEU-TPL-004");
}

/// Name and phase errors reject unused definitions and complete named applications.
#[test]
fn template_names_defaults_and_arguments_reject_invalid_contracts() {
    for (body, code) in [
        (
            "template lower(static num value) -> num { value }",
            "NEU-TPL-003",
        ),
        ("template Item(num value) -> num { value }", "NEU-TPL-002"),
        (
            "template Item(static num value, static num value) -> num { value }",
            "NEU-TPL-003",
        ),
        (
            "template Item(static num first = 1, static num second) -> num { first }",
            "NEU-TPL-003",
        ),
        (
            "template Item(static num first = second, static num second = 1) -> num { first }",
            "NEU-EXP-003",
        ),
        (
            "template Item(static num value) -> num { value }\nnum item = instantiate Item()",
            "NEU-TPL-003",
        ),
        (
            "template Item(static num value) -> num { value }\nnum item = instantiate Item(other: 1)",
            "NEU-TPL-003",
        ),
        (
            "template Item(static num value) -> num { value }\nnum item = instantiate Item(value: 1, value: 2)",
            "NEU-TPL-003",
        ),
        (
            "template Item(static num value) -> num { value }\nstring item = instantiate Item(value: 1)",
            "NEU-TPL-004",
        ),
        (
            "num hidden = 2\ntemplate Item() -> num { hidden }",
            "NEU-TPL-003",
        ),
        (
            "num hidden = 2\ntemplate Item() -> Ref<num> { ref(hidden) }",
            "NEU-TPL-003",
        ),
        ("num item = instantiate Missing()", "NEU-TPL-003"),
    ] {
        assert_eq!(compile(body).unwrap_err().code, code, "{body}");
    }
}

/// Matrix tuple values and source indices survive filtering and exact exclusions.
#[test]
fn matrix_values_follow_original_tuple_order_after_filter_and_exclusion() {
    let ir = compile("template Sum(static num left, static num right) -> num { left + right }\npublic expand sums = Sum(left: left, right: right)\nfor (left in [1, 2], right in [10, 20])\nwhere left == 2 || right == 10\nexclude [{ left: 2, right: 10 }]").unwrap();
    assert_eq!(
        ir.declarations
            .iter()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["sums__0__0", "sums__1__1"]
    );
    assert_eq!(value(&ir, "sums__0__0"), &number("11"));
    assert_eq!(value(&ir, "sums__1__1"), &number("22"));
    assert!(ir.declarations.iter().all(|d| d.public));
    assert_eq!(
        ir.expansion_attributions
            .iter()
            .map(|a| a.original_axis_indices.as_slice())
            .collect::<Vec<_>>(),
        [&[0, 0][..], &[1, 1][..]]
    );
}

/// Empty and filtered matrices still validate shape, phase and exclusion membership.
#[test]
fn matrix_shape_errors_cannot_hide_behind_empty_or_filtered_axes() {
    let prelude = "template Item(static num value) -> num { value }\n";
    for (body, code) in [
        (
            "expand items = Item(value: axis) for (axis in 1)",
            "NEU-TPL-007",
        ),
        ("expand items = Item(value: axis) for ()", "NEU-TPL-007"),
        (
            "expand items = Item(value: axis) for (axis in [1], axis in [2])",
            "NEU-TPL-007",
        ),
        (
            "expand items = Item(value: axis) for (axis in [1]) where 1",
            "NEU-EXP-004",
        ),
        (
            "expand items = Item(value: axis) for (axis in [1]) exclude [{ axis: 2 }]",
            "NEU-TPL-007",
        ),
        (
            "expand items = Item(value: axis) for (axis in [1]) exclude [{ other: 1 }]",
            "NEU-TPL-007",
        ),
        (
            "expand items = Item(value: axis) for (axis in [1]) exclude [{ axis: 1 }, { axis: 1 }]",
            "NEU-TPL-007",
        ),
        (
            "expand items = Item(value: axis) for (axis in [1]) exclude [{ axis: 1, extra: 1 }]",
            "NEU-TPL-007",
        ),
        (
            "input List<num> values\nexpand items = Item(value: axis) for (axis in values)",
            "NEU-TPL-007",
        ),
        (
            "input bool enabled\nexpand items = Item(value: axis) for (axis in [1]) where enabled",
            "NEU-TPL-007",
        ),
        (
            "List<num> empty = []\nexpand items = Item(value: missing) for (axis in empty)",
            "NEU-EXP-003",
        ),
        (
            "expand items = Item(value: missing) for (axis in [1]) where false",
            "NEU-EXP-003",
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

/// Expansion ceilings are aggregate controls, including definition typing and filtered tuples.
#[test]
fn independent_expansion_limits_reject_complete_request() {
    let template = "template Item(static num value) -> num { value }\n";
    for (body, expansion) in [
        (
            "template Other() -> num { 0 }",
            ExpansionLimits {
                definitions: 1,
                ..ExpansionLimits::hard()
            },
        ),
        (
            "template Other(static num first, static num second) -> num { first }",
            ExpansionLimits {
                parameters: 1,
                ..ExpansionLimits::hard()
            },
        ),
        (
            "num first = instantiate Item(value: 1)\nnum second = instantiate Item(value: 2)",
            ExpansionLimits {
                applications: 1,
                ..ExpansionLimits::hard()
            },
        ),
        (
            "expand items = Item(value: axis) for (axis in [1, 2])",
            ExpansionLimits {
                axis_items: 1,
                ..ExpansionLimits::hard()
            },
        ),
        (
            "expand first = Item(value: axis) for (axis in [1, 2]) where false\nexpand second = Item(value: axis) for (axis in [1]) where false",
            ExpansionLimits {
                aggregate_tuples: 2,
                ..ExpansionLimits::hard()
            },
        ),
        (
            "expand items = Item(value: axis) for (axis in [1, 2])",
            ExpansionLimits {
                expanded_bindings: 1,
                ..ExpansionLimits::hard()
            },
        ),
        (
            "num item = instantiate Item(value: 1 + 2)",
            ExpansionLimits {
                expanded_nodes: 1,
                ..ExpansionLimits::hard()
            },
        ),
        (
            "expand items = Item(value: axis) for (axis in [1, 2]) where false",
            ExpansionLimits {
                work: 1,
                ..ExpansionLimits::hard()
            },
        ),
    ] {
        assert_eq!(
            bounded(
                &format!("{template}{body}"),
                ExpressionLimits::hard(),
                expansion
            )
            .unwrap_err()
            .code,
            "NEU-TPL-008",
            "{body}"
        );
    }
}

/// Output paths preserve all three segment contracts as deferred consumer data.
#[test]
fn output_keeps_field_index_and_alternative_contracts() {
    let ir = compile("public record Job { string name }\npublic variant Choice { string ready, num failed }\npublic record Result { List<Choice> choices }\npublic Job job = { name: \"build\" }\npublic num result = output<num>(ref(job), Result, [{ kind: \"field\", name: \"choices\" }, { kind: \"index\", index: 1 }, { kind: \"alternative\", name: \"failed\" }])").unwrap();
    let result = ir
        .declarations
        .iter()
        .find(|d| d.identity.declaration_name() == "result")
        .unwrap();
    let Some(ExpressionBody::Expression(Expression {
        node:
            N::Output {
                target,
                result_type,
                selector,
            },
        ..
    })) = &result.body
    else {
        panic!("future output must remain deferred")
    };
    assert_eq!(target.declaration_name(), "job");
    assert!(matches!(result_type, T::Nominal(owner) if owner.declaration_name() == "Result"));
    assert_eq!(
        selector,
        &[
            SelectorSegment::Field("choices".into()),
            SelectorSegment::Index(1),
            SelectorSegment::Alternative("failed".into())
        ]
    );
    assert_eq!(ir.expression_resources.output_leaves, 1);
    assert!(
        ir.provenance
            .iter()
            .any(|edge| edge.from.declaration_name() == "result"
                && edge.to.declaration_name() == "Result"
                && edge.kind == Edge::Type)
    );
}

/// Malformed output selectors reject before consumer obligations can be published.
#[test]
fn output_rejects_malformed_and_incompatible_selector_contracts() {
    let prelude = "public record Job { string name }\npublic record Result { num count }\npublic Job job = { name: \"build\" }\n";
    for (expression, code) in [
        ("output<num>(ref(job), Result)", "NEU-EXP-002"),
        ("output<num>(ref(job), num, [])", "NEU-EXP-004"),
        (
            "output<num>(ref(job), Result, [{ kind: \"field\" }])",
            "NEU-EXP-002",
        ),
        (
            "output<num>(ref(job), Result, [{ kind: \"field\", kind: \"field\" }])",
            "NEU-EXP-002",
        ),
        (
            "output<num>(ref(job), Result, [{ kind: \"field\", unknown: \"count\" }])",
            "NEU-EXP-002",
        ),
        (
            "output<num>(ref(job), Result, [{ kind: \"field\", index: 0 }])",
            "NEU-EXP-008",
        ),
        (
            "output<num>(ref(job), Result, [{ kind: \"index\", name: \"count\" }])",
            "NEU-EXP-008",
        ),
        (
            "output<num>(ref(job), Result, [{ kind: \"index\", index: 1.5 }])",
            "NEU-EXP-008",
        ),
        (
            "output<num>(ref(job), Result, [{ kind: \"index\", index: 4294967296 }])",
            "NEU-EXP-008",
        ),
        (
            "output<num>(ref(job), Result, [{ kind: \"unknown\", name: \"count\" }])",
            "NEU-EXP-008",
        ),
        (
            "output<num>(ref(job), Result, [{ kind: \"field\", name: \"missing\" }])",
            "NEU-EXP-008",
        ),
        (
            "output<string>(ref(job), Result, [{ kind: \"field\", name: \"count\" }])",
            "NEU-EXP-004",
        ),
    ] {
        assert_eq!(
            compile(&format!("{prelude}num result = {expression}"))
                .unwrap_err()
                .code,
            code,
            "{expression}"
        );
    }
    assert_eq!(
        compile("public num target = 1\nnum result = output<num>(ref(target), num, [])")
            .unwrap_err()
            .code,
        "NEU-EXP-003"
    );
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
