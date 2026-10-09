// SPDX-License-Identifier: Apache-2.0

//! Scoped OS-lock ownership must not depend on transient descriptor copies.

use super::*;

/// Fork/dup copies may outlive the owner briefly; releasing ownership must still unlock.
#[test]
fn releasing_owner_unlocks_duplicate_handles() {
    let root = std::env::temp_dir().join(format!("neutral-file-lock-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("lock");
    #[cfg(unix)]
    {
        // Reproduce why closing only the owner's descriptor is insufficient after dup/fork.
        let raw = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .unwrap();
        raw.try_lock().unwrap();
        let inherited = raw.try_clone().unwrap();
        drop(raw);
        assert!(ExclusiveFileLock::claim(&path).is_err());
        inherited.unlock().unwrap();
    }
    let guard = ExclusiveFileLock::claim(&path).unwrap();
    let duplicate = guard.file.try_clone().unwrap();
    assert!(ExclusiveFileLock::claim(&path).is_err());
    drop(guard);
    let replacement = ExclusiveFileLock::claim(&path).unwrap();
    drop(replacement);
    drop(duplicate);
    fs::remove_dir_all(root).unwrap();
}
