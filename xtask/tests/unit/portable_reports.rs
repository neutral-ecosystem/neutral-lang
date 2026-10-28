// SPDX-License-Identifier: Apache-2.0

//! Portable migration review reports retain their JSON schema.

use super::*;

/// Empty and quoted successor labels remain data, with numeric file counts and no extra fields.
#[test]
fn portable_migration_report_preserves_schema_and_successor() {
    for next_series in ["", "next\"\\\n\té"] {
        let report = PortableMigrationReport {
            schema_version: 1,
            tree_sha256: "digest",
            file_count: 0,
            next_series,
            status: "review-required",
        };
        let bytes = crate::runtime::json::pretty(&report).unwrap();
        assert_eq!(bytes.last(), Some(&b'\n'));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            serde_json::json!({
                "schema_version": 1,
                "tree_sha256": "digest",
                "file_count": 0,
                "next_series": next_series,
                "status": "review-required"
            })
        );
    }
}
