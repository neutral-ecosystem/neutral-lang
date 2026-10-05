// SPDX-License-Identifier: Apache-2.0

//! Legacy receipt strings and current typed gate dispatch must agree exactly.

use super::*;

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
    for (gate, name) in QualityGate::ALL.into_iter().zip(names) {
        assert_eq!(gate.as_str(), name);
        assert_eq!(gate.to_string(), name);
        let json = serde_json::to_string(&gate).unwrap();
        assert_eq!(json, format!("\"{name}\""));
        assert_eq!(serde_json::from_str::<QualityGate>(&json).unwrap(), gate);
        assert_eq!(gate.requires_nightly(), matches!(name, "coverage" | "fuzz"));
    }
    assert!(serde_json::from_str::<QualityGate>("\"unknown\"").is_err());
}
