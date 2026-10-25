// SPDX-License-Identifier: Apache-2.0

//! Executable CLI and reader checks for the current expression-project pipeline.
use super::{Command, TestRoot, executable, fs};
use neutral_core::{CancellationToken, StructuralLimits};
use neutral_encoding::{DecodeLimits, expression::decode_expression_project};
use neutral_ir::expression::{profile::ExpressionLimits, templates::ExpansionLimits};
use neutral_reader::SemanticLimits;
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

const SOURCE: &[u8] = include_bytes!("fixtures/cli/service.neu");
fn semantic() -> SemanticLimits {
    let scalar = StructuralLimits::new(1_048_576, 64).unwrap();
    SemanticLimits {
        project: neutral_encoding::hard_project_limits(),
        composition: CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(scalar)),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    }
}
fn success(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Current source formats idempotently, compiles to `/3`, and passes the standalone reader/probe.
#[test]
fn system_cli_current_source_artifact_and_probe() {
    let cli = executable("neutral-cli");
    let probe = executable("neutral-probe");
    let root = TestRoot::new();
    let source = root.0.join("service.neu");
    let formatted = root.0.join("formatted.neu");
    let artifact = root.0.join("service.nir");
    fs::write(&source, SOURCE).unwrap();
    success(Command::new(&cli).arg("validate").arg(&source));
    success(
        Command::new(&cli)
            .arg("format")
            .arg("--output")
            .arg(&formatted)
            .arg(&source),
    );
    success(Command::new(&cli).arg("validate").arg(&formatted));
    let formatted_again = root.0.join("formatted-again.neu");
    success(
        Command::new(&cli)
            .arg("format")
            .arg("--output")
            .arg(&formatted_again)
            .arg(&formatted),
    );
    assert_eq!(
        fs::read(&formatted).unwrap(),
        fs::read(&formatted_again).unwrap()
    );
    success(
        Command::new(&cli)
            .arg("compile")
            .arg("--output")
            .arg(&artifact)
            .arg(&source),
    );
    let bytes = fs::read(&artifact).unwrap();
    assert!(bytes.starts_with(b"NEUP3\r\n\x1a"));
    let reader = decode_expression_project(
        &bytes,
        DecodeLimits::hard(),
        semantic(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(reader.ir().declarations.len(), 2);
    assert_eq!(
        super::materialized(&reader, "service", "answer"),
        &super::number("42")
    );
    assert_eq!(
        super::materialized(&reader, "service", "label"),
        &neutral_ir::composition::BindingValue::String("web".into())
    );
    assert_eq!(
        reader.ir().declarations[0].identity.declaration_name(),
        "answer"
    );
    let output = success(
        Command::new(&probe)
            .arg("--root")
            .arg("service::answer")
            .arg("--json")
            .arg(&artifact),
    );
    assert_eq!(output.stderr, [] as [u8; 0]);
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        summary["encoding"],
        neutral_ir::expression::profile::ENCODING
    );
    assert_eq!(
        summary["public_roots"],
        serde_json::json!(["service::answer"])
    );
    fs::write(&source, include_bytes!("fixtures/cli/invalid.neu")).unwrap();
    let failed = Command::new(&cli)
        .arg("compile")
        .arg("--overwrite")
        .arg("--output")
        .arg(&artifact)
        .arg(&source)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert_eq!(failed.stdout, [] as [u8; 0]);
    assert_eq!(fs::read(&artifact).unwrap(), bytes);
}

/// Old source headers fail closed and preserve the existing output bytes.
#[test]
fn system_cli_rejects_old_source_without_replacing_output() {
    let cli = executable("neutral-cli");
    let root = TestRoot::new();
    let source = root.0.join("old.neu");
    let artifact = root.0.join("existing.nir");
    fs::write(&source, b"neu \"0.1\"\nmodule old\nnum answer = 42\n").unwrap();
    fs::write(&artifact, b"preserve me").unwrap();
    let output = Command::new(&cli)
        .arg("compile")
        .arg("--overwrite")
        .arg("--output")
        .arg(&artifact)
        .arg(&source)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert_eq!(fs::read(&artifact).unwrap(), b"preserve me");
}
