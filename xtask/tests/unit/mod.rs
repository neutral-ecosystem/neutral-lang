// SPDX-License-Identifier: Apache-2.0

//! Tests for dependency-boundary policy failures.

use super::configuration::quality_value_from;
use super::interface::{
    BuildProfile, CiProfile, FuzzMode, PerformanceProfile, PortableAction, QualityAction,
    QualityProfile, Task, TestLevel, ValidationTarget, VersionAction,
};
use super::versioning::{
    release_evidence_readme, replace_workspace_lock_versions, replace_workspace_package_version,
    validate_version_transition,
};
use super::{
    constants, contract_ids, ensure_ids_covered, ensure_syntax_complete, render_rustdoc_index,
    rustdoc_header_configuration, set, source_has_non_path_test_configuration,
    validate_allowed_packages, validate_direct_dependencies,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
/// Verifies that invalid automation command syntax is rejected.
fn automation_rejects_an_invalid_command() {
    assert!(super::run(["unknown".to_owned()]).is_err());
}

#[test]
/// Unsupported profiles must fail before starting an external campaign.
fn automation_rejects_unknown_campaign_profiles() {
    for arguments in [
        vec!["build", "--profile", "unknown"],
        vec!["test", "unknown"],
        vec!["test", "performance", "--profile", "unknown"],
        vec!["fuzz", "unknown"],
        vec!["ci", "unknown"],
        vec!["quality", "unknown"],
        vec!["ci", "stage1"],
        vec!["format"],
        vec!["progress"],
        vec!["clean-results"],
    ] {
        assert!(super::run(arguments.into_iter().map(str::to_owned)).is_err());
    }
}

#[test]
/// Stable command shapes parse independently from external command execution.
fn automation_parses_the_stable_command_surface() {
    let cases = [
        (vec!["dev"], Task::Dev),
        (vec!["fmt"], Task::Format { write: false }),
        (vec!["fmt", "--write"], Task::Format { write: true }),
        (vec!["lint"], Task::Lint),
        (vec!["check"], Task::Check),
        (
            vec!["build", "--profile", "release"],
            Task::Build(BuildProfile::Release),
        ),
        (
            vec!["test", "conformance"],
            Task::Test(TestLevel::Conformance),
        ),
        (
            vec!["test", "performance", "--profile", "soak"],
            Task::Performance(PerformanceProfile::Soak),
        ),
        (vec!["fuzz", "campaign"], Task::Fuzz(FuzzMode::Campaign)),
        (
            vec!["quality", "--profile", "release"],
            Task::Quality(QualityAction::Run(QualityProfile::Release)),
        ),
        (
            vec!["quality", "evaluate", "--profile", "release"],
            Task::Quality(QualityAction::Evaluate(QualityProfile::Release)),
        ),
        (
            vec!["quality", "status"],
            Task::Quality(QualityAction::Status),
        ),
        (
            vec!["quality", "approve", "--release", "v0.1.0"],
            Task::Quality(QualityAction::Approve("v0.1.0".to_owned())),
        ),
        (
            vec!["quality", "render"],
            Task::Quality(QualityAction::Render),
        ),
        (
            vec!["quality", "verify"],
            Task::Quality(QualityAction::Verify),
        ),
        (
            vec!["validate", "artifact.nir"],
            Task::Validate(ValidationTarget::Artifact("artifact.nir".into())),
        ),
        (vec!["package"], Task::Package),
        (vec!["release", "prepare"], Task::ReleasePrepare),
        (vec!["release", "tag"], Task::ReleaseTag),
        (
            vec!["version", "prepare", "0.2.0-rc.1"],
            Task::Version(VersionAction::Prepare("0.2.0-rc.1".to_owned())),
        ),
        (
            vec!["portable", "install", "/tmp/reviewed-portable"],
            Task::Portable(PortableAction::Install("/tmp/reviewed-portable".into())),
        ),
        (vec!["clean"], Task::Clean),
        (vec!["ci", "pr"], Task::Ci(CiProfile::Pr)),
    ];
    for (arguments, expected) in cases {
        let arguments = arguments.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(super::interface::parse(&arguments), Ok(expected));
    }
}

#[test]
/// Missing files and process failures retain actionable diagnostics.
fn automation_reports_io_failures() {
    let directory =
        std::env::temp_dir().join(format!("neutral-xtask-missing-{}", std::process::id()));
    assert!(!directory.exists());
    assert!(
        super::read_workspace_text(&directory, "missing")
            .unwrap_err()
            .contains("could not read missing")
    );
    assert!(
        super::collect_regular_files(&directory, &mut Vec::new())
            .unwrap_err()
            .contains("could not inspect")
    );
    assert!(
        super::command_output(directory.to_str().unwrap(), &[])
            .unwrap_err()
            .contains("could not run")
    );
    assert!(
        super::command_output("rustc", &["--invalid-neutral-test-option"])
            .unwrap_err()
            .contains("failed with")
    );
    assert!(super::configuration::test_minimums("missing").is_err());
}

#[cfg(unix)]
#[test]
/// Successful commands with malformed bytes cannot bypass UTF-8 validation.
fn automation_rejects_non_utf8_command_output() {
    assert!(
        super::command_output("sh", &["-c", "printf '\\377'"])
            .unwrap_err()
            .contains("non-UTF-8")
    );
}

#[test]
/// Verifies tool arguments come from the active quality-gate configuration.
fn automation_reads_named_quality_values() {
    let configuration =
        "[coverage]\nminimum_line_percent = 85\n\n[fuzz]\ntargets = [\"source\", \"ir\"]\n";
    assert_eq!(
        quality_value_from(configuration, "coverage", "minimum_line_percent"),
        Some("85".to_owned())
    );
    assert_eq!(
        super::configuration_array_from(configuration, "fuzz", "targets").expect("typed array"),
        ["source", "ir"]
    );
}

#[test]
/// Root settings cannot accidentally inherit a same-named value from a section.
fn automation_keeps_root_configuration_scoped() {
    let configuration = "schema_version = 1\n[other]\nschema_version = 2\n";
    assert_eq!(
        super::configuration_value(configuration, "schema_version"),
        Some("1".to_owned())
    );
    assert_eq!(
        super::configuration_value("[other]\nname = \"x\"", "name"),
        None
    );
    assert_eq!(
        super::configuration_array_from(
            "locks = [\"root.lock\"]\n[other]\nlocks = [\"other.lock\"]\n",
            "",
            "locks"
        )
        .unwrap(),
        ["root.lock"]
    );
    assert!(
        super::configuration_array_from("[other]\nlocks = [\"other.lock\"]\n", "", "locks")
            .is_err()
    );
}

#[test]
/// Supported hosts may contain several configured targets without exact-text matching.
fn automation_reads_multiple_supported_hosts() {
    let policy = "[hosts]\nsupported = [\"host-a\", \"host-b\"]\n";
    assert_eq!(
        super::configuration_array_from(policy, "hosts", "supported")
            .expect("supported host array"),
        ["host-a", "host-b"]
    );
}

#[test]
/// Workspace members are read from multiline Cargo configuration, not crate paths.
fn automation_reads_multiline_workspace_members() {
    let manifest = "[workspace]\nmembers = [\n    \"packages/engine\",\n    \"tools/runner\",\n]\n";
    assert_eq!(
        super::configuration_array_from(manifest, "workspace", "members")
            .expect("workspace members"),
        ["packages/engine", "tools/runner"]
    );
}

#[test]
/// Workspace package discovery follows Cargo members instead of directory names.
fn automation_resolves_workspace_members_from_cargo() {
    let root = super::workspace_root().expect("workspace root");
    let metadata = super::cargo_discovery::metadata(&root, false).expect("Cargo workspace");
    let manifests = super::workspace_package_manifests(&root).expect("package manifests");
    assert_eq!(manifests.len(), metadata.workspace_members.len());
    assert!(manifests.iter().all(|path| path.is_file()));
    let slug = super::project_slug(&root).expect("safe project name");
    assert_ne!(slug, "");
}

#[test]
/// Package transitions follow `SemVer` precedence and reject invalid versions.
fn automation_validates_package_version_transitions() {
    assert!(validate_version_transition("0.1.0-rc.1", "0.1.0-rc.2").is_ok());
    assert!(validate_version_transition("0.1.0-rc.2", "0.1.0").is_ok());
    assert!(validate_version_transition("0.1.0", "0.2.0").is_ok());
    assert!(validate_version_transition("0.1.0", "0.1.0").is_err());
    assert!(validate_version_transition("0.2.0", "0.1.0").is_err());
    assert!(super::validate_semver("0.1.0-rc.01").is_err());
    assert!(super::validate_semver("0.1.0-rc..1").is_err());
}

#[test]
/// Version preparation changes only the root workspace package field.
fn automation_replaces_the_workspace_package_version() {
    let manifest = "[workspace]\nresolver = \"3\"\n\n[workspace.package]\nversion = \"0.2.0\"\nlicense = \"Apache-2.0\"\n\n[workspace.dependencies]\nversion = \"0.2.0\"\n";
    let updated = replace_workspace_package_version(manifest, "0.2.0", "0.3.0")
        .expect("workspace version should update");
    assert!(updated.contains("[workspace.package]\nversion = \"0.3.0\""));
    assert!(updated.contains("[workspace.dependencies]\nversion = \"0.2.0\""));
}

#[test]
/// Version preparation updates lockfile records only for workspace packages.
fn automation_replaces_workspace_lock_versions() {
    let lock = "[[package]]\nname = \"neutral-core\"\nversion = \"0.2.0\"\n\n[[package]]\nname = \"third-party\"\nversion = \"0.2.0\"\n";
    let packages = BTreeSet::from(["neutral-core".to_owned()]);
    let updated = replace_workspace_lock_versions(lock, &packages, "0.2.0", "0.3.0")
        .expect("workspace lock entry should update");
    assert!(updated.contains("name = \"neutral-core\"\nversion = \"0.3.0\""));
    assert!(updated.contains("name = \"third-party\"\nversion = \"0.2.0\""));
}

#[test]
/// Prepared releases receive the license-derived quality-evidence scaffold.
fn automation_renders_release_evidence_scaffold() {
    let evidence = release_evidence_readme("0.3.0", "Apache-2.0");
    assert!(evidence.starts_with("<!-- SPDX-License-Identifier: Apache-2.0 -->"));
    assert!(evidence.contains("Neutral v0.3.0 quality evidence"));
}

#[test]
/// Contract-version sections stay separate from package release versions.
fn automation_reads_all_contract_version_domains() {
    let values = super::configuration_section(
        "[contract_versions]\nlanguage_behavior = \"0.1.0\"\nexternal_ir_encoding = \"NIR-CBOR/0.1\"\n",
        "contract_versions",
    )
    .expect("contract version section should parse");
    assert_eq!(values.len(), 2);
    assert!(values.contains(&("external_ir_encoding".to_owned(), "NIR-CBOR/0.1".to_owned())));
}

#[test]
/// Portable link discovery excludes remote URLs and document anchors.
fn automation_extracts_only_local_portable_links() {
    let links = super::markdown_link_targets(
        "[local](specs/README.md) [anchor](#part) [remote](https://example.com) [section](PLAN.md#gate)",
    );
    assert_eq!(links, ["specs/README.md", "PLAN.md"]);
}

#[test]
/// Exact-byte SHA-256 output is stable and lowercase.
fn automation_hashes_snapshot_bytes_deterministically() {
    assert_eq!(
        super::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
/// Quality status rendering is deterministic and contains release state.
fn automation_renders_quality_approval_status() {
    let approval = super::QualityApproval {
        release: "v1.0.0".to_owned(),
        commit: "0123456789012345678901234567890123456789".to_owned(),
        status: "approved".to_owned(),
        approved_at: "1791057134".to_owned(),
        evaluation: "quality/evaluations/example/release.toml".to_owned(),
        quality_gates_sha256: "b".repeat(64),
    };
    let rendered = super::quality_status_markdown(&[approval], "LicenseRef-Neutral-Test")
        .expect("approval timestamps are valid");
    assert!(rendered.starts_with("<!-- SPDX-License-Identifier: LicenseRef-Neutral-Test -->"));
    assert!(rendered.contains("`v1.0.0`"));
    assert!(rendered.contains("| approved | `03-10-2026` |"));
    assert!(super::is_sha256(&"a".repeat(64)));
    assert!(!super::is_sha256(&"A".repeat(64)));
}

#[test]
/// Quality approval dates format historical records and UTC leap days consistently.
fn automation_formats_quality_approval_dates() {
    assert_eq!(
        super::quality_approval_date("2026-09-11"),
        Ok("11-09-2026".to_owned())
    );
    assert_eq!(
        super::quality_approval_date("0"),
        Ok("01-01-1970".to_owned())
    );
    assert_eq!(
        super::quality_approval_date("951782400"),
        Ok("29-02-2000".to_owned())
    );
    assert!(super::quality_approval_date("not-a-date").is_err());
    assert!(super::quality_approval_date(&u64::MAX.to_string()).is_err());
}

#[test]
/// Portable series identifiers accept future numeric versions without package coupling.
fn automation_accepts_numeric_portable_series() {
    assert!(super::is_portable_series("v0"));
    assert!(super::is_portable_series("v1"));
    assert!(super::is_portable_series("v12"));
    assert!(!super::is_portable_series("v"));
    assert!(!super::is_portable_series("1"));
    assert!(!super::is_portable_series("v1-beta"));
}

#[test]
/// Portable layout validation accepts package-owned contract and checklist filenames.
fn automation_accepts_version_independent_portable_layouts() {
    let root = std::env::temp_dir().join(format!(
        "neutral-generic-portable-layout-{}",
        std::process::id()
    ));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("stale portable test root should be removable");
    }
    for directory in [
        constants::PORTABLE_CONTRACT_DIRECTORY,
        constants::PORTABLE_FIXTURE_DIRECTORY,
        constants::PORTABLE_ORACLE_DIRECTORY,
    ] {
        std::fs::create_dir_all(root.join(directory))
            .expect("required portable directory should be creatable");
    }
    for file in [
        constants::PORTABLE_PLAN_FILE,
        constants::PORTABLE_LIFECYCLE_FILE,
        constants::PORTABLE_REQUIREMENTS_FILE,
        constants::PORTABLE_TRACEABILITY_FILE,
        constants::PORTABLE_CONTRACT_FREEZE_FILE,
        constants::PORTABLE_CONFORMANCE_MANIFEST_FILE,
    ] {
        std::fs::write(root.join(file), "required = true\n")
            .expect("required portable file should be writable");
    }
    std::fs::write(
        root.join(constants::PORTABLE_CONTRACT_DIRECTORY)
            .join("future-contract-name.md"),
        "# Future contract\n",
    )
    .expect("package-owned contract should be writable");

    assert!(super::verify_portable_layout(&root).is_ok());
    std::fs::remove_dir_all(root).expect("portable test root should be removable");
}

#[test]
/// Active freeze validation accepts path-only inputs without recurring digests.
fn automation_accepts_version_independent_frozen_inputs() {
    let root = std::env::temp_dir().join(format!(
        "neutral-generic-portable-freeze-{}",
        std::process::id()
    ));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("stale freeze test root should be removable");
    }
    std::fs::create_dir_all(root.join("portable/custom"))
        .expect("frozen input directory should be creatable");
    let bytes = b"future portable input\n";
    std::fs::write(root.join("portable/custom/input.data"), bytes)
        .expect("frozen input should be writable");
    std::fs::write(
        root.join("freeze.toml"),
        "[fixture_corpus]\nfuture_input_path = \"portable/custom/input.data\"\n",
    )
    .expect("freeze manifest should be writable");

    assert!(super::verify_frozen_input_digests(&root, "freeze.toml").is_ok());
    std::fs::remove_dir_all(root).expect("freeze test root should be removable");
}

#[test]
/// Portable snapshot verification detects copied-byte corruption.
fn automation_revalidates_portable_snapshot_bytes() {
    let root = std::env::temp_dir().join(format!(
        "neutral-portable-snapshot-test-{}",
        std::process::id()
    ));
    let bytes = b"portable";
    let record = format!(
        "{}  {}  portable/test.txt\n",
        super::sha256_hex(bytes),
        bytes.len()
    );
    let tree = super::sha256_hex(record.as_bytes());
    let snapshot = root.join("snapshots").join(tree);
    let copied = snapshot.join("content/portable/test.txt");
    std::fs::create_dir_all(copied.parent().expect("copied file should have a parent"))
        .expect("snapshot content should be creatable");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace.package]\nlicense = \"LicenseRef-Neutral-Test\"\n",
    )
    .expect("test workspace manifest should be writable");
    std::fs::write(&copied, bytes).expect("snapshot content should be writable");
    std::fs::write(
        snapshot.join("manifest.sha256"),
        format!(
            "{}\n{record}",
            super::line_spdx_marker("LicenseRef-Neutral-Test")
        ),
    )
    .expect("snapshot manifest should be writable");
    assert!(super::verify_portable_snapshot_directory(&root, &snapshot).is_ok());
    std::fs::write(&copied, b"corrupt").expect("snapshot corruption should be writable");
    assert!(super::verify_portable_snapshot_directory(&root, &snapshot).is_err());
    std::fs::remove_dir_all(root).expect("temporary snapshot should be removable");
}

