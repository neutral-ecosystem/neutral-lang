// SPDX-License-Identifier: Apache-2.0

//! Focused checks for small host helpers using owned temporary inputs.

/// JSON escaping preserves ordinary text and escapes structural/control characters.
#[test]
fn json_strings_escape_host_text() {
    for (source, expected) in [
        ("plain", "plain"),
        ("", ""),
        ("a\nb", "a\\nb"),
        ("\"\\", "\\\"\\\\"),
    ] {
        assert_eq!(crate::json_string(source), expected);
    }
}

/// Generated-directory allocation owns distinct directories and file helpers read their actual contents.
#[test]
fn filesystem_helpers_keep_generated_directories_distinct() {
    let parent = neutral_test_support::TestDirectory::new().unwrap();
    let first = crate::unique_generated_directory(parent.path()).unwrap();
    let second = crate::unique_generated_directory(parent.path()).unwrap();
    assert_ne!(first, second);
    assert!(first.is_dir() && second.is_dir());
    std::fs::write(first.join("input.txt"), "exact contents").unwrap();
    assert_eq!(
        crate::read_workspace_text(&first, "input.txt").unwrap(),
        "exact contents"
    );
    let mut files = Vec::new();
    crate::collect_regular_files(parent.path(), &mut files).unwrap();
    assert_eq!(files, [first.join("input.txt")]);
}

/// Last committed evidence may link to an artifact that was later retired.
#[test]
fn historical_evidence_links_require_the_committed_target_blob() {
    let (root, _directory) = historical_link_workspace("[artifact](../../../crates/retired.rs)");
    std::fs::create_dir_all(root.join("crates")).unwrap();
    std::fs::write(root.join("crates/retired.rs"), "historical target").unwrap();
    commit_fixture(&root, "record target");
    std::fs::remove_file(root.join("crates/retired.rs")).unwrap();
    commit_fixture(&root, "retire target");

    assert!(
        crate::runtime::files::markdown_link_exists(
            &root,
            &std::fs::canonicalize(&root).unwrap(),
            &root.join("portable/development/evidence/record.md"),
            "../../../crates/retired.rs",
        )
        .unwrap()
    );
}

/// Editing an evidence record disables historical link resolution.
#[test]
fn changed_evidence_records_cannot_use_historical_links() {
    let (root, _directory) = historical_link_workspace("[artifact](../../../crates/retired.rs)");
    std::fs::create_dir_all(root.join("crates")).unwrap();
    std::fs::write(root.join("crates/retired.rs"), "historical target").unwrap();
    commit_fixture(&root, "record target");
    std::fs::remove_file(root.join("crates/retired.rs")).unwrap();
    commit_fixture(&root, "retire target");
    std::fs::write(
        root.join("portable/development/evidence/record.md"),
        "[changed](../../../crates/retired.rs)",
    )
    .unwrap();

    assert!(
        !crate::runtime::files::markdown_link_exists(
            &root,
            &std::fs::canonicalize(&root).unwrap(),
            &root.join("portable/development/evidence/record.md"),
            "../../../crates/retired.rs",
        )
        .unwrap()
    );
}

/// An unchanged record cannot make a target valid when the target never existed.
#[test]
fn missing_historical_target_blob_is_rejected() {
    let (root, _directory) = historical_link_workspace("[missing](../../../crates/missing.rs)");
    commit_fixture(&root, "record missing target");

    assert!(
        !crate::runtime::files::markdown_link_exists(
            &root,
            &std::fs::canonicalize(&root).unwrap(),
            &root.join("portable/development/evidence/record.md"),
            "../../../crates/missing.rs",
        )
        .unwrap()
    );
}

/// Historical target normalization rejects workspace escape and non-evidence sources.
#[test]
fn historical_link_fallback_rejects_unsafe_paths_and_other_documents() {
    let (root, _directory) = historical_link_workspace("[escape](../../../../../../etc/passwd)");
    commit_fixture(&root, "record escape");
    let canonical = std::fs::canonicalize(&root).unwrap();
    let evidence = root.join("portable/development/evidence/record.md");
    assert!(
        !crate::runtime::files::markdown_link_exists(
            &root,
            &canonical,
            &evidence,
            "../../../../../../etc/passwd",
        )
        .unwrap()
    );
    assert!(
        !crate::runtime::files::markdown_link_exists(
            &root,
            &canonical,
            &root.join("portable/development/record.md"),
            "../../../crates/retired.rs",
        )
        .unwrap()
    );
}

/// Historical proof fails explicitly when Git cannot guarantee full history.
#[test]
fn historical_link_fallback_fails_closed_in_shallow_checkouts() {
    let (root, _directory) = historical_link_workspace("[artifact](../../../crates/retired.rs)");
    std::fs::create_dir_all(root.join("crates")).unwrap();
    std::fs::write(root.join("crates/retired.rs"), "historical target").unwrap();
    commit_fixture(&root, "record target");
    std::fs::remove_file(root.join("crates/retired.rs")).unwrap();
    commit_fixture(&root, "retire target");
    let head = std::process::Command::new("git")
        .current_dir(&root)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(head.status.success());
    std::fs::write(root.join(".git/shallow"), head.stdout).unwrap();

    let error = crate::runtime::files::markdown_link_exists(
        &root,
        &std::fs::canonicalize(&root).unwrap(),
        &root.join("portable/development/evidence/record.md"),
        "../../../crates/retired.rs",
    )
    .unwrap_err();
    assert!(error.contains("checkout is shallow"), "{error}");
}

fn historical_link_workspace(
    link: &str,
) -> (std::path::PathBuf, neutral_test_support::TestDirectory) {
    let directory = neutral_test_support::TestDirectory::new().unwrap();
    let root = directory.path().to_path_buf();
    let evidence = root.join("portable/development/evidence");
    std::fs::create_dir_all(&evidence).unwrap();
    std::fs::write(evidence.join("record.md"), link).unwrap();
    let output = std::process::Command::new("git")
        .current_dir(&root)
        .args(["init", "--quiet", "--initial-branch=main"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let output = std::process::Command::new("git")
        .current_dir(&root)
        .args(["config", "commit.gpgsign", "false"])
        .output()
        .unwrap();
    assert!(output.status.success());
    (root, directory)
}

fn commit_fixture(root: &std::path::Path, message: &str) {
    for arguments in [
        vec!["add", "--all"],
        vec!["commit", "--quiet", "-m", message],
    ] {
        let mut command = std::process::Command::new("git");
        command
            .current_dir(root)
            .env("GIT_AUTHOR_NAME", "xtask fixture")
            .env("GIT_AUTHOR_EMAIL", "xtask@example.invalid")
            .env("GIT_COMMITTER_NAME", "xtask fixture")
            .env("GIT_COMMITTER_EMAIL", "xtask@example.invalid")
            .args(arguments);
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "git fixture command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
