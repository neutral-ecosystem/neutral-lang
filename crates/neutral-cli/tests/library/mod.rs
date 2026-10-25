// SPDX-License-Identifier: Apache-2.0

//! Crate-local tests for the complete in-process CLI host adapter.

use super::{constants, execute};
use neutral_core::CancellationToken;
use neutral_encoding::{DecodeLimits, expression::decode_expression_project};
use neutral_ir::ExactNumber;
use neutral_test_support::TestDirectory;
use std::fs;

/// Valid source used across in-process host commands.
const SOURCE: &[u8] = b"neu \"1.0\"\nmodule library_test\nnum answer = 42\n";

/// Current CLI artifacts remain independently checked under caller-owned limits.
fn read_current(bytes: &[u8]) -> neutral_reader::ValidatedExpressionProject {
    let scalar = neutral_core::StructuralLimits::new(1_048_576, 64).unwrap();
    decode_expression_project(
        bytes,
        DecodeLimits::hard(),
        neutral_reader::SemanticLimits {
            project: neutral_encoding::hard_project_limits(),
            composition: neutral_reader::composition::ProjectCompositionLimits::from_vocabulary(
                neutral_reader::composition::CompositionScalarLimits::from_structural(scalar),
            ),
            expression: neutral_ir::expression::profile::ExpressionLimits::hard(),
            expansion: neutral_ir::expression::templates::ExpansionLimits::hard(),
        },
        &CancellationToken::new(),
    )
    .unwrap()
}

/// Every CLI command accepts current finite expansion without implicitly evaluating deferred data.
#[test]
fn latest_only_cli_preserves_deferred_templates_through_format_and_wire() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("expansion.neu");
    let formatted = fixture.path().join("formatted.neu");
    let artifact = fixture.path().join("expansion.nir");
    fs::write(&source, include_bytes!("../fixtures/expansion.neu")).unwrap();
    execute([
        constants::VALIDATE.to_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert_eq!(fs::read_dir(fixture.path()).unwrap().count(), 1);
    execute([
        constants::FORMAT.to_owned(),
        source.to_string_lossy().into_owned(),
        constants::OUTPUT.to_owned(),
        formatted.to_string_lossy().into_owned(),
    ])
    .unwrap();
    execute([
        constants::COMPILE.to_owned(),
        formatted.to_string_lossy().into_owned(),
        constants::OUTPUT.to_owned(),
        artifact.to_string_lossy().into_owned(),
    ])
    .unwrap();
    let decoded = read_current(&fs::read(artifact).unwrap());
    assert_eq!(
        decoded
            .ir()
            .declarations
            .iter()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["seed", "values__0", "values__1"]
    );
    assert!(matches!(
        decoded.ir().declarations[0].body,
        Some(neutral_ir::expression::ExpressionBody::Input)
    ));
    for value in &decoded.ir().declarations[1..] {
        assert!(matches!(
            value.body,
            Some(neutral_ir::expression::ExpressionBody::Expression(_))
        ));
    }
}
/// The executable must produce the current complete expression artifact, not an old wrapper.
#[test]
fn latest_only_cli_compiles_current_arithmetic() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("current.neu");
    let artifact = fixture.path().join("current.nir");
    fs::write(
        &source,
        b"neu \"1.0\"\nmodule current\npublic num answer = 6 * 7\n",
    )
    .unwrap();
    execute([
        constants::COMPILE.to_owned(),
        source.to_string_lossy().into_owned(),
        constants::OUTPUT.to_owned(),
        artifact.to_string_lossy().into_owned(),
    ])
    .unwrap();
    let bytes = fs::read(&artifact).unwrap();
    assert!(bytes.starts_with(b"NEUP3\r\n\x1a"));
    let scalar = neutral_core::StructuralLimits::new(1_048_576, 64).unwrap();
    let limits = neutral_reader::SemanticLimits {
        project: neutral_encoding::hard_project_limits(),
        composition: neutral_reader::composition::ProjectCompositionLimits::from_vocabulary(
            neutral_reader::composition::CompositionScalarLimits::from_structural(scalar),
        ),
        expression: neutral_ir::expression::profile::ExpressionLimits::hard(),
        expansion: neutral_ir::expression::templates::ExpansionLimits::hard(),
    };
    let project = neutral_encoding::decode_expression_project(
        &bytes,
        DecodeLimits::hard(),
        limits,
        &CancellationToken::new(),
    )
    .unwrap();
    assert!(
        matches!(&project.ir().declarations[0].body, Some(neutral_ir::expression::ExpressionBody::Value(neutral_ir::composition::BindingValue::Number(value))) if value == &ExactNumber::from_source("42", 64, 64).unwrap())
    );
}

