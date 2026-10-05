// SPDX-License-Identifier: Apache-2.0

//! Configured test execution and inventory independent of repository policy compositions.

use crate::constants::flags;
use crate::{configuration, constants, interface::TestLevel};
use configuration::TestRunner;
use nextest_metadata::{RustTestSuiteStatusSummary, TestListSummary};
use std::{collections::BTreeMap, env};

/// Selects an explicit backend override without silently falling back when nextest is absent.
pub(crate) fn runner() -> Result<TestRunner, String> {
    match env::var(constants::TEST_RUNNER_ENV) {
        Ok(value) => parse_runner(&value),
        Err(env::VarError::NotPresent) => Ok(configuration::automation()?.testing.runner),
        Err(error) => Err(format!("invalid {}: {error}", constants::TEST_RUNNER_ENV)),
    }
}

/// Validates the supported backend names for command-line environment overrides.
fn parse_runner(value: &str) -> Result<TestRunner, String> {
    match value {
        "nextest" => Ok(TestRunner::Nextest),
        "cargo" => Ok(TestRunner::Cargo),
        _ => Err(format!(
            "{} must be nextest or cargo; found {value:?}",
            constants::TEST_RUNNER_ENV
        )),
    }
}

/// Verifies the selected runner early and explains how to install it.
pub(crate) fn verify_runner() -> Result<(), String> {
    if runner()? == TestRunner::Nextest {
        crate::command_output(&configuration::cargo_command()?, &["nextest", flags::VERSION])
            .map_err(|error| format!("nextest is required: {error}; run `cargo install cargo-nextest --locked` (or explicitly set {}=cargo)", constants::TEST_RUNNER_ENV))?;
        let config = configuration::automation()?.testing.config;
        crate::read_workspace_text(&crate::workspace_root()?, &config)?;
    }
    Ok(())
}

/// Builds a locked command using the repository's configured runner/profile and target selection.
fn arguments(
    runner: TestRunner,
    action: &str,
    unit: bool,
    full_gate: bool,
    filter: Option<&str>,
) -> Result<Vec<String>, String> {
    let mut arguments: Vec<String> = match runner {
        TestRunner::Cargo => vec!["test".to_owned()],
        TestRunner::Nextest => {
            let testing = configuration::automation()?.testing;
            vec![
                "nextest".to_owned(),
                action.to_owned(),
                "--config-file".to_owned(),
                crate::workspace_root()?
                    .join(testing.config)
                    .to_string_lossy()
                    .into_owned(),
                "--profile".to_owned(),
                if full_gate {
                    testing.ci_profile
                } else {
                    testing.profile
                },
            ]
        }
    };
    arguments.extend([flags::WORKSPACE, flags::LOCKED, "--lib", "--bins"].map(str::to_owned));
    if !unit {
        arguments.push("--tests".to_owned());
    }
    if runner == TestRunner::Nextest {
        // Default-filter customization must not turn a full gate into a subset run.
        arguments.push("--ignore-default-filter".to_owned());
        if action == "run" {
            arguments.extend(["--no-tests", "fail"].map(str::to_owned));
        }
        if let Some(filter) = filter {
            arguments.extend(["-E".to_owned(), format!("test(/(^|::){filter}_/)")]);
        }
    } else if let Some(filter) = filter {
        arguments.extend(["--".to_owned(), format!("{filter}_")]);
    }
    if action == "run" {
        reporting_arguments(&mut arguments, runner, verbose()?);
    }
    Ok(arguments)
}

/// Resolves explicit test verbosity without changing test selection or acceptance.
fn verbose() -> Result<bool, String> {
    match env::var(constants::TEST_VERBOSE_ENV) {
        Ok(value) => parse_verbose(&value),
        Err(env::VarError::NotPresent) => Ok(configuration::automation()?.testing.verbose),
        Err(error) => Err(format!("invalid {}: {error}", constants::TEST_VERBOSE_ENV)),
    }
}

/// Accepts conventional Boolean spellings for the per-command verbosity override.
fn parse_verbose(value: &str) -> Result<bool, String> {
    match value {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(format!(
            "{} must be true, false, 1, or 0",
            constants::TEST_VERBOSE_ENV
        )),
    }
}

/// Reduces successful-test noise while preserving summaries and failure diagnostics.
fn reporting_arguments(arguments: &mut Vec<String>, runner: TestRunner, verbose: bool) {
    if runner == TestRunner::Nextest {
        let level = if verbose { "pass" } else { "slow" };
        arguments.extend(
            [
                "--status-level",
                level,
                "--final-status-level",
                "fail",
                "--show-progress",
                "auto",
            ]
            .map(str::to_owned),
        );
    } else if !verbose {
        // Libtest options belong after the separator, including filtered runs.
        if !arguments.iter().any(|argument| argument == "--") {
            arguments.push("--".to_owned());
        }
        arguments.push("--quiet".to_owned());
    }
}

