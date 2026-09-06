// SPDX-License-Identifier: Apache-2.0

//! Unit tests for semantic name validation and collection helpers.

use super::{AsciiNameCategory, classify_ascii_name};
use crate::language::names;

#[test]
/// Verifies exact ASCII snake and uppercase-leading name categories.
fn unit_semantics_validates_frozen_names() {
    assert_eq!(classify_ascii_name("answer_two2"), AsciiNameCategory::Snake);
    assert_eq!(classify_ascii_name("Record2"), AsciiNameCategory::Upper);
    assert_eq!(
        classify_ascii_name("Record_Name"),
        AsciiNameCategory::Invalid
    );
    assert_eq!(
        classify_ascii_name("answer__two"),
        AsciiNameCategory::Invalid
    );
}

#[test]
/// Verifies the frozen core namespace cannot be redeclared.
fn unit_semantics_protects_core_names() {
    assert!(names::is_protected_name(names::NUM));
    assert!(names::is_protected_name(names::RECORD));
    assert!(!names::is_protected_name("answer"));
}
