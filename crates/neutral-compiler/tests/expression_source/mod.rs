// SPDX-License-Identifier: Apache-2.0

//! Literal successor source results; compilation alone does not activate global profiles.

use super::*;
use crate::{
    CapturedExpressionProjectRequest, CapturedProjectRequest, CapturedSourceInput,
    CapturedVocabularyInput, CompositionCaptureFailure, ExpressionCaptureFailure,
    ProjectCaptureControls, ProjectCaptureError, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_expression_project,
};
use neutral_core::{StructuralLimits, profile::LanguageProfile};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

/// Source-level regression cases for complete expression and expansion boundaries.
mod boundaries;
/// Exact captured expression request facts and rejection boundaries.
mod capture;
/// Template resolution, static checking and finite matrix expansion.
mod expansion;
/// Request-local allocation and cancellation checks across complete source lowering.
mod fault_sweeps;

fn capture(
    bytes: &str,
    expression: ExpressionLimits,
    expansion: ExpansionLimits,
) -> CapturedExpressionProject {
    capture_expression_project(request(bytes, expression, expansion)).unwrap()
}
fn request(
    bytes: &str,
    expression: ExpressionLimits,
    expansion: ExpansionLimits,
) -> CapturedExpressionProjectRequest {
    request_with_inputs(
        vec![CapturedSourceInput::new(
            "source:fixture",
            "example::expressions",
            bytes.as_bytes().to_vec(),
        )],
        Vec::new(),
        expression_profile::CAPTURE_REQUEST_VERSION,
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(1_048_576, 64).unwrap(),
        )),
        (expression, expansion),
        CancellationToken::new(),
    )
}
fn request_with_inputs(
    sources: Vec<CapturedSourceInput>,
    vocabularies: Vec<CapturedVocabularyInput>,
    version: &str,
    profiles: expression_profile::SemanticProfiles,
    composition: CompositionLimits,
    (expression, expansion): (ExpressionLimits, ExpansionLimits),
    cancellation: CancellationToken,
) -> CapturedExpressionProjectRequest {
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
    CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            version,
            LanguageProfile::V1_0,
            sources,
            vocabularies,
            ProjectCaptureControls::new(ProjectCaptureLimits::new(bounds), cancellation),
        ),
        profiles,
        composition,
        expression,
        expansion,
    )
}
fn compile(body: &str) -> Result<Arc<ExpressionProjectIr>, E> {
    let source = format!("neu \"1.0\"\nmodule example::expressions\n{body}\n");
    compile_expression_project(
        &capture(&source, ExpressionLimits::hard(), ExpansionLimits::hard()),
        &CancellationToken::new(),
    )
}
fn value<'a>(ir: &'a ExpressionProjectIr, name: &str) -> &'a V {
    let declaration = ir
        .declarations
        .iter()
        .find(|d| d.identity.declaration_name() == name)
        .unwrap();
    let Some(ExpressionBody::Value(value)) = &declaration.body else {
        panic!("{name} is not materialized");
    };
    value
}

