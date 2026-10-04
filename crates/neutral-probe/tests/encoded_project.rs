// SPDX-License-Identifier: Apache-2.0

//! Independent executable proof for complete project artifacts and redaction.

use neutral_core::{
    ByteSpan, CancellationToken, SourceContentDigest, SourceLocation, profile::V1_SOURCE_PROFILE,
};
use neutral_encoding::{
    DecodeLimits,
    project::{encode_project, hard_project_limits},
};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    project::{
        PROJECT_IR_SCHEMA, ProjectDeclaration, ProjectIr, ProjectModule, ProjectProvenance,
        ProjectResourceFacts, ProjectSource, ProjectSourceMap, ProjectValue,
    },
    project_interface::{
        ProjectInterface, ProjectPublicEdgeKind, ProjectPublicSignature, ProjectPublicType,
    },
};
use neutral_probe::project::{
    ProjectProbeError, inspect_project_encoded, render_project_summary_json,
};
use neutral_reader::{ProjectReadError, ValidatedProject};
use std::{fs, process::Command, sync::Arc};

/// Constructs a reader-valid two-module project using only public IR constructors.
fn fixture() -> ValidatedProject {
    let module = |name| LogicalModuleIdentity::new(V1_SOURCE_PROFILE, name);
    let symbol = |owner, name| ModuleSymbolIdentity::new(module(owner), name);
    let main_source = b"neu \"1.0\"\nmodule consumer\nimport shared as shared\npublic num copied = shared::answer\npublic Ref<num> pointer = ref(shared::answer)\n";
    let shared_source =
        b"neu \"1.0\"\nmodule shared\nnum hidden = 42\npublic num answer = hidden\n";
    let main_digest = SourceContentDigest::from_bytes(main_source);
    let shared_digest = SourceContentDigest::from_bytes(shared_source);
    let main_location = SourceLocation::new(
        main_digest,
        ByteSpan::new(1, main_source.len() as u64).unwrap(),
    );
    let shared_location = SourceLocation::new(
        shared_digest,
        ByteSpan::new(1, shared_source.len() as u64).unwrap(),
    );
    let declarations = declarations();
    let source_maps = declarations
        .iter()
        .map(|d| ProjectSourceMap {
            declaration: d.identity.clone(),
            location: if d.identity.module().module_name() == "consumer" {
                main_location
            } else {
                shared_location
            },
        })
        .collect();
    let mut ir = ProjectIr {
        schema: PROJECT_IR_SCHEMA.to_owned(),
        modules: vec![
            ProjectModule {
                identity: module("consumer"),
                imports: vec!["shared".to_owned()],
            },
            ProjectModule {
                identity: module("shared"),
                imports: Vec::new(),
            },
        ],
        declarations,
        vocabulary_records: Vec::new(),
        public_interface: ProjectInterface::with_computed_fingerprint(Vec::new(), Vec::new())
            .unwrap(),
        sources: vec![
            ProjectSource {
                module: "consumer".to_owned(),
                source_id: "private:consumer-input".to_owned(),
                digest: main_digest,
                byte_len: main_source.len() as u64,
            },
            ProjectSource {
                module: "shared".to_owned(),
                source_id: "private:shared-input".to_owned(),
                digest: shared_digest,
                byte_len: shared_source.len() as u64,
            },
        ],
        source_maps,
        provenance: vec![
            ProjectProvenance {
                from: symbol("consumer", "copied"),
                to: symbol("shared", "answer"),
                kind: ProjectPublicEdgeKind::Value,
                location: main_location,
            },
            ProjectProvenance {
                from: symbol("consumer", "pointer"),
                to: symbol("shared", "answer"),
                kind: ProjectPublicEdgeKind::Reference,
                location: main_location,
            },
            ProjectProvenance {
                from: symbol("shared", "answer"),
                to: symbol("shared", "hidden"),
                kind: ProjectPublicEdgeKind::Value,
                location: shared_location,
            },
        ],
        limits: hard_project_limits(),
        resources: ProjectResourceFacts {
            source_units: 2,
            source_bytes: (main_source.len() + shared_source.len()) as u64,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: 4,
            import_edges: 1,
            value_nodes: 4,
        },
        vocabulary_sources: Vec::new(),
    };
    ir.public_interface = ir.recompute_public_interface().unwrap();
    ValidatedProject::from_ir(
        Arc::new(ir),
        hard_project_limits(),
        &CancellationToken::new(),
    )
    .unwrap()
}

/// Invokes the standalone binary against a temporary artifact and removes that file.
fn probe(bytes: &[u8], label: &str, options: &[&str]) -> std::process::Output {
    let path = std::env::temp_dir().join(format!(
        "neutral-project-probe-{}-{label}.nir",
        std::process::id()
    ));
    fs::write(&path, bytes).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_neutral-probe"))
        .args(options)
        .arg(&path)
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    result
}

