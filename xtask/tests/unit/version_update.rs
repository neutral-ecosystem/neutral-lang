// SPDX-License-Identifier: Apache-2.0

//! Release metadata failure and interruption recovery checks in disposable roots.

use super::*;

/// Later write failures leave the original manifest intact.
#[test]
fn version_update_does_not_leave_partial_metadata() {
    let root = std::env::temp_dir().join(format!("neutral-version-failure-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Cargo.toml"), "old manifest").unwrap();
    fs::create_dir_all(root.join("Cargo.lock")).unwrap();
    assert!(
        apply(
            &root,
            &[
                ("Cargo.toml".to_owned(), b"new manifest".to_vec()),
                ("Cargo.lock".to_owned(), b"new lock".to_vec())
            ]
        )
        .is_err()
    );
    assert_eq!(fs::read(root.join("Cargo.toml")).unwrap(), b"old manifest");
    fs::remove_dir_all(root).unwrap();
}

/// Ordinary failures after any successful replacement roll back all three owned files.
#[test]
fn version_write_failures_roll_back_after_installation_starts() {
    for failing in 0..3 {
        let root = std::env::temp_dir().join(format!(
            "neutral-version-write-{}-{failing}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Cargo.toml"), "old manifest").unwrap();
        fs::write(root.join("Cargo.lock"), "old lock").unwrap();
        let changes = [
            ("Cargo.toml".to_owned(), b"new manifest".to_vec()),
            ("Cargo.lock".to_owned(), b"new lock".to_vec()),
            (
                "quality/evidence/v1.2.3/README.md".to_owned(),
                b"new evidence".to_vec(),
            ),
        ];
        let mut index = 0;
        let error = apply_with(&root, &changes, |root, destination, bytes| {
            if index == failing {
                return Err("injected write failure".to_owned());
            }
            index += 1;
            replace(root, destination, bytes)
        })
        .unwrap_err();
        assert!(error.contains("rolled back"));
        assert_eq!(fs::read(root.join("Cargo.toml")).unwrap(), b"old manifest");
        assert_eq!(fs::read(root.join("Cargo.lock")).unwrap(), b"old lock");
        assert!(!root.join(&changes[2].0).exists());
        assert_eq!(recover(&root).unwrap(), None);
        fs::remove_dir_all(root).unwrap();
    }
}

/// Ignored staging state must never redirect writes outside the recovery store.
#[cfg(unix)]
#[test]
fn version_update_rejects_staging_symlinks() {
    let root = std::env::temp_dir().join(format!("neutral-version-symlink-{}", std::process::id()));
    fs::create_dir_all(root.join(STORE)).unwrap();
    let outside = root.join("unrelated");
    fs::write(&outside, "preserve").unwrap();
    std::os::unix::fs::symlink(&outside, root.join(STORE).join("replacement")).unwrap();
    assert!(apply(&root, &[("Cargo.toml".to_owned(), b"manifest".to_vec())]).is_err());
    assert_eq!(fs::read(outside).unwrap(), b"preserve");
    fs::remove_dir_all(root).unwrap();
}

/// Every interrupted replacement boundary recovers originals before retry.
#[test]
fn version_recovery_restores_each_interrupted_boundary() {
    for applied in 0..=3 {
        let root = std::env::temp_dir().join(format!(
            "neutral-version-recovery-{}-{applied}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Cargo.toml"), "manifest before").unwrap();
        fs::write(root.join("Cargo.lock"), "lock before").unwrap();
        let journal = Journal {
            schema_version: 1,
            complete: false,
            entries: vec![
                Replacement {
                    path: "Cargo.toml".into(),
                    old: Some(b"manifest before".to_vec()),
                    new: b"manifest after".to_vec(),
                },
                Replacement {
                    path: "Cargo.lock".into(),
                    old: Some(b"lock before".to_vec()),
                    new: b"lock after".to_vec(),
                },
                Replacement {
                    path: "quality/evidence/v1.2.3/README.md".into(),
                    old: None,
                    new: b"new evidence".to_vec(),
                },
            ],
        };
        let lock = lock(&root).unwrap();
        write_journal(&root, &journal).unwrap();
        for entry in journal.entries.iter().take(applied) {
            replace(&root, &root.join(&entry.path), &entry.new).unwrap();
        }
        drop(lock);
        assert_eq!(recover(&root).unwrap(), None);
        assert_eq!(
            fs::read(root.join("Cargo.toml")).unwrap(),
            b"manifest before"
        );
        assert_eq!(fs::read(root.join("Cargo.lock")).unwrap(), b"lock before");
        assert!(!root.join("quality/evidence/v1.2.3/README.md").exists());
        assert_eq!(recover(&root).unwrap(), None);
        let changes = journal
            .entries
            .iter()
            .map(|entry| (entry.path.clone(), entry.new.clone()))
            .collect::<Vec<_>>();
        apply(&root, &changes).unwrap();
        assert_eq!(recover(&root).unwrap().unwrap().len(), 3);
        acknowledge(&root).unwrap();
        assert_eq!(
            fs::read(root.join("Cargo.toml")).unwrap(),
            b"manifest after"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

/// Recovery never overwrites intervening edits, including when earlier paths still match.
#[test]
fn version_recovery_refuses_user_edits_and_unsafe_journals() {
    let root = std::env::temp_dir().join(format!("neutral-version-edits-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let guard = lock(&root).unwrap();
    let mut journal = Journal {
        schema_version: 1,
        complete: false,
        entries: vec![
            Replacement {
                path: "Cargo.toml".into(),
                old: Some(b"old".to_vec()),
                new: b"new".to_vec(),
            },
            Replacement {
                path: "Cargo.lock".into(),
                old: Some(b"old lock".to_vec()),
                new: b"new lock".to_vec(),
            },
        ],
    };
    fs::write(root.join("Cargo.toml"), "new").unwrap();
    fs::write(root.join("Cargo.lock"), "user edit").unwrap();
    write_journal(&root, &journal).unwrap();
    drop(guard);
    assert!(recover(&root).is_err());
    assert_eq!(fs::read(root.join("Cargo.toml")).unwrap(), b"new");
    assert_eq!(fs::read(root.join("Cargo.lock")).unwrap(), b"user edit");
    for path in [
        "../outside",
        "README.md",
        "/outside",
        "quality/evidence/not-version/README.md",
    ] {
        journal.entries[0].path = path.into();
        write_journal(&root, &journal).unwrap();
        assert!(recover(&root).is_err());
    }
    fs::write(root.join(STORE).join("journal.json"), "{}").unwrap();
    assert!(recover(&root).is_err());
    fs::remove_dir_all(root).unwrap();
}
