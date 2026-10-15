// SPDX-License-Identifier: Apache-2.0

//! Development state reuse must be conservative and remain separate from release evidence.

use super::*;

/// Builds a campaign without storing any repository source contents.
fn campaign() -> Campaign {
    Campaign {
        schema: CACHE_SCHEMA,
        context: "tools-and-options".to_owned(),
        inputs: BTreeMap::from([
            ("crate/src/lib.rs".to_owned(), "source".to_owned()),
            ("crate/tests/regression.rs".to_owned(), "test".to_owned()),
        ]),
        directory: "run-fixture".to_owned(),
        passed_reports: None,
    }
}

/// Creates a distinct ignored temporary fixture for one test.
fn fixture(label: &str) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("neutral-mutation-{label}-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    crate::unique_generated_directory(&root).unwrap()
}

/// Retry uses existing catches only when changes cannot weaken an existing test.
#[test]
fn unchanged_inputs_and_added_test_files_can_resume() {
    let previous = campaign();
    assert!(reusable(&previous, &previous.inputs, &previous.context));
    let mut inputs = previous.inputs.clone();
    inputs.insert("crate/tests/new_case.rs".to_owned(), "new".to_owned());
    assert!(reusable(&previous, &inputs, &previous.context));
    // Source edits, test edits/deletions and newly added non-test inputs must start fresh.
    for path in previous.inputs.keys() {
        let mut changed = previous.inputs.clone();
        changed.insert(path.clone(), "changed".to_owned());
        assert!(!reusable(&previous, &changed, &previous.context));
        changed.remove(path);
        assert!(!reusable(&previous, &changed, &previous.context));
    }
    for path in [
        "Cargo.lock",
        "config/nextest.toml",
        "crate/src/new.rs",
        "tests/data.json",
    ] {
        let mut changed = previous.inputs.clone();
        changed.insert(path.to_owned(), "new".to_owned());
        assert!(!reusable(&previous, &changed, &previous.context), "{path}");
    }
    assert!(!reusable(
        &previous,
        &previous.inputs,
        "different-tools-or-options"
    ));
    let mut old = campaign();
    old.schema += 1;
    assert!(!reusable(&old, &old.inputs, &old.context));
}

/// Both interrupted and complete state survive restart without publishing a receipt.
#[test]
fn state_round_trip_preserves_interrupted_and_completed_campaigns() {
    let cache = fixture("state");
    assert!(read_campaign(&cache).unwrap().is_none());
    let mut previous = campaign();
    fs::create_dir(cache.join(&previous.directory)).unwrap();
    save_campaign(&cache, &previous).unwrap();
    let loaded = read_campaign(&cache).unwrap().unwrap();
    assert_eq!(loaded.inputs, previous.inputs);
    assert!(loaded.passed_reports.is_none());
    previous.passed_reports = Some(BTreeMap::from([(
        "outcomes.json".to_owned(),
        "hash".to_owned(),
    )]));
    save_campaign(&cache, &previous).unwrap();
    assert_eq!(
        read_campaign(&cache).unwrap().unwrap().passed_reports,
        previous.passed_reports
    );
    assert!(!cache.join("receipt.json").exists());
    assert!(!cache.join("state.pending").exists());
    fs::write(cache.join(STATE_FILE), "interrupted old state").unwrap();
    assert!(read_campaign(&cache).unwrap().is_none());
    fs::remove_dir_all(cache).unwrap();
}

/// Altered native reports cannot turn an old pass into an immediate cache hit.
#[test]
fn completion_binds_native_iteration_lists_and_outcomes() {
    let directory = fixture("reports");
    let native = directory.join(NATIVE_OUTPUT);
    fs::create_dir(&native).unwrap();
    assert!(native_reports(&directory).is_err());
    fs::write(native.join("outcomes.json"), r#"{"caught":1}"#).unwrap();
    fs::write(native.join("caught.txt"), "caught mutant").unwrap();
    let mut previous = campaign();
    assert!(sealed_reports_match(&previous, &directory));
    previous.passed_reports = Some(native_reports(&directory).unwrap());
    assert!(sealed_reports_match(&previous, &directory));
    fs::write(native.join("caught.txt"), "altered skip list").unwrap();
    assert!(!sealed_reports_match(&previous, &directory));
    fs::write(native.join("caught.txt"), "caught mutant").unwrap();
    fs::write(native.join("previously_caught.txt"), "additional skip").unwrap();
    assert!(!sealed_reports_match(&previous, &directory));
    fs::remove_file(native.join("previously_caught.txt")).unwrap();
    fs::remove_file(native.join("outcomes.json")).unwrap();
    assert!(!sealed_reports_match(&previous, &directory));
    fs::remove_dir_all(directory).unwrap();
}

/// Cargo-mutants intentionally omits outcomes and baseline when iteration filters every mutant.
#[test]
fn empty_iteration_requires_empty_pending_list_and_nonempty_history() {
    let directory = fixture("empty");
    let native = directory.join(NATIVE_OUTPUT);
    fs::create_dir(&native).unwrap();
    fs::write(native.join("mutants.json"), "[]").unwrap();
    assert!(native_reports(&directory).is_err());
    fs::write(native.join("previously_caught.txt"), "").unwrap();
    assert!(native_reports(&directory).is_err());
    fs::write(native.join("previously_caught.txt"), "caught mutant").unwrap();
    let reports = native_reports(&directory).unwrap();
    assert!(!reports.contains_key("outcomes.json"));
    assert!(reports.contains_key("mutants.json"));
    for pending in ["[{}]", "{}", "invalid"] {
        fs::write(native.join("mutants.json"), pending).unwrap();
        assert!(native_reports(&directory).is_err(), "{pending}");
    }
    fs::remove_dir_all(directory).unwrap();
}

/// A serialized state cannot redirect mutation execution outside the development cache.
#[test]
fn unsafe_and_missing_campaign_paths_are_not_reused() {
    let cache = fixture("paths");
    let mut previous = campaign();
    save_campaign(&cache, &previous).unwrap();
    assert!(read_campaign(&cache).unwrap().is_none());
    for path in [
        "../run-escape",
        "/tmp/run-escape",
        "run-a/../run-b",
        "run-a/b",
    ] {
        previous.directory = path.to_owned();
        save_campaign(&cache, &previous).unwrap();
        assert!(read_campaign(&cache).is_err(), "{path}");
    }
    fs::remove_dir_all(cache).unwrap();
}

/// Nonregular state and reports must be rejected without touching their targets.
#[cfg(unix)]
#[test]
fn cache_refuses_symlinks() {
    use std::os::unix::fs::symlink;
    let cache = fixture("symlinks");
    let target = cache.join("untouched");
    fs::write(&target, "keep").unwrap();
    symlink(&target, cache.join(STATE_FILE)).unwrap();
    assert!(read_campaign(&cache).is_err());
    assert!(save_campaign(&cache, &campaign()).is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep");
    fs::remove_file(cache.join(STATE_FILE)).unwrap();
    symlink(&target, cache.join("state.pending")).unwrap();
    assert!(save_campaign(&cache, &campaign()).is_err());
    let native = cache.join(NATIVE_OUTPUT);
    fs::create_dir(&native).unwrap();
    symlink(&target, native.join("outcomes.json")).unwrap();
    assert!(native_reports(&cache).is_err());
    fs::remove_dir_all(cache).unwrap();
}
