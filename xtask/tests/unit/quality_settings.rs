// SPDX-License-Identifier: Apache-2.0

//! Operational quality policy rejects invalid settings without trusting status annotations.

use super::*;

/// Returns the checked-in policy text without inventing alternative thresholds.
fn policy_text() -> &'static str {
    include_str!("../../../config/quality-gates.toml")
}

/// Integer and fractional percentages remain typed and snapshots do not follow later text edits.
#[test]
fn quality_policy_snapshot_is_typed_and_immutable() {
    let original = QualityPolicy::parse(policy_text()).unwrap();
    let mut changed: toml::Value = toml::from_str(policy_text()).unwrap();
    changed["coverage"]["minimum_function_percent"] = toml::Value::Float(90.5);
    let changed = toml::to_string(&changed).unwrap();
    let next = QualityPolicy::parse(&changed).unwrap();
    assert_eq!(
        next.coverage.minimum_function_percent.to_bits(),
        90.5_f64.to_bits()
    );
    assert_eq!(
        original.fuzz.minimum_seconds_per_target.to_string(),
        crate::quality_value("fuzz", "minimum_seconds_per_target").unwrap()
    );
    let annotation = policy_text().replace("status = \"pass\"", "status = \"not-reviewed\"");
    assert_eq!(
        QualityPolicy::parse(&annotation)
            .unwrap()
            .coverage
            .minimum_function_percent
            .to_bits(),
        original.coverage.minimum_function_percent.to_bits()
    );
}

/// Missing, malformed, escaping, and nonfinite operational values cannot become defaults.
#[test]
fn quality_policy_rejects_invalid_operational_settings() {
    for (section, field, value) in [
        (
            "coverage",
            "minimum_line_percent",
            toml::Value::Float(f64::NAN),
        ),
        (
            "coverage",
            "minimum_line_percent",
            toml::Value::Integer(101),
        ),
        ("coverage", "minimum_line_percent", toml::Value::Integer(-1)),
        (
            "coverage",
            "minimum_line_percent",
            toml::Value::String("85".to_owned()),
        ),
        (
            "coverage",
            "html_output",
            toml::Value::String("../outside".to_owned()),
        ),
        (
            "mutation",
            "critical_target",
            toml::Value::String("/outside".to_owned()),
        ),
        ("fuzz", "targets", toml::Value::Array(Vec::new())),
        (
            "fuzz",
            "minimum_seconds_per_target",
            toml::Value::Integer(0),
        ),
        (
            "performance",
            "harness",
            toml::Value::String("other/performance".to_owned()),
        ),
    ] {
        let mut changed: toml::Value = toml::from_str(policy_text()).unwrap();
        changed[section][field] = value;
        assert!(
            QualityPolicy::parse(&toml::to_string(&changed).unwrap()).is_err(),
            "accepted invalid {section}.{field}"
        );
    }
    let mut missing: toml::Value = toml::from_str(policy_text()).unwrap();
    missing["coverage"]
        .as_table_mut()
        .unwrap()
        .remove("minimum_function_percent");
    assert!(QualityPolicy::parse(&toml::to_string(&missing).unwrap()).is_err());
}
