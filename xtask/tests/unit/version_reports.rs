// SPDX-License-Identifier: Apache-2.0

//! Version preparation keeps a stable structured review report.

use super::*;

/// Exact JSON fields, empty arrays and hostile strings survive structured serialization.
#[test]
fn version_preparation_report_preserves_schema_and_text() {
    let report = VersionPreparationReport {
        schema_version: 1,
        current: "old\"\\\né",
        requested: "new\tversion",
        derived_updates: [],
        contract_freeze_sha256: "digest",
        frozen_contracts_changed: false,
        actions: [
            "edit-workspace-package-version",
            "run-version-check",
            "review",
        ],
        status: "review-required",
    };
    let bytes = crate::runtime::json::pretty(&report).unwrap();
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
        serde_json::json!({
            "schema_version": 1,
            "current": "old\"\\\né",
            "requested": "new\tversion",
            "derived_updates": [],
            "contract_freeze_sha256": "digest",
            "frozen_contracts_changed": false,
            "actions": ["edit-workspace-package-version", "run-version-check", "review"],
            "status": "review-required"
        })
    );
}
