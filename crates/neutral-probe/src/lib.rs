// SPDX-License-Identifier: Apache-2.0

//! Reader-only inspection of compiler-free Neutral artifacts.

/// Reader-only complete expression inspection and safe public entry views.
pub mod expression;

/// Stable host-output prefixes shared by the probe library and binary.
pub mod output {
    /// Error output category prefix.
    pub const ERROR: &str = "FAIL";
    /// Informational output category prefix.
    pub const INFO: &str = "INFO";
}

/// Stable consumer-owned diagnostic identifiers.
pub mod diagnostics {
    /// Source-linked probe observation diagnostic.
    pub const OBSERVATION: &str = "NEU-PROBE-001";
}