#[test]
fn frozen_arithmetic_precedence_and_decimal_results() {
    let captured = capture(
        include_str!("fixtures/positive/arithmetic.neu"),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let ir = compile_expression_project(&captured, &CancellationToken::new()).unwrap();
    for (name, literal) in [
        ("priority", "14"),
        ("grouped", "20"),
        ("exact", "0.125"),
        ("third", "0.3333333333333333333333333333333333"),
        ("signed", "-0.1666666666666666666666666666666667"),
        ("difference", "6"),
    ] {
        assert_eq!(
            value(&ir, name),
            &V::Number(ExactNumber::from_source(literal, 4096, 4096).unwrap()),
            "{name}"
        );
    }
    for name in ["compare", "inverted", "equal", "less_equal", "greater"] {
        assert_eq!(value(&ir, name), &V::Bool(true));
    }
}
#[test]
fn every_branch_types_before_selected_arithmetic() {
    assert_eq!(
        compile("num result = if true { 1 } else { missing }")
            .unwrap_err()
            .code,
        "NEU-EXP-003"
    );
    assert_eq!(
        compile("num result = if true { 1 } else { false }")
            .unwrap_err()
            .code,
        "NEU-EXP-004"
    );
    let ir =
        compile("num result = if true { 7 } else { 1 / 0 }\nbool guarded = false && (1 / 0 > 0)")
            .unwrap();
    assert_eq!(
        value(&ir, "result"),
        &V::Number(ExactNumber::from_source("7", 4096, 4096).unwrap())
    );
    assert_eq!(value(&ir, "guarded"), &V::Bool(false));
    assert_eq!(
        compile("num result = 1 / 0").unwrap_err().code,
        "NEU-EXP-006"
    );
}
#[test]
fn input_is_never_a_placeholder_and_unresolved_guard_preserves_failure() {
    let ir =
        compile("public input bool enabled\npublic num result = if enabled { 1 / 0 } else { 0 }")
            .unwrap();
    assert!(matches!(
        ir.declarations[0].body,
        Some(ExpressionBody::Input)
    ));
    let Some(ExpressionBody::Expression(expression)) = &ir.declarations[1].body else {
        panic!("residual guard missing");
    };
    assert!(matches!(expression.node, N::If { .. }));
    assert_eq!(ir.expression_resources.input_declarations, 1);
}
#[test]
fn construction_match_and_interpolation_literals() {
    let ir=compile("variant Outcome { string ready, num failed }\nOutcome outcome = { tag: \"ready\", payload: \"ok\" }\nstring result = match outcome { ready(message) => message, failed(code) => text(code) }\nstring nested = $\"outer=${\"inner\" ++ text(2 + 3)}\"\nstring escaped = $\"literal=\\${ok}\"\nList<num> values = [1 + 2] ++ [4 / 2, -3]").unwrap();
    assert_eq!(value(&ir, "result"), &V::String("ok".into()));
    assert_eq!(value(&ir, "nested"), &V::String("outer=inner5".into()));
    assert_eq!(value(&ir, "escaped"), &V::String("literal=${ok}".into()));
    assert!(matches!(value(&ir,"values"),V::List(items) if items.len()==3));
    assert_eq!(compile("variant Outcome { string ready, num failed }\nOutcome outcome = { tag: \"ready\", payload: \"ok\" }\nstring result = match outcome { ready(message) => message }").unwrap_err().code,"NEU-EXP-005");
}
#[test]
fn output_retains_exact_typed_contract_selector_without_io() {
    let captured = capture(
        include_str!("fixtures/positive/output.neu"),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let ir = compile_expression_project(&captured, &CancellationToken::new()).unwrap();
    assert_eq!(ir.expression_resources.output_leaves, 1);
    assert_eq!(ir.expression_resources.input_declarations, 1);
    assert!(ir.provenance.iter().all(|edge| edge.kind != Edge::Value));
}
#[test]
fn templates_evaluate_unused_static_arguments_and_guard_deferred_arguments() {
    let ir=compile("template Select(static bool enabled, deferred num value) -> num { if enabled { value } else { 0 } }\npublic num result = instantiate Select(enabled: false, value: 1 / 0)").unwrap();
    assert_eq!(
        value(&ir, "result"),
        &V::Number(ExactNumber::from_source("0", 4096, 4096).unwrap())
    );
    assert_eq!(compile("template Select(static num value) -> num { 0 }\nnum result = instantiate Select(value: 1 / 0)").unwrap_err().code,"NEU-EXP-006");
    assert_eq!(compile("template Item(static num value = 3) -> num { value }\nnum result = instantiate Item(value: null)").unwrap_err().code,"NEU-TPL-004");
    assert_eq!(compile("template Recur(static num value) -> num { if true { value } else { instantiate Recur(value: value) } }\nnum result = instantiate Recur(value: 1)").unwrap_err().code,"NEU-TPL-005");
}
#[test]
fn matrix_original_indices_duplicates_empty_axes_and_collision() {
    let captured = capture(
        include_str!("fixtures/positive/matrix.neu"),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let ir = compile_expression_project(&captured, &CancellationToken::new()).unwrap();
    let names: Vec<_> = ir
        .declarations
        .iter()
        .map(|d| d.identity.declaration_name())
        .collect();
    assert_eq!(names, ["labels__0__0", "labels__0__1", "labels__1__1"]);
    assert_eq!(
        ir.expansion_attributions
            .iter()
            .map(|a| a.original_axis_indices.as_slice())
            .collect::<Vec<_>>(),
        [&[0, 0][..], &[0, 1][..], &[1, 1][..]]
    );
    let duplicate=compile("template Item(static num value) -> num { value }\nexpand items = Item(value: item) for (item in [1, 1])").unwrap();
    assert_eq!(duplicate.declarations.len(), 2);
    let empty = capture(
        include_str!("fixtures/positive/matrix-empty.neu"),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    assert_eq!(
        compile_expression_project(&empty, &CancellationToken::new())
            .unwrap()
            .declarations
            .len(),
        1
    );
    assert_eq!(compile("template Item(static num value) -> num { value }\nnum items__0 = 0\nexpand items = Item(value: item) for (item in [1])").unwrap_err().code,"NEU-TPL-006");
}
#[test]
fn filtered_product_preflight_cannot_hide_over_limit() {
    let bytes = include_str!("fixtures/boundary/product.neu");
    let below = capture(
        bytes,
        ExpressionLimits::hard(),
        ExpansionLimits {
            product_tuples: 3,
            ..ExpansionLimits::hard()
        },
    );
    assert_eq!(
        compile_expression_project(&below, &CancellationToken::new())
            .unwrap_err()
            .code,
        "NEU-TPL-008"
    );
    let exact = capture(
        bytes,
        ExpressionLimits::hard(),
        ExpansionLimits {
            product_tuples: 4,
            ..ExpansionLimits::hard()
        },
    );
    assert_eq!(
        compile_expression_project(&exact, &CancellationToken::new())
            .unwrap()
            .declarations,
        [] as [ExpressionDeclaration; 0]
    );
}
#[test]
fn ordinary_reuse_cycles_imperative_and_chained_comparisons_reject() {
    assert_eq!(
        compile("num left = right + 1\nnum right = left + 1")
            .unwrap_err()
            .code,
        "NEU-EXP-007"
    );
    assert_eq!(
        compile("bool result = 1 < 2 < 3").unwrap_err().code,
        "NEU-EXP-002"
    );
    assert!(compile("num count = 0\nwhile true { count = count + 1 }").is_err());
}
#[test]
fn exact_capture_tuple_and_cancellation_precede_body_publication() {
    let mut wrong = expression_profile::SemanticProfiles::try_frozen().unwrap();
    wrong.required_features.reverse();
    let bounds = request(
        "neu \"1.0\"\nmodule example::expressions\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    // A fresh explicitly malformed request can never be accepted by sorted-feature repair.
    let CapturedExpressionProjectRequest { .. } = bounds;
    assert!(!wrong.matches_frozen());
    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\nnum result = 1\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        compile_expression_project(&captured, &cancel)
            .unwrap_err()
            .code,
        "NEU-EXP-010"
    );
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

/// Copying a materialized body preserves allocation errors and leaves the body reusable.
#[test]
fn copied_materialized_body_rejects_allocation_failure_without_consuming_value() {
    use neutral_core::allocation::testing::observe;

    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let cancel = CancellationToken::new();
    let mut state = initial_state(&captured, &cancel).unwrap();
    let value = V::String("retained".into());
    let body = ExpressionBody::Value(value.clone());
    let owner = symbol("example::expressions", "retained").unwrap();
    state
        .bodies
        .insert(owner.clone(), ExpressionBody::Value(value))
        .unwrap();
    let (failed, checkpoints) = observe(Some(0), || state.binding(&owner));
    assert!(checkpoints > 0);
    assert_eq!(failed.unwrap_err(), error(ec::EXP_LIMIT, None));
    assert_eq!(state.binding(&owner).unwrap(), body);
}

/// Checked arithmetic keeps overflow failures lazy and preserves counter update order.
#[test]
fn budget_overflow_preserves_error_codes_and_preceding_charges() {
    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let cancel = CancellationToken::new();
    let mut state = initial_state(&captured, &cancel).unwrap();
    let budget = &mut state.budget;
    budget.work = u64::MAX;
    assert_eq!(budget.charge(1).unwrap_err(), error(ec::EXP_LIMIT, None));
    assert_eq!(budget.work, u64::MAX);
    budget.work = 0;
    assert_eq!(
        budget.bytes(u64::MAX).unwrap_err(),
        error(ec::EXP_LIMIT, None)
    );
    assert_eq!(budget.work, 0);
    budget.bytes = u64::MAX;
    assert_eq!(budget.bytes(1).unwrap_err(), error(ec::EXP_LIMIT, None));
    assert_eq!(budget.work, 2);
    assert_eq!(budget.bytes, u64::MAX);
    budget.nodes = u64::MAX;
    assert_eq!(budget.node(1).unwrap_err(), error(ec::EXP_LIMIT, None));
    assert_eq!(budget.work, 3);
    budget.expansion_work = u64::MAX;
    assert_eq!(
        budget.expansion(1, 0).unwrap_err(),
        error(ec::TPL_LIMIT, None)
    );
    assert_eq!(budget.work, 4);
    budget.expansion_work = 0;
    budget.expansion_bytes = u64::MAX;
    assert_eq!(
        budget.expansion(1, 1).unwrap_err(),
        error(ec::TPL_LIMIT, None)
    );
    assert_eq!(budget.work, 5);
    assert_eq!(budget.expansion_work, 1);
    assert_eq!(budget.expansion_bytes, u64::MAX);
    assert_eq!(
        budget.retain::<u64>(usize::MAX).unwrap_err(),
        error(ec::EXP_LIMIT, None)
    );
}
