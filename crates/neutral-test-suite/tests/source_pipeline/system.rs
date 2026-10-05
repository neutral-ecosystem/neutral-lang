// SPDX-License-Identifier: Apache-2.0

//! Executable host acquisition and the fully available source-profile compilation pipeline.

use super::{CASES, Command, TestRoot, executable, fs};
use neutral_core::CancellationToken;
use neutral_encoding::DecodeLimits;
use neutral_ir::LogicalValue;

/// Compiler success must satisfy the independent reader even for default-bearing reused records.
#[test]
fn available_service_compiler_output_is_reader_valid() {
    neutral_reader::ValidatedDocument::from_compiler_output(available_artifacts(include_bytes!(
        "fixtures/cli/service.neu"
    )))
    .unwrap();
}

/// Compiles an available-profile fixture without bypassing any semantic or resource checks.
fn available_artifacts(source: &[u8]) -> std::sync::Arc<neutral_ir::CompilationArtifacts> {
    let limits = neutral_core::StructuralLimits::new(16384, 16).unwrap();
    let result = neutral_compiler::compile(neutral_compiler::CompilationRequest::new(
        source.to_vec(),
        limits,
        CancellationToken::new(),
    ))
    .unwrap();
    let neutral_compiler::CompilationResult::Success(artifacts) = result else {
        panic!("available service must compile");
    };
    artifacts
}

/// Root, nullable, list and nested reuse inherit exact provider origins under consumer paths.
#[test]
fn nested_reuse_preserves_field_origins_and_round_trips() {
    let artifacts = available_artifacts(include_bytes!("fixtures/cli/nested-reuse.neu"));
    let reader =
        neutral_reader::ValidatedDocument::from_compiler_output(std::sync::Arc::clone(&artifacts))
            .unwrap();
    for (name, count) in [
        ("copied", 2),
        ("reused_items", 4),
        ("copied_holder", 8),
        ("widened", 2),
    ] {
        let id = reader.declaration_by_name(name).unwrap().element_id();
        assert_eq!(
            artifacts
                .field_provenance()
                .iter()
                .filter(|entry| entry.element_id() == id)
                .count(),
            count,
            "{name}"
        );
    }
    let original = reader.declaration_by_name("original").unwrap().element_id();
    let copied = reader.declaration_by_name("copied").unwrap().element_id();
    let paths = |id| {
        artifacts
            .field_provenance()
            .iter()
            .filter(|entry| entry.element_id() == id)
            .map(|entry| (entry.field_path(), entry.origin()))
            .collect::<Vec<_>>()
    };
    assert_eq!(paths(original), paths(copied));
    assert!(
        paths(copied)
            .iter()
            .all(|(_, origin)| *origin == neutral_ir::ValueOrigin::UserRecordDefault)
    );
    let encoded = neutral_encoding::encode(
        &reader,
        &neutral_encoding::ProducerInfo::new("pipeline-test", env!("CARGO_PKG_VERSION")),
    )
    .unwrap()
    .into_bytes();
    let decoded =
        neutral_encoding::decode(&encoded, DecodeLimits::hard(), &CancellationToken::new())
            .unwrap();
    assert_eq!(decoded.declarations(), reader.declarations());
    assert_eq!(
        neutral_probe::summarize(&decoded),
        neutral_probe::summarize(&reader)
    );
}

/// Inherited evidence remains independently mandatory, unique and path-checked.
#[test]
fn forged_reused_field_provenance_is_rejected() {
    let artifacts = available_artifacts(include_bytes!("fixtures/cli/nested-reuse.neu"));
    let reader =
        neutral_reader::ValidatedDocument::from_compiler_output(std::sync::Arc::clone(&artifacts))
            .unwrap();
    let copied = reader
        .declaration_by_name("copied_holder")
        .unwrap()
        .element_id();
    let position = artifacts
        .field_provenance()
        .iter()
        .position(|entry| entry.element_id() == copied)
        .unwrap();
    for mutation in 0..3 {
        let mut fields = artifacts.field_provenance().to_vec();
        match mutation {
            0 => {
                fields.remove(position);
            }
            1 => fields.push(fields[position].clone()),
            _ => {
                fields[position] = neutral_ir::FieldProvenanceRecord::new(
                    copied,
                    vec!["nonexistent".to_owned()],
                    fields[position].origin(),
                );
            }
        }
        let corrupt = artifacts.as_ref().clone().with_field_provenance(fields);
        assert_eq!(
            neutral_reader::ValidatedDocument::from_compiler_output(std::sync::Arc::new(corrupt))
                .unwrap_err(),
            neutral_reader::ReaderError::InvalidFieldProvenance
        );
    }
}

