// SPDX-License-Identifier: Apache-2.0

//! Typed quality records preserve historical data and reject malformed authority.

use super::*;

/// Escaping belongs to TOML, including quoted paths, control characters and Unicode.
#[test]
fn quality_records_round_trip_without_json_escaping() {
    let approval = QualityApproval {
        schema_version: QUALITY_RECORD_SCHEMA_VERSION,
        release: "v1.0.0".into(),
        commit: "a".repeat(40),
        status: "approved".into(),
        approved_at: "2026-09-11".into(),
        evaluation: "quality/é/quote\"/back\\slash\n\t\0".into(),
        quality_gates_sha256: "b".repeat(64),
    };
    let document = quality_record_toml(&approval, "LicenseRef-Test").unwrap();
    assert!(document.starts_with("# SPDX-License-Identifier: LicenseRef-Test\n\n"));
    assert_eq!(
        toml::from_str::<QualityApproval>(&document).unwrap(),
        approval
    );
    let evaluation = QualityEvaluation {
        schema_version: QUALITY_RECORD_SCHEMA_VERSION,
        commit: "a".repeat(40),
        profile: "release".into(),
        status: "pass".into(),
        toolchain: "rustc \"custom\"\\path\nsecond line\t\0é".into(),
        quality_manifest_sha256: "c".repeat(64),
        quality_gates_sha256: "d".repeat(64),
    };
    let document = quality_record_toml(&evaluation, "LicenseRef-Test").unwrap();
    assert_eq!(
        toml::from_str::<QualityEvaluation>(&document).unwrap(),
        evaluation
    );
}

/// Existing release records remain readable without rewriting immutable evidence or the ledger.
#[test]
fn historical_quality_records_keep_the_same_rendered_ledger() {
    let approvals = read_quality_approvals().unwrap();
    assert_ne!(approvals, [] as [QualityApproval; 0]);
    for approval in &approvals {
        check_record_schema(approval.schema_version).unwrap();
        let encoded = quality_record_toml(approval, "LicenseRef-Test").unwrap();
        assert_eq!(
            toml::from_str::<QualityApproval>(&encoded).unwrap(),
            *approval
        );
    }
    let root = workspace_root().unwrap();
    assert_eq!(
        quality_status_markdown(&approvals, &project_license(&root).unwrap()).unwrap(),
        read_workspace_text(&root, constants::QUALITY_STATUS_FILE).unwrap()
    );
}

/// Missing fields, wrong scalar types, extra authority and unknown schemas fail closed.
#[test]
fn malformed_quality_records_are_not_partial_authority() {
    let root = workspace_root().unwrap();
    let content = read_workspace_text(&root, "quality/evidence/v0.1.0/record.toml").unwrap();
    let valid: toml::Table = toml::from_str(&content).unwrap();
    for change in ["missing", "type", "unknown"] {
        let mut document = valid.clone();
        match change {
            "missing" => {
                document.remove("commit");
            }
            "type" => {
                document.insert("commit".into(), toml::Value::Integer(42));
            }
            _ => {
                document.insert("unexpected_authority".into(), toml::Value::Boolean(true));
            }
        }
        assert!(toml::from_str::<QualityApproval>(&toml::to_string(&document).unwrap()).is_err());
    }
    for schema in [0, QUALITY_RECORD_SCHEMA_VERSION + 1, u32::MAX] {
        assert!(check_record_schema(schema).is_err());
    }
    assert!(toml::from_str::<QualityEvaluation>("schema_version=1\ncommit='abc'").is_err());
}
