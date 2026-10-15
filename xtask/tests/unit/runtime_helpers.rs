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
