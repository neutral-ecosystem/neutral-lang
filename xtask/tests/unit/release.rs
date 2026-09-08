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
            "candidate_tag = \"v0.1.0\"\n",
            "stage9_residual_risk = true\n",
            "source_tag = true\n",
            "github_binaries = true\n",
            "crates_io = false\n",
            "binaries = [\"neutral-cli\", \"neutral-probe\"]\n",
        ),
    )
    .expect("temporary release plan should be writable");
    let plan = ReleasePlan::read(&path).expect("explicit release plan should parse");
    assert!(plan.channels.contains(&DistributionChannel::SourceTag));
    assert!(plan.channels.contains(&DistributionChannel::GithubBinaries));
    assert!(!plan.channels.contains(&DistributionChannel::CratesIo));
    fs::remove_file(path).expect("temporary release plan should be removable");
}

#[test]
/// Missing approval and distribution selections fail closed.
fn release_plan_rejects_unapproved_empty_scope() {
    let path = temporary_plan_path("rejected");
    fs::write(
        &path,
        concat!(
            "candidate_tag = \"v0.1.0\"\n",
            "stage9_residual_risk = false\n",
            "source_tag = false\n",
            "github_binaries = false\n",
            "crates_io = false\n",
            "binaries = []\n",
        ),
    )
    .expect("temporary release plan should be writable");
    assert!(ReleasePlan::read(&path).is_err());
    fs::remove_file(path).expect("temporary release plan should be removable");
}

/// Returns a process-unique temporary release-plan path.
fn temporary_plan_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "neutral-release-plan-{label}-{}.toml",
        std::process::id()
    ))
}
