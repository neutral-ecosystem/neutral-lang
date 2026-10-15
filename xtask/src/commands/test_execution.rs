// SPDX-License-Identifier: Apache-2.0

//! Configured test execution and inventory independent of repository policy compositions.

use crate::constants::flags;
use crate::{configuration, constants, interface::TestLevel};
use configuration::TestRunner;
use nextest_metadata::{RustTestSuiteStatusSummary, TestListSummary};
use std::{collections::BTreeMap, env};

/// Existing explicit categories excluded by the focused crate-local unit selection.
const NON_UNIT_CATEGORIES: &[&str] = &[
    "integration_",
    "system_",
    "conformance_",
    "property_",
    "security_",
    "smoke_",
    "determinism_",
    "fuzz_",
];

/// One invocation's backend, profiles, output policy and executable; never cached globally.
pub(crate) struct TestExecution {
    /// Effective backend after environment overrides.
    pub(crate) backend: TestRunner,
    /// Parsed profiles and selection policy for the invocation.
    pub(crate) testing: crate::configuration_models::Testing,
    /// Effective successful-test reporting policy.
    verbose: bool,
    /// Effective Cargo executable, resolved once.
    cargo: String,
}
impl TestExecution {
    /// Loads configuration and environment overrides once for a complete test invocation.
    pub(crate) fn load() -> Result<Self, String> {
        let config = configuration::automation()?;
        let backend = match env::var(constants::TEST_RUNNER_ENV) {
            Ok(value) => parse_runner(&value)?,
            Err(env::VarError::NotPresent) => config.testing.runner,
            Err(error) => return Err(format!("invalid {}: {error}", constants::TEST_RUNNER_ENV)),
        };
        let verbose = match env::var(constants::TEST_VERBOSE_ENV) {
            Ok(value) => parse_verbose(&value)?,
            Err(env::VarError::NotPresent) => config.testing.verbose,
            Err(error) => return Err(format!("invalid {}: {error}", constants::TEST_VERBOSE_ENV)),
        };
        let cargo =
            configuration::configured_command(constants::CARGO_COMMAND_ENV, config.tools.cargo)?;
        Ok(Self {
            backend,
            testing: config.testing,
            verbose,
            cargo,
        })
    }
    /// Checks the selected backend using the same executable and config as execution.
    pub(crate) fn verify(&self) -> Result<(), String> {
        if self.backend == TestRunner::Nextest {
            crate::command_output(&self.cargo, &["nextest", flags::VERSION])
                .map_err(|error| format!("nextest is required: {error}; run cargo install cargo-nextest --locked (or explicitly set {}=cargo)", constants::TEST_RUNNER_ENV))?;
            crate::read_workspace_text(&crate::workspace_root()?, &self.testing.config)?;
        }
        Ok(())
    }
    /// Runs through the shared subprocess adapter without resolving configuration again.
    fn run(&self, arguments: &[String]) -> Result<(), String> {
        crate::run_program(
            std::path::Path::new(&self.cargo),
            &arguments
                .iter()
                .map(std::ffi::OsStr::new)
                .collect::<Vec<_>>(),
        )
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
    TestExecution::load()?.verify()
}

impl TestExecution {
    /// Builds a locked command using the repository's configured runner/profile and target selection.
    fn arguments(
        &self,
        runner: TestRunner,
        action: &str,
        unit: bool,
        full_gate: bool,
        filter: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let mut arguments: Vec<String> = match runner {
            TestRunner::Cargo => vec!["test".to_owned()],
            TestRunner::Nextest => {
                let testing = &self.testing;
                vec![
                    "nextest".to_owned(),
                    action.to_owned(),
                    "--config-file".to_owned(),
                    crate::workspace_root()?
                        .join(&testing.config)
                        .to_string_lossy()
                        .into_owned(),
                    "--profile".to_owned(),
                    if full_gate {
                        testing.ci_profile.clone()
                    } else {
                        testing.profile.clone()
                    },
                ]
            }
        };
        arguments.extend([flags::WORKSPACE, flags::LOCKED, "--lib", "--bins"].map(str::to_owned));
        if unit {
            arguments.extend(["--exclude", constants::NEUTRAL_TEST_SUITE].map(str::to_owned));
        }
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
            if unit {
                let excluded = NON_UNIT_CATEGORIES
                    .iter()
                    .map(|category| format!("test(~{category})"))
                    .collect::<Vec<_>>()
                    .join(" | ");
                arguments.extend(["-E".to_owned(), format!("not ({excluded})")]);
            }
        } else if let Some(filter) = filter {
            arguments.extend(["--".to_owned(), format!("{filter}_")]);
        }
        if runner == TestRunner::Cargo && unit {
            arguments.push("--".to_owned());
            for category in NON_UNIT_CATEGORIES {
                arguments.extend(["--skip".to_owned(), (*category).to_owned()]);
            }
        }
        if action == "run" {
            reporting_arguments(&mut arguments, runner, self.verbose);
        }
        Ok(arguments)
    }