/// Runs a real host command and requires success with its diagnostic stream as failure context.
fn success(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Actual file capture works for supported project inputs, without pretending CLI compilation is active.
#[test]
fn system_cli_captures_project_files_and_preserves_profile_gate() {
    let cli = executable("neutral-cli");
    let root = TestRoot::new();
    for case in CASES
        .iter()
        .filter(|case| case.units.len() == 1 && !case.vocabulary)
    {
        let (module, text) = case.units[0];
        let path = root.0.join(format!("{}.neu", case.name));
        fs::write(&path, text).unwrap();
        let captured = success(
            Command::new(&cli)
                .args([
                    "capture-project",
                    "--source-id",
                    "pipeline:source",
                    "--module-id",
                    module,
                ])
                .arg(&path),
        );
        assert_eq!(captured.stdout, [] as [u8; 0]);
        assert!(
            String::from_utf8_lossy(&captured.stderr).contains("capture succeeded source-units=1")
        );
        let artifact = root.0.join(format!("{}.nir", case.name));
        let unavailable = Command::new(&cli)
            .arg("compile")
            .arg("--output")
            .arg(&artifact)
            .arg(&path)
            .output()
            .unwrap();
        assert!(!unavailable.status.success());
        assert_eq!(unavailable.stdout, [] as [u8; 0]);
        let expected = if case.name == "reference-cycle" {
            // The available-profile lexer enforces record layout before profile dispatch.
            neutral_compiler::diagnostics::MALFORMED_BOUNDARY
        } else {
            neutral_core::profile::PROFILE_UNAVAILABLE_DIAGNOSTIC
        };
        assert!(
            String::from_utf8_lossy(&unavailable.stderr).contains(expected),
            "{}: {}",
            case.name,
            String::from_utf8_lossy(&unavailable.stderr)
        );
        assert!(!artifact.exists());
        assert_eq!(fs::read(&path).unwrap(), text.as_bytes());
    }
}

/// Available CLI file compilation, formatting, typed defaults/reuse, and standalone results agree.
#[test]
fn system_cli_file_to_artifact_to_probe() {
    let cli = executable("neutral-cli");
    let probe = executable("neutral-probe");
    let root = TestRoot::new();
    let source = root.0.join("service.neu");
    let artifact = root.0.join("service.nir");
    let formatted = root.0.join("formatted.neu");
    fs::write(&source, include_bytes!("fixtures/cli/service.neu")).unwrap();
    success(Command::new(&cli).arg("validate").arg(&source));
    success(
        Command::new(&cli)
            .arg("format")
            .arg("--output")
            .arg(&formatted)
            .arg(&source),
    );
    success(Command::new(&cli).arg("validate").arg(&formatted));
    let compiled = success(
        Command::new(&cli)
            .arg("compile")
            .arg("--output")
            .arg(&artifact)
            .arg(&source),
    );
    assert_eq!(compiled.stdout, [] as [u8; 0]);
    let bytes = fs::read(&artifact).unwrap();
    let reader =
        neutral_encoding::decode(&bytes, DecodeLimits::hard(), &CancellationToken::new()).unwrap();
    assert_eq!(reader.declarations().len(), 3);
    let api = reader.declaration_by_name("api").unwrap();
    let copy = reader.declaration_by_name("copied").unwrap();
    assert_eq!(api.value(), copy.value());
    let LogicalValue::Record(record) = api.value() else {
        panic!("service must be a record");
    };
    let field = |name| {
        record
            .fields()
            .iter()
            .find(|field| field.name() == name)
            .unwrap()
            .value()
    };
    assert_eq!(field("label"), &LogicalValue::String("web".to_owned()));
    assert_eq!(
        field("replicas"),
        &LogicalValue::Number(neutral_ir::ExactNumber::from_source("2", 128, 128).unwrap())
    );
    assert_eq!(
        field("ports"),
        &LogicalValue::List(vec![
            LogicalValue::Number(neutral_ir::ExactNumber::from_source("80", 128, 128).unwrap()),
            LogicalValue::Number(neutral_ir::ExactNumber::from_source("443", 128, 128).unwrap()),
        ])
    );
    let output = success(Command::new(&probe).arg("--json").arg(&artifact));
    assert_eq!(output.stderr, [] as [u8; 0]);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        neutral_probe::render_summary_json(&neutral_probe::summarize(&reader))
    );
    let canonical_artifact = root.0.join("canonical.nir");
    success(
        Command::new(&cli)
            .arg("compile")
            .arg("--output")
            .arg(&canonical_artifact)
            .arg(&formatted),
    );
    let canonical = neutral_encoding::decode(
        &fs::read(&canonical_artifact).unwrap(),
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(reader.declarations(), canonical.declarations());
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
    assert!(String::from_utf8_lossy(&failed.stderr).starts_with("[error]"));
    assert_eq!(
        fs::read(&artifact).unwrap(),
        bytes,
        "failure must not replace the last valid artifact"
    );
}
