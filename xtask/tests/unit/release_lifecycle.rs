// SPDX-License-Identifier: Apache-2.0

//! Release safety tests use isolated repositories, never the developer's real refs.

use super::*;
use crate::{PathBuf, Task};

/// Creates a local main repository and bare origin with test-local identity.
fn repository(label: &str) -> PathBuf {
    let root = env::temp_dir().join(format!(
        "neutral-release-lifecycle-{label}-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let origin = root.join("origin.git");
    git(&root, &["init", "--bare", origin.to_str().unwrap()]).unwrap();
    git(&root, &["init", "--initial-branch=main"]).unwrap();
    git(&root, &["config", "user.name", "Neutral release test"]).unwrap();
    git(&root, &["config", "user.email", "release@example.invalid"]).unwrap();
    git(&root, &["config", "commit.gpgsign", "false"]).unwrap();
    git(
        &root,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    )
    .unwrap();
    fs::write(root.join("source"), "first").unwrap();
    commit_paths(&root, &["source"], "initial").unwrap();
    root
}

/// Successful publication sends exactly main and its selected tag, not other local tags.
#[test]
fn atomic_publication_sends_branch_and_selected_tag_together() {
    let root = repository("atomic-success");
    let tag = format!("v{}", env!("CARGO_PKG_VERSION"));
    git(&root, &["tag", &tag]).unwrap();
    git(&root, &["tag", "unrelated-local-tag"]).unwrap();
    let args = atomic_push_arguments(&tag);
    git(&root, &args.iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
    let head = git(&root, &["rev-parse", "HEAD"]).unwrap();
    let remote = git(&root, &["ls-remote", "origin"]).unwrap();
    assert!(remote.contains(&format!("{head}\trefs/heads/main")));
    assert!(remote.contains(&format!("{head}\trefs/tags/{tag}")));
    assert!(!remote.contains("unrelated-local-tag"));
    assert!(
        require_remote_tag_available(&root, &tag)
            .unwrap_err()
            .contains("immutable")
    );
    fs::remove_dir_all(root).unwrap();
}

/// A rejected tag prevents the same push from advancing main; no force fallback is used.
#[test]
fn atomic_publication_rejects_conflicts_without_partial_branch_push() {
    let root = repository("atomic-conflict");
    git(&root, &["tag", "reserved"]).unwrap();
    git(&root, &["push", "origin", "main", "refs/tags/reserved"]).unwrap();
    let old = git(&root, &["rev-parse", "HEAD"]).unwrap();
    fs::write(root.join("source"), "second").unwrap();
    commit_paths(&root, &["source"], "second").unwrap();
    // The differing local ref is created under another name; the remote tag is never changed.
    git(&root, &["tag", "new-local-tag"]).unwrap();
    let args = [
        "push",
        "--atomic",
        "origin",
        "main:main",
        "refs/tags/new-local-tag:refs/tags/reserved",
    ];
    assert!(git(&root, &args).is_err());
    let remote = git(&root, &["ls-remote", "origin", "refs/heads/main"]).unwrap();
    assert_eq!(remote, format!("{old}\trefs/heads/main"));
    fs::remove_dir_all(root).unwrap();
}

/// Local tag conflicts and remote transport errors are distinct from an available release name.
#[test]
fn tag_checks_fail_closed_and_metadata_commits_are_repeatable() {
    let root = repository("tag-checks");
    require_local_tag_available(&root, "available").unwrap();
    require_remote_tag_available(&root, "available").unwrap();
    let head = git(&root, &["rev-parse", "HEAD"]).unwrap();
    commit_paths(&root, &["source"], "unchanged").unwrap();
    assert_eq!(git(&root, &["rev-parse", "HEAD"]).unwrap(), head);
    git(&root, &["tag", "taken"]).unwrap();
    assert!(require_local_tag_available(&root, "taken").is_err());
    require_tag_commit(&root, "taken", &head).unwrap();
    assert!(require_tag_commit(&root, "taken", "different-commit").is_err());
    git(
        &root,
        &["remote", "set-url", "origin", "missing-local-repository"],
    )
    .unwrap();
    assert!(
        require_remote_tag_available(&root, "available")
            .unwrap_err()
            .contains("could not check")
    );
    fs::remove_dir_all(root).unwrap();
}

/// Porcelain's leading status column must survive capture for exact recovery path checks.
#[test]
fn recovery_preserves_porcelain_status_columns() {
    let root = repository("recovery-status");
    fs::write(root.join("source"), "changed").unwrap();
    let status = git(&root, &["status", "--porcelain", "--", "source"]).unwrap();
    assert_eq!(status, " M source");
    assert_eq!(status.get(3..), Some("source"));
    fs::remove_dir_all(root).unwrap();
}

/// Completed version preparation can commit its new scaffold, but never other edits.
#[test]
fn recovery_accepts_new_scaffold_and_rejects_unrelated_changes() {
    let root = repository("recovery-scaffold");
    fs::write(root.join(".git/info/exclude"), "origin.git/\n").unwrap();
    let readme = "quality/evidence/v1.2.3/README.md";
    let changes = vec![(readme.to_owned(), b"evidence".to_vec())];
    super::super::version_update::apply(&root, &changes).unwrap();
    // The real repository ignores recovery data independently of generated results.
    fs::write(
        root.join(".git/info/exclude"),
        "origin.git/\n.neutral-version-update/\n",
    )
    .unwrap();
    let paths = super::super::version_update::recover(&root)
        .unwrap()
        .unwrap();
    require_only_metadata_changes(&root, &paths).unwrap();
    fs::write(root.join("source"), "user edit").unwrap();
    assert!(require_only_metadata_changes(&root, &paths).is_err());
    fs::write(root.join("source"), "first").unwrap();
    require_only_metadata_changes(&root, &paths).unwrap();
    commit_paths(&root, &[readme], "metadata").unwrap();
    super::super::version_update::acknowledge(&root).unwrap();
    assert!(git(&root, &["status", "--porcelain"]).unwrap().is_empty());
    fs::remove_dir_all(root).unwrap();
}

/// Every required gate delegates to the existing measurement family and correct toolchain.
#[test]
fn release_measurement_mapping_covers_all_required_gates() {
    for gate in QualityGate::RELEASE_REQUIRED {
        let arguments = measurement_arguments(gate);
        if gate == QualityGate::Advisories {
            assert_eq!(arguments, [] as [&str; 0]);
        } else {
            let task = crate::interface::parse(
                &arguments
                    .iter()
                    .map(|arg| (*arg).to_owned())
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            assert!(matches!(
                task,
                Task::Coverage | Task::Mutate | Task::Performance(_)
            ));
        }
    }
    assert!(QualityGate::Coverage.requires_nightly());
    assert!(QualityGate::Fuzz.requires_nightly());
    assert!(!QualityGate::Mutation.requires_nightly());
}