    /// Instruments the complete target set while reusing the selected test runner's policy.
    pub(crate) fn coverage_arguments(&self, backend: TestRunner) -> Result<Vec<String>, String> {
        let tests = self.arguments(backend, "run", false, true, None)?;
        let mut command = vec!["llvm-cov".to_owned()];
        let prefix = if backend == TestRunner::Nextest {
            command.push("nextest".to_owned());
            2
        } else {
            1
        };
        command.extend(["--all-targets", "--no-report"].map(str::to_owned));
        command.extend(
            tests
                .into_iter()
                .skip(prefix)
                .filter(|argument| !matches!(argument.as_str(), "--lib" | "--bins" | "--tests")),
        );
        Ok(command)
    }
}

/// Selects cargo-mutants' native integration with the configured test backend.
pub(crate) fn mutation_arguments(
    backend: TestRunner,
    targets: &[String],
    output: &str,
    testing: &crate::configuration_models::Testing,
) -> Vec<String> {
    let tool = backend.as_str();
    let mut command = ["mutants", "--test-tool", tool, "--output", output]
        .map(str::to_owned)
        .to_vec();
    for target in targets {
        command.extend(["--file".to_owned(), target.clone()]);
    }
    command.push("--no-config".to_owned());
    // Baseline timing must include the same workspace tested for each mutant.
    command.push("--cargo-arg=--workspace".to_owned());
    command.extend(["--jobs".to_owned(), testing.mutation_jobs.to_string()]);
    // Generated nested build trees can exceed the temporary workspace quota.
    command.extend(["--gitignore".to_owned(), "true".to_owned()]);
    // Workspace tests must resolve Git policy within the copy, not its parent checkout.
    command.extend(["--copy-vcs".to_owned(), "true".to_owned()]);
    if backend == TestRunner::Nextest {
        // cargo-mutants tests copied workspaces: resolve config relative to that copy.
        command.push("--".to_owned());
        command.extend(mutation_nextest_arguments(testing));
    }
    command
}

/// Runs the unmutated suite even when native iteration has skipped every mutant.
pub(crate) fn mutation_baseline_arguments(
    backend: TestRunner,
    testing: &crate::configuration_models::Testing,
) -> Vec<String> {
    let mut command = if backend == TestRunner::Nextest {
        vec!["nextest".to_owned(), "run".to_owned()]
    } else {
        vec!["test".to_owned()]
    };
    command.extend([flags::WORKSPACE, flags::LOCKED].map(str::to_owned));
    if backend == TestRunner::Nextest {
        command.extend(mutation_nextest_arguments(testing));
    }
    command
}

/// Shares the complete mutation-profile test selection with the zero-pending baseline.
fn mutation_nextest_arguments(testing: &crate::configuration_models::Testing) -> Vec<String> {
    vec![
        "--config-file".to_owned(),
        testing.config.clone(),
        "--profile".to_owned(),
        testing.mutation_profile.clone(),
        "--fail-fast".to_owned(),
        "--ignore-default-filter".to_owned(),
        "--no-tests".to_owned(),
        "fail".to_owned(),
    ]
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
    let execution = TestExecution::load()?;
    execute(&execution, level == TestLevel::Unit, full_gate, filter)?;
    if level == TestLevel::All {
        verify_counts(&execution, configuration::active_test_profile(), full_gate)?;
        // Nextest cannot run Rustdoc tests; every complete run includes them separately.
        execution.run(&["test", flags::WORKSPACE, "--doc", flags::LOCKED].map(str::to_owned))?;
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
    execute(&TestExecution::load()?, false, false, Some(filter))
}

/// Executes a command with inherited streams and a visible backend/profile selection.
fn execute(
    execution: &TestExecution,
    unit: bool,
    full_gate: bool,
    filter: Option<&str>,
) -> Result<(), String> {
    execution.verify()?;
    let backend = execution.backend;
    let arguments = execution.arguments(backend, "run", unit, full_gate, filter)?;
    crate::output::info(format!(
        "tests: {:?}{}",
        backend,
        if full_gate { " (full gate)" } else { "" }
    ));
    execution.run(&arguments)
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
fn verify_counts(execution: &TestExecution, profile: &str, full_gate: bool) -> Result<(), String> {
    let minimums = configuration::test_minimums(profile)?;
    let backend = execution.backend;
    let mut args = execution.arguments(backend, "list", false, full_gate, None)?;
    if backend == TestRunner::Nextest {
        args.extend(["--message-format", "json-pretty"].map(str::to_owned));
    } else {
        args.extend(["--", "--list"].map(str::to_owned));
    }
    let output = crate::command_output(
        &execution.cargo,
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
