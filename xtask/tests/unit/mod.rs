// SPDX-License-Identifier: Apache-2.0

//! Tests for dependency-boundary policy failures.

use super::{
    constants, contract_ids, ensure_ids_covered, ensure_syntax_complete, quality_value_from,
    render_rustdoc_index, rustdoc_header_configuration, set,
    source_has_non_path_test_configuration, validate_allowed_packages,
    validate_direct_dependencies,
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
    ] {
        assert!(super::run(arguments.into_iter().map(str::to_owned)).is_err());
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
    assert!(super::test_minimums("missing").is_err());
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
/// Verifies Stage 9 tool arguments come from the quality-gate configuration.
fn automation_reads_named_quality_values() {
    let configuration =
        "[coverage]\nminimum_line_percent = 85\n\n[fuzz]\ntargets = [\"source\", \"ir\"]\n";
    assert_eq!(
        quality_value_from(configuration, "coverage", "minimum_line_percent"),
        Some("85".to_owned())
    );
    assert_eq!(
        quality_value_from(configuration, "fuzz", "targets"),
        Some("[\"source\", \"ir\"]".to_owned())
    );
}

#[test]
/// Verifies that the environment manifest identifies the selected toolchain channel.
fn environment_manifest_identifies_the_toolchain_channel() {
    let manifest = super::environment_manifest().expect("environment manifest should be available");
    let channel = super::rust_channel().expect("toolchain channel should be readable");
    let stage = super::active_stage().expect("active stage should be readable");
    assert!(manifest.contains(&format!("\"rust_channel\": \"{channel}\"")));
    assert!(manifest.contains(&format!("\"active_stage\": {stage}")));
}

#[test]
/// Verifies the stable channel excludes prerelease compiler identities.
fn stable_toolchain_channel_rejects_prereleases() {
    assert!(super::rust_version_matches_channel(
        "rustc 1.98.1 (stable-hash 2026-09-03)",
        "stable",
    ));
    assert!(!super::rust_version_matches_channel(
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
}

#[test]
/// Verifies that the shared rustdoc header contributes a stable cache token.
fn automation_tracks_the_rustdoc_header_content() {
    let configuration = rustdoc_header_configuration();
    assert!(configuration.starts_with(constants::RUSTDOC_HEADER_CFG_PREFIX));
    assert_eq!(configuration, rustdoc_header_configuration());
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
/// Verifies unchecked syntax items fail the completed Stage 8 gate.
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
    assert!(super::is_safe_result_path(std::path::Path::new(
        "test-results"
    )));
    assert!(!super::is_safe_result_path(std::path::Path::new(".")));
    assert!(!super::is_safe_result_path(std::path::Path::new(
        "../test-results"
    )));
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
    assert!(super::run(["boundary".into(), "check".into()]).is_ok());
    assert!(super::run(["test-layout".into(), "check".into()]).is_ok());
    assert!(super::run(["traceability".into(), "check".into()]).is_ok());
    assert!(super::run(["boundary".into(), "help".into()]).is_ok());

    assert_eq!(
        super::json_string("hello\n\"world\""),
        "hello\n\\\"world\\\""
    );
    assert!(super::quality_array("fuzz", "targets").is_ok());
    assert!(super::quality_array("fuzz", "nonexistent").is_err());
    assert!(super::quality_array("nonexistent", "nonexistent").is_err());

    let root = super::workspace_root().expect("workspace root should exist");
    assert!(root.exists());
    let res_root = super::result_root().expect("result root should exist");
    assert!(res_root.ends_with("test-results"));

    let uniq_dir = super::unique_result_directory("unit_test_probe").expect("unique result dir");
    assert!(uniq_dir.exists());

    let text = super::read_workspace_text(&root, "Cargo.toml").expect("read workspace text");
    assert!(text.contains("workspace"));

    let mut files = Vec::new();
    assert!(super::collect_regular_files(&root.join("config"), &mut files).is_ok());
    assert!(!files.is_empty());

    assert!(super::ensure_registered_paths_exist(&root, "portable/specs/REQUIREMENTS.md").is_ok());
    assert!(super::ensure_inventory_registered(&root, "config", "config/development-stage.toml config/host-policy.toml config/ir-encoding.toml config/quality-gates.toml config/test-suites.toml").is_ok());

    assert!(super::print_environment_manifest().is_ok());
    assert!(super::active_test_profile().is_ok());
    let min_map = super::test_minimums("stage9").expect("test minimums");
    assert!(!min_map.is_empty());
}
