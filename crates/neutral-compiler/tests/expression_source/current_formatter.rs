// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::format_expression_source;

#[test]
fn property_current_expression_source_formatting_is_idempotent_and_recompilable() {
    let source = r#"neu "1.0"
module example::expressions
/* retain this comment: café */
public input bool enabled
public num selected=if enabled{1}else{2}
template Greeting(static string name)->string{$"hello ${name}"}
template Item(static num value)->num{value}
public expand items=Item(value:item) for(item in [1,2])
"#;
    let captured = capture(source, ExpressionLimits::hard(), ExpansionLimits::hard());
    let formatted =
        format_expression_source(&captured, "example::expressions", &CancellationToken::new())
            .unwrap();
    assert!(
        std::str::from_utf8(&formatted)
            .unwrap()
            .contains("/* retain this comment: café */")
    );
    let recaptured = capture(
        std::str::from_utf8(&formatted).unwrap(),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let original_ir = compile_expression_project(&captured, &CancellationToken::new()).unwrap();
    let formatted_ir = compile_expression_project(&recaptured, &CancellationToken::new()).unwrap();
    assert_eq!(original_ir.declarations, formatted_ir.declarations);
    assert_eq!(original_ir.interface_digest, formatted_ir.interface_digest);
    assert_eq!(
        format_expression_source(
            &recaptured,
            "example::expressions",
            &CancellationToken::new()
        )
        .unwrap(),
        formatted
    );
    compile_expression_project(&recaptured, &CancellationToken::new()).unwrap();
}

#[test]
fn braces_and_lists_use_indented_multiline_layout_and_output_limit() {
    let source = "neu \"1.0\"\nmodule example::expressions\npublic record R{num a,string b}\nList<num> values=[1,2]\n";
    let captured = capture(source, ExpressionLimits::hard(), ExpansionLimits::hard());
    let formatted =
        format_expression_source(&captured, "example::expressions", &CancellationToken::new())
            .unwrap();
    let formatted = std::str::from_utf8(&formatted).unwrap();
    assert!(formatted.contains("R {\n    num a,\n    string b\n}"));
    assert!(formatted.contains("[\n    1,\n    2\n]"));

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
        output_bytes: 8,
    };
    let constrained = capture_expression_project(CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            expression_profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "source:fixture",
                "example::expressions",
                source.as_bytes().to_vec(),
            )],
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
    ))
    .unwrap();
    assert_eq!(
        format_expression_source(
            &constrained,
            "example::expressions",
            &CancellationToken::new()
        )
        .unwrap_err()
        .code,
        "NEU-EXP-009"
    );
}

#[test]
fn current_output_and_matrix_forms_are_accepted() {
    for source in [
        include_str!("fixtures/positive/output.neu"),
        include_str!("fixtures/positive/matrix.neu"),
    ] {
        let captured = capture(source, ExpressionLimits::hard(), ExpansionLimits::hard());
        let result =
            format_expression_source(&captured, "example::expressions", &CancellationToken::new())
                .unwrap();
        let captured = capture(
            std::str::from_utf8(&result).unwrap(),
            ExpressionLimits::hard(),
            ExpansionLimits::hard(),
        );
        compile_expression_project(&captured, &CancellationToken::new()).unwrap();
    }
}

#[test]
fn previous_source_header_is_rejected_during_capture() {
    assert!(
        capture_expression_project(request(
            "neu \"0.9\"\nmodule example::expressions\nnum result = 1\n",
            ExpressionLimits::hard(),
            ExpansionLimits::hard(),
        ))
        .is_err()
    );
}

#[test]
fn cancellation_and_allocation_failures_do_not_poison_retry() {
    use neutral_core::allocation::testing::observe;

    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\nnum answer = 42\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        format_expression_source(&captured, "example::expressions", &cancel)
            .unwrap_err()
            .code,
        "NEU-EXP-010"
    );

    let (failed, checkpoints) = observe(Some(0), || {
        format_expression_source(&captured, "example::expressions", &CancellationToken::new())
    });
    assert!(checkpoints > 0);
    assert!(failed.is_err());
    assert!(
        format_expression_source(&captured, "example::expressions", &CancellationToken::new())
            .is_ok()
    );
}
