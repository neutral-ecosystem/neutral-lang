// SPDX-License-Identifier: Apache-2.0

//! Conservative development mutation reuse; only `mutate --full` produces quality evidence.
//!
//! Cargo-mutants owns survivor/unfinished selection. We retain its output only
//! while existing measured files, tools and execution settings remain unchanged.

use super::test_execution::{TestExecution, mutation_arguments, mutation_baseline_arguments};
use crate::{
    config::quality_settings::QualitySettings,
    constants, output, quality_evidence,
    runtime::{files::ExclusiveFileLock, progress},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Stdio},
};

/// Independent local cache schema; not an evidence or language version.
const CACHE_SCHEMA: u32 = 1;
/// Atomically published development state.
const STATE_FILE: &str = "state.json";
/// Directory owned by cargo-mutants beneath the selected campaign.
const NATIVE_OUTPUT: &str = "mutants.out";

/// Local, ignored execution state, never an accepted quality receipt.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Campaign {
    /// Version of this local storage layout.
    schema: u32,
    /// Hash of execution arguments, tools and environment.
    context: String,
    /// Exact repository-relative file hashes observed by the campaign.
    inputs: BTreeMap<String, String>,
    /// One direct generated-directory basename beneath the cache root.
    directory: String,
    /// Native report hashes only after successful, unchanged-input completion.
    passed_reports: Option<BTreeMap<String, String>>,
}

/// Reuses a completed campaign or asks cargo-mutants to retry unfinished/surviving mutants.
pub(crate) fn develop() -> Result<(), String> {
    let root = crate::workspace_root()?;
    let execution = TestExecution::load()?;
    execution.verify()?;
    let settings = QualitySettings::load(&root)?;
    let cache = quality_evidence::generated_root(&settings)?.join("mutation-development");
    fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    let _lock = ExclusiveFileLock::claim(&cache.join("campaign.lock"))?;
    let inputs = quality_evidence::input_snapshot(&root)?;
    let arguments = mutation_arguments(
        execution.backend,
        &settings.policy.mutation.critical_targets,
        "<development-output>",
        &execution.testing,
    );
    let context = execution_context(&arguments)?;
    let previous = read_campaign(&cache)?;
    let mut campaign = match previous {
        Some(previous)
            if reusable(&previous, &inputs, &context)
                && sealed_reports_match(&previous, &cache.join(&previous.directory)) =>
        {
            let directory = cache.join(&previous.directory);
            if previous.inputs == inputs && previous.passed_reports.is_some() {
                output::pass(
                    "mutation development: unchanged passing campaign reused (not release evidence)",
                );
                output::file("reports", &directory);
                return Ok(());
            }
            output::info(
                "mutation development: retrying survivors and unfinished mutants with --iterate",
            );
            previous
        }
        _ => {
            output::info("mutation development: new campaign; inputs or execution context changed");
            let directory = crate::unique_generated_directory(&cache)?;
            Campaign {
                schema: CACHE_SCHEMA,
                context: context.clone(),
                inputs: inputs.clone(),
                directory: directory
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                passed_reports: None,
            }
        }
    };
    let directory = cache.join(&campaign.directory);
    campaign.inputs = inputs;
    campaign.passed_reports = None;
    save_campaign(&cache, &campaign)?;
    output::file("reports", &directory);
    let mut arguments = mutation_arguments(
        execution.backend,
        &settings.policy.mutation.critical_targets,
        directory
            .to_str()
            .ok_or("mutation output path is not UTF-8")?,
        &execution.testing,
    );
    arguments.insert(1, "--iterate".to_owned());
    let result = run_development(&root, &directory, &arguments, "mutation").and_then(|()| {
        let reports = native_reports(&directory)?;
        if !reports.contains_key("outcomes.json") {
            output::info(
                "mutation development: no pending mutants; checking the unmutated test suite",
            );
            let baseline = mutation_baseline_arguments(execution.backend, &execution.testing);
            run_development(&root, &directory, &baseline, "baseline")?;
        }
        Ok(reports)
    });
    if campaign.inputs != quality_evidence::input_snapshot(&root)?
        || context
            != execution_context(&mutation_arguments(
                execution.backend,
                &settings.policy.mutation.critical_targets,
                "<development-output>",
                &execution.testing,
            ))?
    {
        campaign.context.clear();
        save_campaign(&cache, &campaign)?;
        return Err(
            "mutation inputs changed during execution; rerun cargo xtask mutate".to_owned(),
        );
    }
    campaign.passed_reports = Some(result?);
    save_campaign(&cache, &campaign)?;
    output::pass(
        "mutation development: pass (not release evidence; qualify with cargo xtask mutate --full)",
    );
    Ok(())
}

/// Existing files must match exactly; only additional standalone Rust test files may reuse catches.
fn reusable(campaign: &Campaign, inputs: &BTreeMap<String, String>, context: &str) -> bool {
    campaign.schema == CACHE_SCHEMA
        && campaign.context == context
        && campaign
            .inputs
            .iter()
            .all(|(path, digest)| inputs.get(path) == Some(digest))
        && inputs
            .keys()
            .filter(|path| !campaign.inputs.contains_key(*path))
            .all(|path| {
                let path = Path::new(path);
                path.extension().is_some_and(|extension| extension == "rs")
                    && path
                        .components()
                        .any(|component| component.as_os_str() == "tests")
            })
}

/// A changed or missing sealed report invalidates a pass and its skip lists together.
fn sealed_reports_match(campaign: &Campaign, directory: &Path) -> bool {
    campaign
        .passed_reports
        .as_ref()
        .is_none_or(|reports| native_reports(directory).as_ref() == Ok(reports))
}

