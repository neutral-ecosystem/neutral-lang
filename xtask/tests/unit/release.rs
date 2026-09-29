// SPDX-License-Identifier: Apache-2.0

//! Unit tests for fail-closed release-selection parsing.

use super::{DistributionChannel, ReleasePlan};
use std::fs;

#[test]
/// A complete explicit distribution selection is accepted.
fn release_plan_accepts_an_explicit_scope() {
    let path = temporary_plan_path("accepted");
    fs::write(
        &path,
        concat!(
            "schema_version = 1\n[distribution]\n",
            "source_tag = true\n",
            "github_binaries = true\n",
            "crates_io = false\n",
            "binaries = [\"neutral-cli\", \"neutral-probe\"]\n",
        ),
    )
    .expect("temporary release plan should be writable");
    let version = env!("CARGO_PKG_VERSION");
    let plan = ReleasePlan::read(&path, version).expect("explicit release plan should parse");
    assert_eq!(plan.release_tag, format!("v{version}"));
    assert!(plan.channels.contains(&DistributionChannel::SourceTag));
    assert!(plan.channels.contains(&DistributionChannel::GithubBinaries));
    assert!(!plan.channels.contains(&DistributionChannel::CratesIo));
    fs::remove_file(path).expect("temporary release plan should be removable");
}

#[test]
/// An empty distribution selection fails closed.
fn release_plan_rejects_empty_scope() {
    let path = temporary_plan_path("rejected");
    fs::write(
        &path,
        concat!(
            "schema_version = 1\n[distribution]\n",
            "source_tag = false\n",
            "github_binaries = false\n",
            "crates_io = false\n",
            "binaries = []\n",
        ),
    )
    .expect("temporary release plan should be writable");
    assert!(ReleasePlan::read(&path, "0.1.0").is_err());
    fs::remove_file(path).expect("temporary release plan should be removable");
}

#[test]
/// Binary publication cannot bypass the selected immutable source-tag channel.
fn release_plan_rejects_binaries_without_a_source_tag() {
    let path = temporary_plan_path("binary-without-source");
    fs::write(
        &path,
        concat!(
            "schema_version = 1\n[distribution]\n",
            "source_tag = false\n",
            "github_binaries = true\n",
            "crates_io = false\n",
            "binaries = [\"neutral-cli\", \"neutral-probe\"]\n",
        ),
    )
    .expect("temporary release plan should be writable");
    assert!(ReleasePlan::read(&path, "0.1.0").is_err());
    fs::remove_file(path).expect("temporary release plan should be removable");
}

#[test]
/// Duplicate or path-like binary selections cannot become release filenames.
fn release_plan_rejects_unsafe_binary_names() {
    let path = temporary_plan_path("unsafe-binary");
    for binaries in ["[\"neutral-cli\", \"neutral-cli\"]", "[\"../escape\"]"] {
        fs::write(
            &path,
            format!(
                "schema_version = 1\n[distribution]\nsource_tag = true\ngithub_binaries = true\ncrates_io = false\nbinaries = {binaries}\n"
            ),
        )
        .expect("temporary release plan should be writable");
        assert!(ReleasePlan::read(&path, env!("CARGO_PKG_VERSION")).is_err());
    }
    fs::remove_file(path).expect("temporary release plan should be removable");
}

#[test]
/// A distribution block cannot silently inherit values from another TOML section.
fn release_plan_requires_the_selected_schema_and_section() {
    let path = temporary_plan_path("wrong-section");
    fs::write(
        &path,
        "schema_version = 1\n[unrelated]\nsource_tag = true\ngithub_binaries = true\ncrates_io = false\nbinaries = [\"neutral-cli\"]\n",
    )
    .expect("temporary release plan should be writable");
    assert!(ReleasePlan::read(&path, env!("CARGO_PKG_VERSION")).is_err());
    fs::remove_file(path).expect("temporary release plan should be removable");
}

/// Returns a process-unique temporary release-plan path.
fn temporary_plan_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "neutral-release-plan-{label}-{}.toml",
        std::process::id()
    ))
}
