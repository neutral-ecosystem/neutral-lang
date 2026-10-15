// SPDX-License-Identifier: Apache-2.0

//! Template resolution, static phase checks and finite matrix expansion contracts.

use super::*;

/// Captures and compiles an explicit multi-module expression project.
fn compile_sources(sources: &[(&str, &str, &str)]) -> Result<Arc<ExpressionProjectIr>, E> {
    let bounds = ProjectCaptureLimitValues {
        total_source_bytes: 1_048_576,
        source_bytes_per_unit: 1_048_576,
        source_units: 8,
        source_id_bytes: 256,
        module_id_bytes: 256,
        vocabulary_units: 8,
        vocabulary_bytes_per_unit: 1_048_576,
        total_vocabulary_bytes: 1_048_576,
        imports_per_module: 16,
        import_edges: 64,
        scc_units: 8,
        declarations: 4096,
        diagnostics: 64,
        output_bytes: 1_048_576,
    };
    let request = CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            expression_profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            sources
                .iter()
                .map(|(source_id, module_id, body)| {
                    CapturedSourceInput::new(
                        *source_id,
                        *module_id,
                        format!("neu \"1.0\"\nmodule {module_id}\n{body}\n").into_bytes(),
                    )
                })
                .collect(),
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(bounds),
                CancellationToken::new(),
            ),
        ),
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(1_048_576, 64).unwrap(),
        )),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let captured = capture_expression_project(request).unwrap();
    compile_expression_project(&captured, &CancellationToken::new())
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

/// Imported templates keep their defining module's nominal types and source spans.
#[test]
fn imported_public_template_keeps_definition_scope() {
    let ir = compile_sources(&[
        (
            "source:definition",
            "demo::definition",
            "public record Shape { num count }\npublic variant Choice { num ready, num failed }\npublic template Make(static num count) -> Shape { { count: count } }\npublic template MakeMatrix(static num count) -> Choice { { tag: \"ready\", payload: count } }",
        ),
        (
            "source:caller",
            "demo::caller",
            "import demo::definition as definition\nrecord Shape { num count }\ndefinition::Shape one = instantiate definition::Make(count: 1)\nexpand items = definition::MakeMatrix(count: count) for (count in [2])",
        ),
    ])
    .expect("imported public templates should compile");
    let generated: Vec<_> = ir
        .declarations
        .iter()
        .filter(|declaration| {
            ["one", "items__0"].contains(&declaration.identity.declaration_name())
        })
        .collect();
    assert_eq!(generated.len(), 2);
    for declaration in generated {
        assert!(matches!(
            &declaration.signature,
            S::Binding(T::Nominal(owner)) if owner.module().module_name() == "demo::definition"
        ));
    }
    assert_eq!(
        value(&ir, "one"),
        &V::Record(vec![("count".into(), Some(number("1")))])
    );
    assert_eq!(
        value(&ir, "items__0"),
        &V::Variant {
            tag: "ready".into(),
            payload: Box::new(number("2")),
        }
    );
    assert_eq!(ir.expansion_attributions.len(), 2);
    for attribution in &ir.expansion_attributions {
        assert_eq!(attribution.owner.module().module_name(), "demo::caller");
        assert_eq!(
            attribution.application.source(),
            ir.sources
                .iter()
                .find(|source| source.module == "demo::caller")
                .unwrap()
                .digest
        );
        assert_eq!(
            attribution.definition.source(),
            ir.sources
                .iter()
                .find(|source| source.module == "demo::definition")
                .unwrap()
                .digest
        );
        assert!(attribution.application.span().start() < attribution.application.span().end());
        assert!(attribution.definition.span().start() < attribution.definition.span().end());
    }
}

/// Unknown aliases and private imported templates use the same inaccessible-template failure.
#[test]
fn imported_template_lookup_rejects_unknown_alias_and_private_definition() {
    let definition = (
        "source:definition",
        "demo::definition",
        "template Item() -> num { 1 }\npublic template Visible() -> num { 2 }",
    );
    let unknown = compile_sources(&[
        definition,
        (
            "source:caller",
            "demo::caller",
            "num result = instantiate missing::Visible()",
        ),
    ])
    .expect_err("unknown aliases must be rejected");
    assert_eq!(unknown.code, "NEU-TPL-003");
    let private = compile_sources(&[
        definition,
        (
            "source:caller",
            "demo::caller",
            "import demo::definition as definition\nnum result = instantiate definition::Item()",
        ),
    ])
    .expect_err("private imported templates must be rejected");
    assert_eq!(private.code, "NEU-TPL-003");
    let public = compile_sources(&[
        definition,
        (
            "source:caller",
            "demo::caller",
            "import demo::definition as definition\nnum result = instantiate definition::Visible()",
        ),
    ])
    .expect("public imported templates are visible");
    assert_eq!(value(&public, "result"), &number("2"));
}