/// Executes test binaries with the selected backend; shell smoke stays in its host adapter.
pub(crate) fn run(level: TestLevel, full_gate: bool) -> Result<(), String> {
    if level == TestLevel::Smoke {
        return crate::run_shell_smoke();
    }
    let filter = match level {
        TestLevel::Integration => Some("integration"),
        TestLevel::System => Some("system"),
        TestLevel::Conformance => Some("conformance"),
        TestLevel::Property => Some("property"),
        TestLevel::Security => Some("security"),
        _ => None,
    };
    execute(level == TestLevel::Unit, full_gate, filter)?;
    if level == TestLevel::All {
        verify_counts(configuration::active_test_profile(), full_gate)?;
        // Nextest runs binaries, not doctests. Keep doctests in the full quality gate.
        if full_gate {
            crate::run_cargo(&["test", flags::WORKSPACE, "--doc", flags::LOCKED])?;
        }
    }
    Ok(())
}

/// Runs one named regression category, including the bounded fuzz-smoke suite.
pub(crate) fn run_filter(filter: &str) -> Result<(), String> {
    if !filter
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        || filter.is_empty()
    {
        return Err("test filter must be a nonempty category identifier".to_owned());
    }
    execute(false, false, Some(filter))
}

/// Executes a command with inherited streams and a visible backend/profile selection.
fn execute(unit: bool, full_gate: bool, filter: Option<&str>) -> Result<(), String> {
    verify_runner()?;
    let backend = runner()?;
    let arguments = arguments(backend, "run", unit, full_gate, filter)?;
    crate::output::info(format!(
        "tests: {:?}{}",
        backend,
        if full_gate { " (full gate)" } else { "" }
    ));
    crate::run_cargo(&arguments.iter().map(String::as_str).collect::<Vec<_>>())
}

/// Counts actual runnable test names, never occurrences in binary paths, banners, or metadata.
fn counts<'a>(
    minimums: &BTreeMap<String, usize>,
    names: impl Iterator<Item = &'a str>,
) -> BTreeMap<String, usize> {
    let mut counts = minimums
        .keys()
        .map(|name| (name.clone(), 0))
        .collect::<BTreeMap<_, _>>();
    for name in names {
        let leaf = name.rsplit("::").next().unwrap_or(name);
        for (category, count) in &mut counts {
            if leaf
                .strip_prefix(category.as_str())
                .is_some_and(|suffix| suffix.starts_with('_'))
            {
                *count += 1;
            }
        }
    }
    counts
}

/// Parses Cargo's explicit libtest records when the compatibility backend is selected.
fn cargo_test_names(list: &str) -> impl Iterator<Item = &str> {
    list.lines().filter_map(|line| line.strip_suffix(": test"))
}

/// Verifies category minima using nextest's own typed listing format or explicit libtest records.
pub(crate) fn verify_counts(profile: &str, full_gate: bool) -> Result<(), String> {
    let minimums = configuration::test_minimums(profile)?;
    let backend = runner()?;
    let mut args = arguments(backend, "list", false, full_gate, None)?;
    if backend == TestRunner::Nextest {
        args.extend(["--message-format", "json-pretty"].map(str::to_owned));
    } else {
        args.extend(["--", "--list"].map(str::to_owned));
    }
    let output = crate::command_output(
        &configuration::cargo_command()?,
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    let discovered = if backend == TestRunner::Nextest {
        let listing = TestListSummary::parse_json(&output)
            .map_err(|error| format!("invalid nextest inventory: {error}"))?;
        let actual = listing
            .rust_suites
            .values()
            .map(|suite| suite.test_cases.len())
            .sum::<usize>();
        if actual != listing.test_count {
            return Err("nextest inventory test count is inconsistent".to_owned());
        }
        let names = listing
            .rust_suites
            .values()
            .filter(|suite| suite.status == RustTestSuiteStatusSummary::LISTED)
            .flat_map(|suite| &suite.test_cases)
            .filter(|(_, case)| !case.ignored && case.filter_match.is_match())
            .map(|(name, _)| name.as_str());
        let discovered = counts(&minimums, names);
        let directory = crate::unique_generated_directory(&crate::result_root()?.join("tests"))?;
        std::fs::write(directory.join("inventory.json"), format!("{output}\n"))
            .map_err(|error| format!("could not write test inventory: {error}"))?;
        discovered
    } else {
        let mut discovered = counts(&minimums, cargo_test_names(&output));
        // Libtest's default listing includes ignored tests; subtract its explicit ignored-only listing.
        args.push("--ignored".to_owned());
        let ignored = crate::command_output(
            &configuration::cargo_command()?,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
        )?;
        let ignored = counts(&minimums, cargo_test_names(&ignored));
        for (category, count) in &mut discovered {
            *count = count
                .checked_sub(ignored[category])
                .ok_or("Cargo test inventory has inconsistent ignored counts")?;
        }
        discovered
    };
    crate::validate_test_minimums(&minimums, &discovered)
}

#[cfg(test)]
#[path = "../../tests/unit/test_execution.rs"]
mod tests;
