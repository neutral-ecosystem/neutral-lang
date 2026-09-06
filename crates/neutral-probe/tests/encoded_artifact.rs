// SPDX-License-Identifier: Apache-2.0

//! System proof for standalone encoded-artifact inspection.

use neutral_core::{ByteSpan, SourceContentDigest, StructuralLimits};
use neutral_encoding::{ProducerInfo, encode};
use neutral_ir::{
    AcceptancePartition, CompilationArtifacts, Declaration, DeclarationFingerprint,
    DerivationManifest, ElementId, ExactNumber, LANGUAGE_BEHAVIOR_VERSION, LogicalDocument,
    LogicalModuleIdentity, LogicalValue, ModuleSymbolIdentity, Normalization, ProvenanceRecord,
    ResolvedType, ResourceFacts, SourceMap, SourceMapEntry, ValueOrigin,
};
use neutral_reader::ValidatedDocument;
use std::{fs, path::PathBuf, process::Command, sync::Arc};

/// Captured source bytes represented by the manually constructed test artifact.
const SOURCE: &[u8] = b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n";
/// Module-header end in the captured fixture.
const MODULE_END: u64 = 24;
/// Declaration start in the captured fixture.
const DECLARATION_START: u64 = 26;
/// Numeric digit and scale ceiling used by the fixture.
const NUMBER_LIMIT: u64 = 64;
/// Source and structural ceiling used by the fixture.
const STRUCTURAL_LIMIT: u64 = 4_096;
/// Retained diagnostic ceiling used by the fixture.
const DIAGNOSTIC_LIMIT: u32 = 16;

/// Builds one valid external artifact without using the compiler crate.
fn encoded_fixture() -> Vec<u8> {
    let element_id = ElementId::new(1);
    let module = LogicalModuleIdentity::new(LANGUAGE_BEHAVIOR_VERSION, "sample");
    let value = LogicalValue::Number(
        ExactNumber::from_source("42", NUMBER_LIMIT, NUMBER_LIMIT)
            .expect("fixture number must normalize"),
    );
    let fingerprint = DeclarationFingerprint::for_binding(&ResolvedType::Num, &value)
        .expect("fixture fingerprint must fit fixed framing");
    let declaration = Declaration::new(
        element_id,
        ModuleSymbolIdentity::new(module.clone(), "answer"),
        fingerprint,
        "answer",
        ResolvedType::Num,
        value,
    );
    let source_length = u64::try_from(SOURCE.len()).expect("fixture length must fit u64");
    let module_span = ByteSpan::new(0, MODULE_END).expect("module span must be ordered");
    let declaration_span =
        ByteSpan::new(DECLARATION_START, source_length).expect("declaration span must be ordered");
    let source_map = SourceMap::new(
        SourceContentDigest::from_bytes(SOURCE),
        source_length,
        module_span,
        vec![SourceMapEntry::new(
            element_id,
            declaration_span,
            declaration_span,
            declaration_span,
            declaration_span,
        )],
    );
    let limits = StructuralLimits::new(STRUCTURAL_LIMIT, DIAGNOSTIC_LIMIT)
        .expect("fixture limits must be nonzero");
    let artifacts = CompilationArtifacts::new(
        LogicalDocument::new(module, vec![declaration]),
        source_map,
        vec![ProvenanceRecord::new(
            element_id,
            ValueOrigin::ExplicitSource,
            Normalization::ExactNumberCanonicalization,
        )],
        DerivationManifest::new(
            LANGUAGE_BEHAVIOR_VERSION,
            SourceContentDigest::from_bytes(SOURCE),
            AcceptancePartition::from_limits(limits),
            ResourceFacts::new(source_length, 1, 0, 0),
        ),
    );
    let document = ValidatedDocument::from_compiler_output(Arc::new(artifacts))
        .expect("manual artifacts must satisfy reader invariants");
    encode(
        &document,
        &ProducerInfo::new("neutral-probe-test", env!("CARGO_PKG_VERSION")),
    )
    .expect("fixture must encode")
    .into_bytes()
}

/// Returns a process-unique path for the encoded fixture.
fn temporary_artifact_path() -> PathBuf {
    std::env::temp_dir().join(format!("neutral-probe-system-{}.nir", std::process::id()))
}

#[test]
/// Proves the standalone executable inspects an artifact without compiler linkage.
fn system_standalone_probe_inspects_encoded_artifact_without_compiler() {
    let path = temporary_artifact_path();
    fs::write(&path, encoded_fixture()).expect("fixture artifact must be writable");
    let output = Command::new(env!("CARGO_BIN_EXE_neutral-probe"))
        .arg(&path)
        .output()
        .expect("standalone probe must execute");
    fs::remove_file(&path).expect("fixture artifact must be removable");

    let stdout = String::from_utf8(output.stdout).expect("probe stdout must be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("probe stderr must be UTF-8");
    assert!(output.status.success(), "probe failed: {stderr}");
    assert!(stdout.contains("[info] module sample"));
    assert!(stdout.contains("[info] declaration answer: num = 42/1"));
    assert!(stderr.is_empty());
}