#[test]
/// Project license parsing and SPDX rendering use the workspace manifest value.
fn automation_derives_spdx_markers_from_the_workspace_license() {
    let manifest = "[workspace.package]\nversion = \"1.2.3\"\nlicense = \"MIT OR Apache-2.0\"\n";
    let license =
        super::workspace_package_license(manifest).expect("workspace license should parse");

    assert_eq!(license, "MIT OR Apache-2.0");
    assert_eq!(
        super::line_spdx_marker(&license),
        "# SPDX-License-Identifier: MIT OR Apache-2.0"
    );
    assert_eq!(
        super::html_spdx_marker(&license),
        "<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->"
    );
}

#[test]
/// Failed workflows retain the failed step and JSON-safe diagnostic automatically.
fn automation_records_failed_workflow_steps() {
    let error = super::run_recorded_workflow(
        "unit-test",
        "failure",
        vec![(
            "synthetic",
            Box::new(|| Err("line one\n\"line two\"".to_owned())),
        )],
    )
    .expect_err("synthetic workflow should fail");
    let directory = error
        .split_once("workflow log: ")
        .map(|(_, directory)| super::workspace_root().unwrap().join(directory))
        .expect("failure should identify its workflow log");
    let summary = std::fs::read_to_string(directory.join("summary.json"))
        .expect("failed workflow summary should exist");
    let events = std::fs::read_to_string(directory.join("events.jsonl"))
        .expect("failed workflow events should exist");

    assert!(summary.contains("\n  \"status\": \"fail\","));
    assert!(summary.contains("\n  \"worktree_clean\": "));
    assert!(summary.contains("line one\\n\\\"line two\\\""));
    assert!(events.contains("\"step\":\"synthetic\""));
    assert!(events.contains("\"status\":\"fail\""));
}

