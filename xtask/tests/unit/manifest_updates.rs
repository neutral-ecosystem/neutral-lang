// SPDX-License-Identifier: Apache-2.0

//! Manifest editing regressions independent of release state.

use super::*;

/// Package edits retain comments, spacing, and unrelated equal-valued keys.
#[test]
fn automation_manifest_edit_preserves_decoration() {
    let source = "# header\n[workspace.package]\nversion  =  '2.1.0' # release\nlicense = 'Apache-2.0'\n[workspace.metadata]\nversion = '2.1.0'\n";
    let result = package_version(source, "2.1.0", "2.2.0").expect("version edit");
    assert!(result.contains("version  =  \"2.2.0\" # release"));
    assert!(result.contains("version = '2.1.0'"));
    assert!(result.starts_with("# header"));
    assert!(package_version(source, "2.0.0", "2.2.0").is_err());
    assert!(package_version("[workspace.package]\nversion = 2", "2", "3").is_err());
    assert!(package_version("[workspace]", "2", "3").is_err());
}

/// Lock edits ignore sourced namesakes and work regardless of record field order.
#[test]
fn automation_lock_edit_selects_only_workspace_packages() {
    let source = "# lock\n[[package]]\nversion = '2.1.0' # keep\nname = 'engine'\n\n[[package]]\nname = 'engine'\nversion = '2.1.0'\nsource = 'registry+https://example.invalid/index'\n";
    let names = std::collections::BTreeSet::from(["engine".to_owned()]);
    let result = lock_versions(source, &names, "2.1.0", "2.2.0").expect("lock edit");
    assert!(result.contains("version = \"2.2.0\" # keep"));
    assert!(result.contains("version = '2.1.0'\nsource"));
    assert!(lock_versions(source, &names, "2.0.0", "2.2.0").is_err());
    let missing = std::collections::BTreeSet::from(["missing".to_owned()]);
    assert!(lock_versions(source, &missing, "2.1.0", "2.2.0").is_err());
    let duplicate = "[[package]]\nname='engine'\nversion='2.1.0'\n[[package]]\nname='engine'\nversion='2.1.0'\n";
    assert!(lock_versions(duplicate, &names, "2.1.0", "2.2.0").is_err());
}
