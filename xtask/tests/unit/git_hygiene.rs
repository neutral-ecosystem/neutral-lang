// SPDX-License-Identifier: Apache-2.0

//! Tracking and ignore behavior, including nested and loose fuzz output.

use super::*;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

/// Runs Git against only the isolated fixture repository, never the user's index.
fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new(constants::GIT_COMMAND)
            .current_dir(root)
            .args(args)
            .status()
            .unwrap()
            .success()
    );
}

/// Repository ignores cover generated state without hiding seeds, lockfiles, or retained approvals.
#[test]
fn ignore_policy_preserves_inputs_and_blocks_generated_output() {
    let root = crate::workspace_root().unwrap();
    for (path, ignored) in [
        ("fuzz/corpus/loose-generated-input", true),
        ("fuzz/corpus/source/generated-input", true),
        ("fuzz/artifacts/source/crash-input", true),
        ("fuzz/coverage/source/coverage.profdata", true),
        ("fuzz/target/debug/source", true),
        ("fuzz/crash-input", true),
        ("timeout-input", true),
        (".env.local", true),
        ("fuzz/corpus/README.md", false),
        ("fuzz/Cargo.lock", false),
        ("fuzz/fuzz_targets/source.rs", false),
        ("fuzz/seeds/vocabulary/project-v1-public.json", false),
        (
            "quality/evidence/release/gates/input/performance-release/massif.out",
            true,
        ),
        ("quality/evidence/release/record.toml", false),
        (".env.example", false),
    ] {
        let status = Command::new(constants::GIT_COMMAND)
            .current_dir(&root)
            .args(["check-ignore", "--quiet", "--no-index", "--", path])
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(i32::from(!ignored)), "{path}");
    }
}

/// CI detects force-added ignored files, and removing only their tracking preserves local data.
#[test]
fn tracking_guard_rejects_force_added_ignored_files() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "neutral-ignore-test-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "--quiet"]);
    fs::write(root.join(".gitignore"), "generated/\n").unwrap();
    git(&root, &["add", ".gitignore"]);
    assert!(verify_no_ignored_tracked_files(&root).is_ok());
    fs::create_dir(root.join("generated")).unwrap();
    fs::write(root.join("generated/finding"), "local finding").unwrap();
    git(&root, &["add", "--force", "generated/finding"]);
    assert!(
        verify_no_ignored_tracked_files(&root)
            .unwrap_err()
            .contains("generated/finding")
    );
    git(&root, &["rm", "--cached", "--quiet", "generated/finding"]);
    assert!(verify_no_ignored_tracked_files(&root).is_ok());
    assert!(root.join("generated/finding").is_file());
    fs::remove_dir_all(root).unwrap();
}
