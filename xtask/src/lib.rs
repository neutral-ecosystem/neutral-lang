// SPDX-License-Identifier: Apache-2.0

//! Stable developer and CI automation for the Neutral workspace.
//!
//! Commands compose the owning checks, configuration, runtime, quality,
//! conformance, and release boundaries. This automation-only crate stays outside
//! production dependency graphs and does not implement language behavior.

use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write as _,
    fs::{self, OpenOptions},
    path::{Component, Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

mod checks;
mod commands;
mod config;
mod conformance;
mod interface;
mod quality;
mod release;
mod runtime;
mod versioning;

use checks::git_hygiene;
use commands::test_execution;
pub use config::constants;
use config::{cargo_discovery, configuration, configuration_models, manifest_updates};
use conformance::{fixtures, portable_stage, released_conformance};
use quality::quality_evidence;
use release::release_metadata;
pub use runtime::output;
use runtime::{environment, progress, results, workspace};

use configuration::{
    automation_value, cargo_command, cargo_target_directory, configuration_array_from,
    configuration_section, configuration_value, is_safe_relative_path, quality_array,
    quality_value, repository_directories, rustc_command,
};
use environment::{
    bootstrap, json_string, print_environment_manifest, verify_complete_environment,
    verify_environment,
};
use interface::{
    BuildProfile, CiProfile, FuzzMode, PerformanceProfile, PortableAction, QualityAction,
    QualityProfile, Task, TestLevel, ValidationTarget, VersionAction,
};
use released_conformance::ReleasedBundle;
use results::{clean_results, quality_output_path, result_root};
use versioning::{
    check_versions, collect_named_files, html_spdx_marker, line_spdx_marker, project_license,
    project_slug, sha256_hex, validate_semver, version, workspace_package_directory,
    workspace_package_license, workspace_package_manifests, workspace_package_version,
};

use checks::dependencies::check_boundaries;
use checks::repository::{
    check_generated_outputs, check_quality_inventory, check_repository_structure,
    check_workflow_contract, ensure_no_portable_archive_dependencies,
    verify_repository_markdown_links,
};
use checks::test_layout::check_test_layout;
use checks::traceability::{check_portable_traceability_at, check_traceability};
use commands::analysis::{coverage, fuzz, mutate, performance};
use commands::development::{build, check, ci, develop, format_workspace, lint};
use commands::distribution::{package, release_prepare, validate};
use commands::documentation::documentation;
use commands::portable::{portable, verify_frozen_input_digests, verify_optional_portable};
use commands::quality::{quality, quality_action, quality_profile_name};
use commands::testing::{
    run_active_test_filter, run_shell_smoke, test_suite, validate_test_minimums,
};
use quality::ledger::{
    approve_quality_release, evaluate_quality, quality_status, read_quality_approvals,
    render_quality_status, verify_quality_ledger,
};
use release::approval::{quality_document_status, release_plan, verify_release_approval};
use runtime::execution::{command_output, run_cargo, run_program};
use runtime::files::{
    collect_regular_files, is_sha256, markdown_link_targets, read_workspace_text,
    require_clean_checkout, require_main_head_checkout, sha256_file, unique_generated_directory,
};
use runtime::workflow::{run_recorded_workflow, unix_time_millis};

/// One named fallible step in an aggregate automation workflow.
type WorkflowStep<'a> = (&'static str, Box<dyn FnOnce() -> Result<(), String> + 'a>);

/// Source template for the generated workspace rustdoc landing page.
const RUSTDOC_INDEX_TEMPLATE: &str = include_str!("../templates/rustdoc-index.html");

/// Shared HTML fragment injected into every generated rustdoc page.
const RUSTDOC_HEADER_TEMPLATE: &str = include_str!("../templates/rustdoc-header.html");

/// Runs an `xtask` subcommand.
///
/// # Errors
///
/// Returns one contextual error when parsing or any selected task fails.
pub fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    let label = arguments.join(" ");
    let started = Instant::now();
    output::start(format!("xtask {label}"));
    let result = interface::parse(&arguments).and_then(execute);
    let duration = output::duration(started.elapsed());
    if result.is_ok() {
        output::pass(format!("xtask {label} ({duration})"));
    }
    result.map_err(|error| format!("xtask {label} ({duration}): {error}"))
}

/// Executes one parsed stable automation task.
fn execute(task: Task) -> Result<(), String> {
    match task {
        Task::Help => {
            output::help(interface::HELP);
            Ok(())
        }
        Task::Bootstrap => bootstrap(),
        Task::Dev => develop(),
        Task::EnvironmentVerify => verify_complete_environment(),
        Task::EnvironmentManifest => print_environment_manifest(),
        Task::Format { write } => format_workspace(write),
        Task::Lint => lint(),
        Task::Check => check(),
        Task::Build(profile) => build(profile),
        Task::Docs => documentation(),
        Task::Test(level) => test_suite(level),
        Task::Performance(profile) => performance(profile),
        Task::Fuzz(mode) => fuzz(mode),
        Task::Coverage => coverage(),
        Task::Mutate => mutate(),
        Task::Quality(action) => quality_action(action),
        Task::Validate(target) => validate(target),
        Task::Package => package(),
        Task::ReleasePrepare => release_prepare(),
        Task::ReleaseTag => output::machine(release_plan()?.release_tag),
        Task::Version(action) => version(action),
        Task::Portable(action) => portable(action),
        Task::Clean => clean_results(),
        Task::Ci(profile) => ci(profile),
        Task::Fixtures { check } => fixtures::sync_fixtures(&workspace_root()?, check),
    }
}

/// Finds the owning Cargo workspace even if the automation crate moves.
fn workspace_root() -> Result<PathBuf, String> {
    let current = env::current_dir()
        .map_err(|error| format!("could not locate the current directory: {error}"))?;
    workspace::discover(&current)
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