/// Hashes commands, tool versions and environment without retaining environment values or secrets.
fn execution_context(arguments: &[String]) -> Result<String, String> {
    let cargo = crate::cargo_command()?;
    let tools = [
        crate::command_output(&crate::rustc_command()?, &["-vV"])?,
        crate::command_output(&cargo, &["--version"])?,
        crate::command_output(&cargo, &["mutants", "--version"])?,
        crate::command_output(constants::GIT_COMMAND, &["rev-parse", "HEAD"])?,
    ];
    let nextest = if arguments.iter().any(|value| value == "nextest") {
        Some(crate::command_output(&cargo, &["nextest", "--version"])?)
    } else {
        None
    };
    let environment = std::env::vars_os()
        .map(|(key, value)| {
            Ok((
                key.into_string().map_err(|_| "non-UTF-8 environment key")?,
                value
                    .into_string()
                    .map_err(|_| "non-UTF-8 environment value")?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let bytes = serde_json::to_vec(&(cargo, arguments, tools, nextest, environment))
        .map_err(|error| error.to_string())?;
    Ok(crate::sha256_hex(&bytes))
}

/// Refuses unsafe cache paths; malformed old state simply starts a fresh campaign.
fn read_campaign(cache: &Path) -> Result<Option<Campaign>, String> {
    let path = cache.join(STATE_FILE);
    if !regular_file(&path)? {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let Ok(campaign) = serde_json::from_slice::<Campaign>(&bytes) else {
        return Ok(None);
    };
    if !campaign.directory.starts_with("run-")
        || Path::new(&campaign.directory).components().count() != 1
    {
        return Err("mutation cache has an unsafe campaign path".to_owned());
    }
    let directory = cache.join(&campaign.directory);
    match fs::symlink_metadata(&directory) {
        Ok(metadata) if metadata.is_dir() => Ok(Some(campaign)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        _ => Err("mutation campaign must be a regular directory".to_owned()),
    }
}

/// Publishes state atomically so interruption leaves either the previous or new valid record.
fn save_campaign(cache: &Path, campaign: &Campaign) -> Result<(), String> {
    let pending = cache.join("state.pending");
    regular_file(&pending)?;
    regular_file(&cache.join(STATE_FILE))?;
    let bytes = serde_json::to_vec_pretty(campaign).map_err(|error| error.to_string())?;
    fs::write(&pending, bytes).map_err(|error| error.to_string())?;
    fs::rename(pending, cache.join(STATE_FILE)).map_err(|error| error.to_string())
}

/// Returns false for absent reports and rejects symlinks or nonregular files.
fn regular_file(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Ok(_) => Err(format!(
            "mutation cache requires a regular file: {}",
            path.display()
        )),
        Err(error) => Err(error.to_string()),
    }
}

/// Seals the native completion report and every list used by cargo-mutants iteration.
fn native_reports(directory: &Path) -> Result<BTreeMap<String, String>, String> {
    let native = directory.join(NATIVE_OUTPUT);
    if !fs::symlink_metadata(&native).is_ok_and(|metadata| metadata.is_dir()) {
        return Err("mutation native reports must be a regular directory".to_owned());
    }
    let mut reports = BTreeMap::new();
    for name in [
        "outcomes.json",
        "mutants.json",
        "caught.txt",
        "previously_caught.txt",
        "unviable.txt",
    ] {
        let path = native.join(name);
        if regular_file(&path)? {
            reports.insert(name.to_owned(), crate::sha256_file(&path)?);
        }
    }
    if !reports.contains_key("outcomes.json") {
        let pending = fs::read(native.join("mutants.json"))
            .map_err(|error| format!("missing mutation results: {error}"))?;
        let pending: serde_json::Value = serde_json::from_slice(&pending)
            .map_err(|error| format!("invalid pending mutation list: {error}"))?;
        let history = fs::read_to_string(native.join("previously_caught.txt"))
            .map_err(|error| format!("missing mutation history: {error}"))?;
        if pending.as_array().is_none_or(|pending| !pending.is_empty()) || history.trim().is_empty()
        {
            return Err(
                "mutation run has neither complete outcomes nor verified empty iteration history"
                    .to_owned(),
            );
        }
    }
    Ok(reports)
}

/// Captures native output while preserving cargo-mutants state on failure or interruption.
fn run_development(
    root: &Path,
    directory: &Path,
    arguments: &[String],
    phase: &str,
) -> Result<(), String> {
    let cargo = crate::cargo_command()?;
    let args = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let label = format!("mutation development/{phase}");
    output::start(&label);
    output::invocation(&cargo, &args);
    let capture = |name| -> Result<fs::File, String> {
        let path = directory.join(name);
        regular_file(&path)?;
        fs::File::create(path).map_err(|error| error.to_string())
    };
    let mut child = Command::new(cargo)
        .current_dir(root)
        .args(arguments)
        .env(constants::CARGO_TERM_COLOR_ENV, "never")
        .stdout(Stdio::from(capture(&format!("{phase}.stdout"))?))
        .stderr(Stdio::from(capture(&format!("{phase}.stderr"))?))
        .spawn()
        .map_err(|error| error.to_string())?;
    let status = progress::wait(
        &mut child,
        &label,
        None,
        Some(&directory.join(NATIVE_OUTPUT)),
    )
    .map_err(|error| error.to_string())?;
    if !status.success() {
        output::file("tool errors", &directory.join(format!("{phase}.stderr")));
        output::file(
            "surviving mutants",
            &directory.join(NATIVE_OUTPUT).join("missed.txt"),
        );
        return Err(format!(
            "mutation development failed ({status}); rerun cargo xtask mutate to retry survivors/unfinished mutants"
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/mutation.rs"]
mod tests;
