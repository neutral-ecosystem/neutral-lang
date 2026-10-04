// SPDX-License-Identifier: Apache-2.0

//! Fixture editing safety and field-order regressions.

use super::*;

/// Creates a uniquely owned temporary fixture tree for one regression.
fn tree(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "neutral-fixture-edit-{label}-{}",
        std::process::id()
    ));
    fs::create_dir(&root).expect("new test tree");
    fs::create_dir_all(root.join("portable/conformance")).expect("conformance directory");
    fs::create_dir_all(root.join("portable/specs/contracts")).expect("contracts directory");
    fs::write(root.join("fixture.neu"), "neu \"0.1\"\n").expect("fixture");
    fs::write(root.join("oracle.toml"), "accepted = true\n").expect("oracle");
    fs::canonicalize(root).expect("canonical test tree")
}

/// Fixture edits support reordered keys, preserve comments, and check without writes.
#[test]
fn automation_fixture_hash_edits_preserve_formatting() {
    let root = tree("format");
    let original = "# case\n[[case]]\nfixture_sha256  = 'stale' # fixture comment\noracle_sha256 = 'stale'\noracle = 'oracle.toml'\nfixture = 'fixture.neu'\n";
    let mut document = manifest_updates::parse(original, "manifest").expect("editable manifest");
    let mut summary = SyncSummary::default();
    let registered = sync_manifest(&root, &mut document, true, &mut summary).expect("preflight");
    assert_eq!(summary.mismatch_errors.len(), 2);
    assert_eq!(document.to_string(), original);
    assert!(registered.contains("fixture.neu"));
    sync_manifest(&root, &mut document, false, &mut SyncSummary::default()).expect("sync");
    assert!(document.to_string().contains(" # fixture comment"));
    assert!(document.to_string().contains("fixture_sha256  = \""));
    let mut verified = SyncSummary::default();
    sync_manifest(&root, &mut document, true, &mut verified).expect("verify changed hashes");
    assert_eq!(verified.mismatch_errors, [] as [String; 0]);
    fs::remove_dir_all(root).expect("remove owned tree");
}

/// Failed freeze preflight leaves the conformance manifest and freeze unchanged.
#[test]
fn automation_fixture_preflight_prevents_partial_manifest_updates() {
    let root = tree("preflight");
    let manifest = "[[case]]\nfixture='fixture.neu'\nfixture_sha256='stale'\noracle='oracle.toml'\noracle_sha256='stale'\n";
    let freeze = "[input]\nsource_path='missing.md'\nsource_sha256='stale'\n";
    fs::write(
        root.join(constants::PORTABLE_CONFORMANCE_MANIFEST_FILE),
        manifest,
    )
    .expect("manifest");
    fs::write(root.join(constants::PORTABLE_CONTRACT_FREEZE_FILE), freeze).expect("freeze");
    assert!(sync_fixtures(&root, false).is_err());
    assert_eq!(
        fs::read_to_string(root.join(constants::PORTABLE_CONFORMANCE_MANIFEST_FILE))
            .expect("manifest unchanged"),
        manifest
    );
    assert_eq!(
        fs::read_to_string(root.join(constants::PORTABLE_CONTRACT_FREEZE_FILE))
            .expect("freeze unchanged"),
        freeze
    );
    assert!(registered_path(&root, "../external").is_err());
    assert!(registered_path(&root, "/external").is_err());
    fs::remove_dir_all(root).expect("remove owned tree");
}

/// Freeze pairs are matched by key in their own table, not by adjacent lines.
#[test]
fn automation_freeze_edits_support_reordered_nested_pairs() {
    let root = tree("freeze");
    let source = "[nested.input]\none_sha256='stale' # first\ntwo_path='oracle.toml'\none_path='fixture.neu'\ntwo_sha256='stale' # second\n";
    let mut document = manifest_updates::parse(source, "freeze").expect("freeze");
    let mut summary = SyncSummary::default();
    sync_freeze_table(
        &root,
        document.as_table_mut(),
        false,
        &mut summary,
        &root.join("manifest.toml"),
        b"unused",
    )
    .expect("freeze sync");
    assert_eq!(summary.freeze_changes, 2);
    assert!(document.to_string().contains(" # first"));
    assert!(document.to_string().contains(" # second"));
    fs::remove_dir_all(root).expect("remove owned tree");
}
