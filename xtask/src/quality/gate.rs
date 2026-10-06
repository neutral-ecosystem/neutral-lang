// SPDX-License-Identifier: Apache-2.0

//! Closed gate identities shared by measurement dispatch, receipts, and validation.

use serde::{Deserialize, Serialize};

/// Release measurement identifiers; serialized names preserve existing evidence paths.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum QualityGate {
    /// Whole-workspace coverage.
    Coverage,
    /// Critical mutation review.
    Mutation,
    /// Full configured fuzz campaign.
    Fuzz,
    /// Controlled release performance and allocation review.
    PerformanceRelease,
    /// Extended stress/soak review.
    PerformanceSoak,
    /// Fresh dependency vulnerability review.
    Advisories,
}

impl QualityGate {
    /// Mandatory release measurements; fuzz and extended soak remain opt-in analysis.
    pub(crate) const RELEASE_REQUIRED: [Self; 4] = [
        Self::Coverage,
        Self::Mutation,
        Self::PerformanceRelease,
        Self::Advisories,
    ];

    /// Returns the stable receipt and directory name without allocating.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Coverage => "coverage",
            Self::Mutation => "mutation",
            Self::Fuzz => "fuzz",
            Self::PerformanceRelease => "performance-release",
            Self::PerformanceSoak => "performance-soak",
            Self::Advisories => "advisories",
        }
    }

    /// Identifies gates that require isolated nightly sanitizer/coverage tooling.
    pub(crate) const fn requires_nightly(self) -> bool {
        matches!(self, Self::Coverage | Self::Fuzz)
    }
}

impl std::fmt::Display for QualityGate {
    /// Formats the stable external gate name, not the Rust variant spelling.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
#[path = "../../tests/unit/quality_gate.rs"]
mod tests;