/// A compiler-free artifact builder and standalone process inspect cross-module public values.
#[test]
fn system_project_probe_reader_only_and_redacted() {
    let token = CancellationToken::new();
    let project = fixture();
    let bytes = encode_project(&project, &token).unwrap();
    let output = probe(&bytes, "json", &["--json", "--root", "consumer::pointer"]);
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(output.stderr, []);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let roots = ["consumer::pointer".to_owned()];
    let summary = inspect_project_encoded(
        &bytes,
        DecodeLimits::hard(),
        hard_project_limits(),
        Some(&roots),
        &token,
    )
    .unwrap();
    assert_eq!(stdout, render_project_summary_json(&summary));
    assert_eq!(summary.modules, 2);
    assert_eq!(summary.declarations, 4);
    assert_eq!(summary.view.exports().len(), 2);
    assert!(stdout.contains("shared::answer"));
    assert!(stdout.contains("Reference"));
    for private in [
        "hidden",
        "private:",
        "source_maps",
        "provenance",
        "SourceContentDigest",
    ] {
        assert!(!stdout.contains(private), "leaked {private}");
    }
    let text = probe(&bytes, "text", &[]);
    assert!(text.status.success());
    assert!(
        String::from_utf8(text.stdout)
            .unwrap()
            .lines()
            .all(|line| line.starts_with(neutral_probe::output::INFO))
    );
}

/// Private/unknown/duplicate roots and hostile bytes fail with no stdout or partial summary.
#[test]
fn security_project_probe_failure_envelopes() {
    let token = CancellationToken::new();
    let bytes = encode_project(&fixture(), &token).unwrap();
    for (label, options) in [
        ("private", vec!["--root", "shared::hidden"]),
        ("absent", vec!["--root", "missing::root"]),
        (
            "duplicate",
            vec!["--root", "consumer::pointer", "--root", "consumer::pointer"],
        ),
    ] {
        let output = probe(&bytes, label, &options);
        assert!(!output.status.success());
        assert_eq!(output.stdout, []);
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .starts_with(neutral_probe::output::ERROR)
        );
    }
    let output = probe(&bytes[..bytes.len() - 1], "truncated", &["--json"]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, []);
    let empty = inspect_project_encoded(
        &bytes,
        DecodeLimits::hard(),
        hard_project_limits(),
        Some(&[]),
        &token,
    )
    .unwrap();
    assert_eq!(empty.view.exports(), []);
    assert_eq!(empty.declarations, 4);
    let roots = ["shared::hidden".to_owned()];
    assert_eq!(
        inspect_project_encoded(
            &bytes,
            DecodeLimits::hard(),
            hard_project_limits(),
            Some(&roots),
            &token
        ),
        Err(ProjectProbeError::View(ProjectReadError::View))
    );
    token.cancel();
    assert!(
        matches!(inspect_project_encoded(&bytes, DecodeLimits::hard(), hard_project_limits(), None, &token), Err(ProjectProbeError::Decode(e)) if e.class() == neutral_encoding::DecodeErrorClass::Cancelled)
    );
}

/// Builds canonical public/private declarations independently of compiler output.
fn declarations() -> Vec<ProjectDeclaration> {
    let number = || ProjectValue::Number(ExactNumber::from_source("42", 64, 64).unwrap());
    let module = |name| LogicalModuleIdentity::new(V1_SOURCE_PROFILE, name);
    let symbol = |owner, name| ModuleSymbolIdentity::new(module(owner), name);
    vec![
        ProjectDeclaration {
            identity: symbol("consumer", "copied"),
            public: true,
            signature: ProjectPublicSignature::Binding(ProjectPublicType::Num),
            value: Some(number()),
            defaults: Vec::new(),
        },
        ProjectDeclaration {
            identity: symbol("consumer", "pointer"),
            public: true,
            signature: ProjectPublicSignature::Binding(ProjectPublicType::Ref(Box::new(
                ProjectPublicType::Num,
            ))),
            value: Some(ProjectValue::Reference(symbol("shared", "answer"))),
            defaults: Vec::new(),
        },
        ProjectDeclaration {
            identity: symbol("shared", "answer"),
            public: true,
            signature: ProjectPublicSignature::Binding(ProjectPublicType::Num),
            value: Some(number()),
            defaults: Vec::new(),
        },
        ProjectDeclaration {
            identity: symbol("shared", "hidden"),
            public: false,
            signature: ProjectPublicSignature::Binding(ProjectPublicType::Num),
            value: Some(number()),
            defaults: Vec::new(),
        },
    ]
}

/// A closed stdout pipe is a typed host failure, never an implementation panic.
#[cfg(unix)]
#[test]
fn system_project_probe_broken_pipe_is_bounded() {
    let path =
        std::env::temp_dir().join(format!("neutral-project-pipe-{}.nir", std::process::id()));
    fs::write(
        &path,
        encode_project(&fixture(), &CancellationToken::new()).unwrap(),
    )
    .unwrap();
    let (writer, reader) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader);
    let mut command = Command::new(env!("CARGO_BIN_EXE_neutral-probe"));
    command
        .arg("--json")
        .arg(&path)
        .stdout(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            writer,
        )))
        .stderr(std::process::Stdio::piped());
    let result = command.output().unwrap();
    fs::remove_file(path).unwrap();
    assert!(!result.status.success());
    assert_eq!(
        String::from_utf8(result.stderr).unwrap(),
        format!("{} probe-output-failed\n", neutral_probe::output::ERROR)
    );
}