/// Static checking descends into list, record, variant, interpolation and match arguments.
#[test]
fn nested_static_composite_arguments_keep_static_phase() {
    let ir = compile(
        "public record Shape { num count }\npublic variant Choice { num ready, num failed }\ntemplate EchoList(static List<num> value) -> List<num> { value }\ntemplate EchoRecord(static Shape value) -> Shape { value }\ntemplate EchoVariant(static Choice value) -> Choice { value }\ntemplate EchoText(static string value) -> string { value }\ntemplate EchoNumber(static num value) -> num { value }\npublic template WrapList(static num value) -> List<num> { instantiate EchoList(value: [value]) }\npublic template WrapRecord(static num value) -> Shape { instantiate EchoRecord(value: { count: value }) }\npublic template WrapVariant(static num value) -> Choice { instantiate EchoVariant(value: { tag: \"ready\", payload: value }) }\npublic template WrapText(static num value) -> string { instantiate EchoText(value: $\"${text(value)}\") }\npublic template WrapMatch(static Choice value) -> num { instantiate EchoNumber(value: match value { ready(payload) => 7, failed(payload) => 8 }) }\nList<num> list = instantiate WrapList(value: 1)\nShape record_value = instantiate WrapRecord(value: 2)\nChoice variant = instantiate WrapVariant(value: 3)\nstring text_value = instantiate WrapText(value: 4)\nnum match_value = instantiate WrapMatch(value: { tag: \"ready\", payload: 5 })",
    )
    .expect("nested static composite arguments should resolve");
    assert_eq!(value(&ir, "list"), &V::List(vec![number("1")]));
    assert_eq!(
        value(&ir, "record_value"),
        &V::Record(vec![("count".into(), Some(number("2")))])
    );
    assert_eq!(
        value(&ir, "variant"),
        &V::Variant {
            tag: "ready".into(),
            payload: Box::new(number("3")),
        }
    );
    assert_eq!(value(&ir, "text_value"), &V::String("4".into()));
    assert_eq!(value(&ir, "match_value"), &number("7"));
    assert_eq!(ir.public_templates.len(), 5);
}

/// Deferred leaves cannot hide inside composite arguments to static parameters.
#[test]
fn nested_static_composite_arguments_reject_deferred_leaves() {
    for body in [
        "template Echo(static List<num> value) -> List<num> { value }\ntemplate Outer(deferred num value) -> List<num> { instantiate Echo(value: [value]) }",
        "record Shape { num count }\ntemplate Echo(static Shape value) -> Shape { value }\ntemplate Outer(deferred num value) -> Shape { instantiate Echo(value: { count: value }) }",
        "variant Choice { num ready, num failed }\ntemplate Echo(static Choice value) -> Choice { value }\ntemplate Outer(deferred num value) -> Choice { instantiate Echo(value: { tag: \"ready\", payload: value }) }",
        "template Echo(static string value) -> string { value }\ntemplate Outer(deferred num value) -> string { instantiate Echo(value: $\"${text(value)}\") }",
        "variant Choice { num ready, num failed }\ntemplate Echo(static num value) -> num { value }\ntemplate Outer(static Choice choice) -> num { instantiate Echo(value: match choice { ready(value) => instantiate Echo(value: value), failed(value) => 0 }) }",
    ] {
        assert_eq!(compile(body).unwrap_err().code, "NEU-TPL-004", "{body}");
    }
}

/// Matrix exclusion tuple members must be static even when no tuple survives selection.
#[test]
fn matrix_exclusions_reject_deferred_tuple_members() {
    for body in [
        "input num denied\ntemplate Item(static num value) -> num { value }\nexpand items = Item(value: axis) for (axis in [1]) exclude [{ axis: denied }]",
        "input num denied\ntemplate Item(static num value) -> num { value }\nList<num> axes = []\nexpand items = Item(value: axis) for (axis in axes) exclude [{ axis: denied }]",
        "input num denied\ntemplate Item(static num value) -> num { value }\nexpand items = Item(value: axis) for (axis in [1]) where false exclude [{ axis: denied }]",
    ] {
        assert_eq!(compile(body).unwrap_err().code, "NEU-TPL-007", "{body}");
    }
    let ir = compile(
        "template Item(static num value) -> num { value }\nexpand items = Item(value: axis) for (axis in [1]) exclude [{ axis: 1 }]",
    )
    .expect("static exclusions should filter tuples");
    assert_eq!(
        ir.declarations,
        [] as [neutral_ir::expression::project::ExpressionDeclaration; 0]
    );
}