/// An obsolete source profile must fail before an existing destination is replaced.
#[test]
fn latest_only_cli_rejects_old_source_without_replacing_output() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("old.neu");
    let artifact = fixture.path().join("existing.nir");
    fs::write(&source, SOURCE).unwrap();
    fs::write(&artifact, b"keep existing output").unwrap();
    fs::write(&source, b"neu \"0.1\"\nmodule old\nnum answer = 42\n").unwrap();
    let failure = execute([
        constants::COMPILE.to_owned(),
        source.to_string_lossy().into_owned(),
        constants::OUTPUT.to_owned(),
        artifact.to_string_lossy().into_owned(),
        constants::OVERWRITE.to_owned(),
    ])
    .unwrap_err();
    assert_eq!(failure.class().code(), constants::EXIT_VALIDATION);
    assert_eq!(fs::read(artifact).unwrap(), b"keep existing output");
}

#[test]
/// Validation accepts valid source and classifies invalid source without producing files.
fn cli_library_validates_source() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("source.neu");
    fs::write(&source, SOURCE).unwrap();
    let arguments = [
        constants::VALIDATE.to_owned(),
        source.to_string_lossy().into_owned(),
    ];
    execute(arguments.clone()).expect("valid source should validate");
    fs::write(&source, b"invalid source").unwrap();
    let failure = execute(arguments).expect_err("invalid source must fail validation");
    assert_eq!(failure.class().code(), constants::EXIT_VALIDATION);
    assert_ne!(failure.messages(), &[] as &[String]);
    assert_eq!(fs::read_dir(fixture.path()).unwrap().count(), 1);
}

#[test]
/// Formatting writes exact canonical bytes and preserves the captured source.
fn cli_library_formats_canonical_source() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("source.neu");
    let formatted = fixture.path().join("formatted.neu");
    fs::write(&source, SOURCE).unwrap();
    execute([
        constants::FORMAT.to_owned(),
        constants::OUTPUT.to_owned(),
        formatted.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("valid source should format");
    let canonical = fs::read(&formatted).unwrap();
    let text = std::str::from_utf8(&canonical).unwrap();
    assert!(text.starts_with("neu \"1.0\"\nmodule library_test\n"));
    assert!(text.contains("num answer = 42"));
    let formatted_again = fixture.path().join("formatted-again.neu");
    execute([
        constants::FORMAT.to_owned(),
        constants::OUTPUT.to_owned(),
        formatted_again.to_string_lossy().into_owned(),
        formatted.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert_eq!(fs::read(formatted_again).unwrap(), canonical);
    assert_eq!(fs::read(source).unwrap(), SOURCE);
}

#[test]
/// Compilation publishes an independently readable artifact with the expected declaration value.
fn cli_library_compiles_readable_artifact() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("source.neu");
    let artifact = fixture.path().join("artifact.neuir");
    fs::write(&source, SOURCE).unwrap();
    execute([
        constants::COMPILE.to_owned(),
        constants::OUTPUT.to_owned(),
        artifact.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("valid source should compile");
    let bytes = fs::read(artifact).unwrap();
    let scalar = neutral_core::StructuralLimits::new(1_048_576, 64).unwrap();
    let limits = neutral_reader::SemanticLimits {
        project: neutral_encoding::hard_project_limits(),
        composition: neutral_reader::composition::ProjectCompositionLimits::from_vocabulary(
            neutral_reader::composition::CompositionScalarLimits::from_structural(scalar),
        ),
        expression: neutral_reader::expression::profile::ExpressionLimits::hard(),
        expansion: neutral_ir::expression::templates::ExpansionLimits::default(),
    };
    let reader = decode_expression_project(
        &bytes,
        DecodeLimits::hard(),
        limits,
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(reader.ir().declarations.len(), 1);
    assert!(
        matches!(&reader.ir().declarations[0].body, Some(neutral_ir::expression::ExpressionBody::Value(neutral_ir::composition::BindingValue::Number(value))) if value == &ExactNumber::from_source("42", 64, 64).unwrap())
    );
    assert_eq!(fs::read(source).unwrap(), SOURCE);
}

#[test]
/// Output protection preserves existing bytes until explicit overwrite replaces them completely.
fn cli_library_requires_explicit_overwrite() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("source.neu");
    let formatted = fixture.path().join("formatted.neu");
    fs::write(&source, SOURCE).unwrap();
    fs::write(&formatted, b"existing output to preserve").unwrap();
    let arguments = [
        constants::FORMAT.to_owned(),
        constants::OUTPUT.to_owned(),
        formatted.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ];
    let failure = execute(arguments.clone()).expect_err("existing output requires overwrite");
    assert_eq!(failure.class().code(), constants::EXIT_OUTPUT);
    assert_eq!(failure.messages(), ["output-exists"]);
    assert_eq!(
        fs::read(&formatted).unwrap(),
        b"existing output to preserve"
    );
    execute(
        arguments
            .into_iter()
            .chain([constants::OVERWRITE.to_owned()]),
    )
    .expect("explicit overwrite should replace output");
    let text = fs::read(formatted).unwrap();
    assert!(
        std::str::from_utf8(&text)
            .unwrap()
            .contains("num answer = 42")
    );
    assert_eq!(fs::read(source).unwrap(), SOURCE);
    assert_eq!(fs::read_dir(fixture.path()).unwrap().count(), 2);
}
