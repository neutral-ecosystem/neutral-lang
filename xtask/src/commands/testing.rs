// SPDX-License-Identifier: Apache-2.0

//! commands / testing responsibilities for repository automation.

use crate::constants::flags;
use crate::{BTreeMap, TestLevel, constants, run_cargo, test_execution};

/// Runs one independently selectable, stage-free test level.
pub(crate) fn test_suite(level: TestLevel) -> Result<(), String> {
    test_execution::run(level, false)
}

/// Runs one active cross-package suite by stable test-name prefix.
pub(crate) fn run_active_test_filter(suite: &str) -> Result<(), String> {
    test_execution::run_filter(suite)
}

/// Runs the built CLI and probe command-shell smoke checks.
pub(crate) fn run_shell_smoke() -> Result<(), String> {
    run_cargo(&[
        "run",
        "--quiet",
        flags::PACKAGE,
        constants::NEUTRAL_CLI,
        "--",
        "--help",
    ])?;
    run_cargo(&[
        "run",
        "--quiet",
        flags::PACKAGE,
        constants::NEUTRAL_PROBE,
        "--",
        "--help",
    ])
}

/// Rejects an active test category whose discovered count is below its minimum.
pub(crate) fn validate_test_minimums(
    minimums: &BTreeMap<String, usize>,
    discovered: &BTreeMap<String, usize>,
) -> Result<(), String> {
    for (category, minimum) in minimums {
        let discovered = discovered.get(category).copied().unwrap_or_default();
        if discovered < *minimum {
            return Err(format!(
                "active suite {category} requires at least {minimum} tests; discovered {discovered}"
            ));
        }
    }
    Ok(())
}