#[test]
/// Verifies that the environment manifest identifies the selected toolchain channel.
fn environment_manifest_identifies_the_toolchain_channel() {
    let manifest = super::environment::environment_manifest()
        .expect("environment manifest should be available");
    let channel = super::environment::rust_channel().expect("toolchain channel should be readable");
    let stage = super::environment::active_stage().expect("active stage should be readable");
    assert!(manifest.contains(&format!("\"rust_channel\": \"{channel}\"")));
    assert!(manifest.contains(&format!("\"active_stage\": {stage}")));
    assert!(manifest.contains("\"tools\": {"));
    assert!(!manifest.contains("\"workspace_root\""));
}

#[test]
/// Verifies missing workstation tools produce an actionable diagnostic.
fn missing_environment_tool_reports_an_install_action() {
    let tool = super::environment::ToolSpec {
        key: "missing",
        label: "Missing test tool",
        command: "neutral-command-that-must-not-exist".to_owned(),
        arguments: &["--version"],
        install_hint: "install the missing test tool",
    };
    let error = super::environment::tool_version(&tool).expect_err("missing tool must fail");
    assert!(error.contains("Missing test tool is unavailable"));
    assert!(error.contains("install the missing test tool"));
}

#[test]
/// Verifies the stable channel excludes prerelease compiler identities.
fn stable_toolchain_channel_rejects_prereleases() {
    assert!(super::environment::rust_version_matches_channel(
        "rustc 1.98.1 (stable-hash 2026-09-03)",
        "stable",
    ));
    assert!(!super::environment::rust_version_matches_channel(
        "rustc 1.99.0-nightly (nightly-hash 2026-09-05)",
        "stable",
    ));
}

