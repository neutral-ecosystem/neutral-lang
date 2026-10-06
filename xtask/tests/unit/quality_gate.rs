// SPDX-License-Identifier: Apache-2.0

//! Legacy receipt strings and current typed gate dispatch must agree exactly.

use super::*;

/// All supported receipt identities, including the optional fuzz command.
const SUPPORTED_GATES: [QualityGate; 6] = [
    QualityGate::Coverage,
    QualityGate::Mutation,
    QualityGate::Fuzz,
    QualityGate::PerformanceRelease,
    QualityGate::PerformanceSoak,
    QualityGate::Advisories,
];

/// Every required gate preserves its external name, serde representation, and toolchain rule.
#[test]
fn quality_gate_names_round_trip_without_schema_changes() {
    let names = [
        "coverage",
        "mutation",
        "fuzz",
        "performance-release",
        "performance-soak",
        "advisories",
    ];
    for (gate, name) in SUPPORTED_GATES.into_iter().zip(names) {
        assert_eq!(gate.as_str(), name);
        assert_eq!(gate.to_string(), name);
        let json = serde_json::to_string(&gate).unwrap();
        assert_eq!(json, format!("\"{name}\""));
        assert_eq!(serde_json::from_str::<QualityGate>(&json).unwrap(), gate);
        assert_eq!(gate.requires_nightly(), matches!(name, "coverage" | "fuzz"));
    }
    assert!(serde_json::from_str::<QualityGate>("\"unknown\"").is_err());
}

/// Release preparation, approval, and retention share non-fuzz requirements without weakening other gates.
#[test]
fn release_requirements_exclude_only_optional_fuzz() {
    let expected: Vec<_> = SUPPORTED_GATES
        .into_iter()
        .filter(|gate| *gate != QualityGate::Fuzz)
        .collect();
    assert_eq!(
        QualityGate::RELEASE_REQUIRED.as_slice(),
        expected.as_slice()
    );
    assert!(QualityGate::Fuzz.requires_nightly());
}