#[test]
/// Verifies that the rustdoc index embeds metadata without an HTML script escape.
fn automation_generates_a_safe_rustdoc_index() {
    let generated = render_rustdoc_index(r#"{"description":"</script>"}"#)
        .expect("the rustdoc template should contain its metadata placeholder");
    assert!(!generated.contains(constants::CARGO_METADATA_PLACEHOLDER));
    assert!(generated.contains(r#"{"description":"\u003c/script>"}"#));
    assert!(generated.contains("Language capabilities"));
    assert!(generated.contains("cargo xtask docs"));
}

#[test]
/// Verifies that the shared rustdoc header contributes a stable cache token.
fn automation_tracks_the_rustdoc_header_content() {
    let configuration = rustdoc_header_configuration();
    assert!(configuration.starts_with(constants::RUSTDOC_HEADER_CFG_PREFIX));
    assert_eq!(configuration, rustdoc_header_configuration());
    assert!(super::RUSTDOC_HEADER_TEMPLATE.contains("neutral-docs-nav"));
    assert!(super::RUSTDOC_HEADER_TEMPLATE.contains("assets/neutral-logo.png"));
}

#[test]
/// Verifies accepted identifiers are extracted without Markdown punctuation.
fn automation_extracts_contract_identifiers() {
    assert_eq!(
        contract_ids("- **NL-ONE-001:** first\n`NL-TWO-002`", "NL-"),
        BTreeSet::from(["NL-ONE-001".to_owned(), "NL-TWO-002".to_owned()])
    );
}

#[test]
/// Verifies a missing accepted identifier fails traceability coverage.
fn automation_rejects_missing_traceability_identifiers() {
    let identifiers = BTreeSet::from(["NL-ONE-001".to_owned(), "NL-TWO-002".to_owned()]);
    assert!(ensure_ids_covered("test", &identifiers, "NL-ONE-001").is_err());
}

#[test]
/// Verifies unchecked syntax items fail the completed traceability gate.
fn automation_rejects_unchecked_syntax_contracts() {
    assert!(ensure_syntax_complete("syntax.md", "- [ ] SYN-GOV-001").is_err());
    assert!(ensure_syntax_complete("syntax.md", "- [x] SYN-GOV-001").is_ok());
}

#[test]
/// Verifies that a compiler-to-CLI edge violates the workspace policy.
fn workspace_rejects_a_forbidden_direct_compiler_dependency() {
    let actual = BTreeSet::from([
        constants::NEUTRAL_CLI.to_owned(),
        constants::NEUTRAL_CORE.to_owned(),
        constants::NEUTRAL_IR.to_owned(),
        constants::NEUTRAL_VOCABULARY.to_owned(),
    ]);

    assert!(
        validate_direct_dependencies(
            constants::NEUTRAL_COMPILER,
            &actual,
            &set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_VOCABULARY
            ]),
        )
        .is_err()
    );
}

#[test]
/// Verifies that core may depend only on the reviewed SHA-256 value utility.
fn core_allows_only_the_reviewed_sha256_dependency() {
    let actual = BTreeSet::from(["sha2".to_owned()]);
    assert!(validate_direct_dependencies(constants::NEUTRAL_CORE, &actual, &set(["sha2"])).is_ok());
}

#[test]
/// Verifies that a compiler package in the probe closure is rejected.
fn probe_allowlist_rejects_a_compiler_dependency_in_the_probe_closure() {
    let actual = BTreeSet::from([
        constants::NEUTRAL_COMPILER.to_owned(),
        constants::NEUTRAL_CORE.to_owned(),
        constants::NEUTRAL_PROBE.to_owned(),
        constants::NEUTRAL_READER.to_owned(),
    ]);

    assert!(
        validate_allowed_packages(
            "neutral-probe dependency tree",
            &actual,
            &set([
                constants::NEUTRAL_PROBE,
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_ENCODING,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_READER,
                constants::NEUTRAL_VOCABULARY,
            ]),
        )
        .is_err()
    );
}

#[test]
/// Verifies that automation cleanup cannot target a parent or workspace path.
fn automation_rejects_unsafe_result_paths() {
    assert!(super::results::is_safe_result_path(std::path::Path::new(
        "test-results"
    )));
    assert!(!super::results::is_safe_result_path(std::path::Path::new(
        "."
    )));
    assert!(!super::results::is_safe_result_path(std::path::Path::new(
        "../test-results"
    )));
    assert!(!super::results::is_safe_result_path(std::path::Path::new(
        "target"
    )));
    assert!(!super::results::is_safe_result_path(std::path::Path::new(
        "target/release"
    )));
}

#[test]
/// Configured quality report files stay beneath the selected generated root.
fn automation_scopes_coverage_outputs_to_results() {
    let root = super::result_root().expect("generated root");
    let html =
        super::quality_output_path("coverage", "html_output").expect("configured HTML report");
    let json =
        super::quality_output_path("coverage", "json_output").expect("configured JSON report");
    assert!(html.starts_with(&root));
    assert!(json.starts_with(&root));
}

#[test]
/// Binary package staging is atomic and contains only selected release files.
fn automation_stages_the_selected_binary_package() {
    let root = std::env::temp_dir().join(format!("neutral-package-stage-{}", std::process::id()));
    let source = root.join("source");
    let output = root.join("output").join("host");
    std::fs::create_dir_all(&source).expect("temporary package source should be created");
    std::fs::write(root.join(constants::LICENSE_FILE), "license")
        .expect("temporary license should be written");
    std::fs::write(root.join(constants::ROOT_README_FILE), "readme")
        .expect("temporary README should be written");
    for binary in [constants::NEUTRAL_CLI, constants::NEUTRAL_PROBE] {
        std::fs::write(super::release_binary_path(&source, binary), binary)
            .expect("temporary binary should be written");
    }
    let binaries = vec![
        constants::NEUTRAL_CLI.to_owned(),
        constants::NEUTRAL_PROBE.to_owned(),
    ];
    let assets = vec![super::DistributionAsset {
        filename: constants::RELEASE_CHECKSUM_FILE.to_owned(),
        bytes: b"checksums".to_vec(),
    }];
    assert!(
        super::stage_binary_package(&root, &source, &output, &binaries, &assets, "{}\n").is_ok()
    );
    assert!(
        output
            .join(constants::RELEASE_PACKAGE_SUMMARY_FILE)
            .is_file()
    );
    assert!(output.join(constants::LICENSE_FILE).is_file());
    assert!(output.join(constants::ROOT_README_FILE).is_file());
    assert!(output.join(constants::RELEASE_CHECKSUM_FILE).is_file());
    assert!(
        super::stage_binary_package(&root, &source, &output, &binaries, &assets, "{}\n").is_ok()
    );
    std::fs::write(output.join(constants::RELEASE_CHECKSUM_FILE), "tampered")
        .expect("temporary package asset should be writable");
    assert!(
        super::stage_binary_package(&root, &source, &output, &binaries, &assets, "{}\n").is_err()
    );
    std::fs::write(output.join(constants::RELEASE_CHECKSUM_FILE), "checksums")
        .expect("temporary package asset should be restored");
    std::fs::write(output.join("unexpected"), "extra")
        .expect("unexpected package file should be writable");
    assert!(
        super::stage_binary_package(&root, &source, &output, &binaries, &assets, "{}\n").is_err()
    );
    std::fs::remove_file(output.join("unexpected"))
        .expect("unexpected package file should be removable");
    std::fs::remove_file(output.join(constants::LICENSE_FILE))
        .expect("temporary package license should be removable");
    assert!(
        super::stage_binary_package(&root, &source, &output, &binaries, &assets, "{}\n").is_err()
    );
    std::fs::remove_dir_all(root).expect("temporary package tree should be removable");
}

#[test]
/// Release package metadata is escaped by the serializer and follows one layout.
fn automation_uses_typed_release_package_metadata() {
    let root = std::path::Path::new("generated-results");
    let directory = super::release_metadata::package_output_directory(
        root,
        "v0.5.0",
        "candidate-commit",
        "example-host",
    );
    assert_eq!(
        directory,
        root.join(constants::RELEASE_RESULT_DIRECTORY)
            .join(constants::RELEASE_PACKAGE_DIRECTORY)
            .join("v0.5.0")
            .join("candidate-commit")
            .join("example-host")
    );

    let summary = super::release_metadata::package_summary_json(
        "v0.5.0",
        "commit\"with-quote",
        "example-host",
    )
    .expect("package summary should serialize");
    let value: serde_json::Value =
        serde_json::from_str(&summary).expect("package summary should be valid JSON");
    assert_eq!(value["candidate_commit"], "commit\"with-quote");
    assert_eq!(value["release_tag"], "v0.5.0");
    assert_eq!(value["channel"], "github-binaries");
    assert!(summary.contains("\n  \"release_tag\": "));
    assert!(summary.ends_with('\n'));
}

#[test]
/// Verifies that active-suite discovery fails when a required category is empty.
fn automation_rejects_a_zero_active_suite() {
    let minimums = BTreeMap::from([("automation".to_owned(), 1)]);
    let discovered = BTreeMap::from([("automation".to_owned(), 0)]);

    assert!(super::validate_test_minimums(&minimums, &discovered).is_err());
}

#[test]
/// Verifies every test-only production declaration requires a crate-local path module.
fn automation_rejects_inline_test_module_bodies() {
    assert!(source_has_non_path_test_configuration(
        "#[cfg(test)]\nmod private_checks {\n}\n"
    ));
    assert!(!source_has_non_path_test_configuration(
        "#[cfg(test)]\n#[path = \"../tests/unit/mod.rs\"]\nmod tests;\n"
    ));
}

#[test]
/// Verifies xtask commands run successfully.
fn xtask_commands_and_helpers() {
    assert!(super::run(["environment".into(), "manifest".into()]).is_ok());
    assert!(super::check_boundaries().is_ok());
    assert!(super::check_test_layout().is_ok());
    assert!(super::check_traceability().is_ok());
    assert!(super::check_workflow_contract().is_ok());
    assert!(super::run(["--help".into()]).is_ok());

    assert_eq!(
        super::json_string("hello\n\"world\""),
        "hello\\n\\\"world\\\""
    );
    assert!(super::quality_array("fuzz", "targets").is_ok());
    assert!(super::quality_array("fuzz", "nonexistent").is_err());
    assert!(super::quality_array("nonexistent", "nonexistent").is_err());

    let root = super::workspace_root().expect("workspace root should exist");
    assert!(root.exists());
    let res_root = super::result_root().expect("result root should exist");
    let configured =
        super::automation_value("output", "results_root").expect("configured result directory");
    assert!(res_root.ends_with(configured));

    let uniq_dir = super::unique_generated_directory(&res_root.join("unit-test-probe"))
        .expect("unique result dir");
    assert!(uniq_dir.exists());

    let text = super::read_workspace_text(&root, "Cargo.toml").expect("read workspace text");
    assert!(text.contains("workspace"));

    let mut files = Vec::new();
    assert!(super::collect_regular_files(&root.join("config"), &mut files).is_ok());
    assert_ne!(files.len(), 0);

    let bundle = super::ReleasedBundle::load(&root).expect("configured inherited bundle");
    assert!(
        bundle
            .member("specs/REQUIREMENTS.md")
            .starts_with("conformance/releases/v")
    );
    assert!(
        super::ensure_registered_paths_exist(&root, &bundle.member("specs/REQUIREMENTS.md"))
            .is_ok()
    );
    let inventory = files
        .iter()
        .map(|path| {
            path.strip_prefix(&root)
                .expect("workspace file")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(super::ensure_inventory_registered(&root, "config", &inventory).is_ok());
    assert!(super::ensure_inventory_registered(&root, "config", "").is_err());

    assert!(super::print_environment_manifest().is_ok());
    assert_eq!(super::configuration::active_test_profile(), "current");
    let min_map = super::configuration::test_minimums("current").expect("test minimums");
    assert_ne!(min_map.len(), 0);
}

#[test]
/// Conformance selection rejects traversal, absent bundles, and unknown schemas.
fn inherited_conformance_selection_is_bounded_and_explicit() {
    let root = super::workspace_root().expect("workspace root");
    for configuration in [
        "schema_version = 2\ninherited_release = \"v0.1.0\"\n",
        "schema_version = 1\ninherited_release = \"../v0.1.0\"\n",
        "schema_version = 1\ninherited_release = \"v99.0.0\"\n",
    ] {
        assert!(
            super::ReleasedBundle::from_configuration(&root, configuration).is_err(),
            "invalid release selection must fail: {configuration}"
        );
    }
    assert!(
        super::ReleasedBundle::from_configuration(
            &root,
            "schema_version = 1\ninherited_release = \"v0.1.0\"\n"
        )
        .is_ok()
    );
}
