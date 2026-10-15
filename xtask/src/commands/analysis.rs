// SPDX-License-Identifier: Apache-2.0

//! commands / analysis responsibilities for repository automation.

use crate::config::quality_settings::QualitySettings;
use crate::constants::{flags, reports};
use crate::quality::gate::QualityGate;
use crate::{
    FuzzMode, Path, PerformanceProfile, cargo_discovery, constants, fs, output, quality_evidence,
    result_root, run_active_test_filter, run_cargo, workspace_root,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    process::Command,
};

/// Runs the selected durable fuzz mode.
pub(crate) fn fuzz(mode: FuzzMode) -> Result<(), String> {
    match mode {
        FuzzMode::Smoke => run_active_test_filter("fuzz_smoke"),
        FuzzMode::Campaign => coverage_guided_fuzz_campaign(),
    }
}

/// Runs every configured coverage-guided fuzz target for its approved budget.
pub(crate) fn coverage_guided_fuzz_campaign() -> Result<(), String> {
    let mut measurement = quality_evidence::Measurement::begin(QualityGate::Fuzz)?;
    let settings = measurement.settings.policy.fuzz.clone();
    let targets = settings.targets;
    let seconds = settings.minimum_seconds_per_target;
    let root = workspace_root()?;
    let corpus_root = settings.corpus_root;
    let seed_root = settings.seed_root;
    let total = targets.len();
    output::info(format!(
        "fuzz campaign: {total} targets | budget {} each | estimated total {} + build/startup",
        output::duration(std::time::Duration::from_secs(seconds)),
        output::duration(std::time::Duration::from_secs(
            seconds.saturating_mul(total as u64)
        ))
    ));
    for (index, target) in targets.into_iter().enumerate() {
        output::info(format!("fuzz [{}/{total}]: {target}", index + 1));
        let seed_directory = root.join(&seed_root).join(&target);
        let corpus_directory = corpus_root.join(&target);
        let corpus = corpus_directory
            .to_str()
            .ok_or_else(|| "fuzz corpus path is not UTF-8".to_owned())?;
        let budget = format!("-max_total_time={seconds}");
        if seed_directory.is_dir() {
            let seed = seed_directory
                .to_str()
                .ok_or_else(|| "fuzz seed path is not UTF-8".to_owned())?;
            measurement.cargo_with_progress(
                &["fuzz", "run", &target, corpus, seed, "--", &budget],
                &target,
                seconds,
            )?;
        } else {
            measurement.cargo_with_progress(
                &["fuzz", "run", &target, "--", &budget],
                &target,
                seconds,
            )?;
        }
    }
    measurement.finish()
}

/// Runs changed-file coverage by default or the authoritative workspace gate with `--full`.
pub(crate) fn coverage(full: bool) -> Result<(), String> {
    if full {
        return workspace_coverage();
    }
    changed_coverage()
}

/// Runs workspace coverage and enforces every configured percentage threshold.
fn workspace_coverage() -> Result<(), String> {
    let execution = super::test_execution::TestExecution::load()?;
    execution.verify()?;
    let mut measurement = quality_evidence::Measurement::begin(QualityGate::Coverage)?;
    let settings = measurement.settings.policy.coverage.clone();
    let lines = settings.minimum_line_percent.to_string();
    let functions = settings.minimum_function_percent.to_string();
    let regions = settings.minimum_region_percent.to_string();
    let exclusions = settings.exclusion_regex;
    let results = result_root()?;
    let html_index = results.join(&settings.html_output);
    let html = html_index
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "coverage HTML output must be below a report directory".to_owned())?;
    let json = results.join(&settings.json_output);
    fs::create_dir_all(
        json.parent()
            .ok_or_else(|| "coverage JSON output has no parent".to_owned())?,
    )
    .map_err(|error| format!("could not create coverage result directory: {error}"))?;
    let html = html
        .to_str()
        .ok_or_else(|| "coverage HTML path is not valid UTF-8".to_owned())?;
    let json = json
        .to_str()
        .ok_or_else(|| "coverage JSON path is not valid UTF-8".to_owned())?;
    let tests = execution.coverage_arguments(execution.backend)?;
    measurement.cargo(
        &tests.iter().map(String::as_str).collect::<Vec<_>>(),
        "tests",
    )?;
    measurement.cargo(
        &[
            "llvm-cov",
            "report",
            "--html",
            "--output-dir",
            html,
            "--ignore-filename-regex",
            &exclusions,
            "--fail-under-lines",
            &lines,
            "--fail-under-functions",
            &functions,
            "--fail-under-regions",
            &regions,
        ],
        "html",
    )?;
    measurement.cargo(
        &[
            "llvm-cov",
            "report",
            "--json",
            "--summary-only",
            "--output-path",
            json,
            "--ignore-filename-regex",
            &exclusions,
            "--fail-under-lines",
            &lines,
            "--fail-under-functions",
            &functions,
            "--fail-under-regions",
            &regions,
        ],
        "json",
    )?;
    output::file("coverage HTML", &html_index);
    output::file("coverage JSON", Path::new(json));
    measurement.copy_report(Path::new(json), "coverage.json")?;
    measurement.finish()
}

/// Measures changed Rust files using tests from their packages and reverse dependents.
fn changed_coverage() -> Result<(), String> {
    let root = workspace_root()?;
    let all_changed = changed_files(&root)?;
    if all_changed.iter().any(|path| is_workspace_wide_input(path)) {
        output::info("workspace or test configuration changed; running full coverage");
        return workspace_coverage();
    }
    let changed = all_changed
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect::<Vec<_>>();
    if changed.is_empty() {
        output::info("no changed Rust files; coverage skipped");
        return Ok(());
    }

    let metadata = cargo_discovery::metadata(&root, true)?;
    let members = metadata
        .workspace_packages()
        .into_iter()
        .map(|package| (package.id.clone(), package))
        .collect::<BTreeMap<_, _>>();
    let owners = changed
        .iter()
        .filter_map(|path| {
            let absolute = root.join(path);
            members
                .values()
                .filter_map(|package| {
                    package
                        .manifest_path
                        .parent()
                        .filter(|directory| absolute.starts_with(directory.as_std_path()))
                        .map(|_| (package.id.clone(), package.name.to_string()))
                })
                .max_by_key(|(id, _)| members[id].manifest_path.as_str().len())
        })
        .collect::<BTreeMap<_, _>>();
    if owners.is_empty() {
        output::info("changed Rust files are outside Cargo workspace packages; coverage skipped");
        return Ok(());
    }

    let mut impacted = owners.keys().cloned().collect::<BTreeSet<_>>();
    let nodes = metadata
        .resolve
        .as_ref()
        .ok_or("Cargo metadata did not return the resolved workspace graph")?
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect::<BTreeMap<_, _>>();
    loop {
        let additions = nodes
            .iter()
            .filter(|(_, node)| {
                node.deps
                    .iter()
                    .any(|dependency| impacted.contains(&dependency.pkg))
            })
            .map(|(id, _)| id.clone())
            .filter(|id| members.contains_key(id) && !impacted.contains(id))
            .collect::<Vec<_>>();
        if additions.is_empty() {
            break;
        }
        impacted.extend(additions);
    }
    let packages = impacted
        .iter()
        .filter_map(|id| members.get(id).map(|package| package.name.to_string()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let files = changed
        .iter()
        .map(|path| root.join(path).to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let include = format!(
        "-include-filename-regex=^({})$",
        files
            .iter()
            .map(|file| llvm_regex_path(file))
            .collect::<Vec<_>>()
            .join("|")
    );
    let mut llvm_flags = env::var("LLVM_COV_FLAGS").unwrap_or_default();
    if !llvm_flags.is_empty() {
        llvm_flags.push(' ');
    }
    llvm_flags.push_str(&include);

    let execution = super::test_execution::TestExecution::load()?;
    execution.verify()?;
    let tests = execution.coverage_arguments_for_packages(execution.backend, &packages)?;
    run_cargo(&["llvm-cov", "clean", "--workspace"])?;
    let test_args = tests.iter().map(String::as_str).collect::<Vec<_>>();
    run_cargo(&test_args)?;
    let report = cargo_discovery::metadata(&root, false)?
        .target_directory
        .into_std_path_buf()
        .join("llvm-cov-changed");
    fs::create_dir_all(&report)
        .map_err(|error| format!("could not create changed coverage report directory: {error}"))?;
    let cargo = crate::configuration::cargo_command()?;
    let report_args = [
        "llvm-cov",
        "report",
        "--html",
        "--output-dir",
        report.to_str().ok_or("coverage report path is not UTF-8")?,
    ];
    output::invocation(&cargo, &report_args);
    let status = Command::new(&cargo)
        .current_dir(&root)
        .args(report_args)
        .env("LLVM_COV_FLAGS", llvm_flags)
        .env(constants::CARGO_TERM_COLOR_ENV, output::child_color())
        .status()
        .map_err(|error| format!("could not run changed coverage report: {error}"))?;
    if !status.success() {
        return Err(format!("changed coverage report failed with {status}"));
    }
    output::info(format!(
        "changed coverage: {} file(s), {} affected package(s); diagnostic only",
        changed.len(),
        packages.len()
    ));
    output::file("changed coverage HTML", &report.join("index.html"));
    Ok(())
}

/// Returns tracked changes since the remote default branch plus all untracked files.
fn changed_files(root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let base = git_output(
        root,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )
    .map_err(|_| {
        "cannot determine the base branch; set origin/HEAD with `git remote set-head origin --auto`"
            .to_owned()
    })?
    .trim()
    .to_owned();
    let merge_base = git_output(root, &["merge-base", "HEAD", &base])
        .map_err(|_| format!("cannot find a merge base with {base}; fetch the base branch"))?;
    let merge_base = merge_base.trim();
    let tracked = git_output(root, &["diff", "--name-only", "-z", merge_base, "--"])?;
    let untracked = git_output(root, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    let mut paths = BTreeSet::new();
    for path in tracked.split('\0').chain(untracked.split('\0')) {
        if !path.is_empty() {
            paths.insert(std::path::PathBuf::from(path));
        }
    }
    Ok(paths.into_iter().collect())
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not run git {}: {error}", arguments.join(" ")))?;
    if !output.status.success() {
        return Err(format!("git {} failed", arguments.join(" ")));
    }
    String::from_utf8(output.stdout).map_err(|error| {
        format!(
            "git {} returned non-UTF-8 paths: {error}",
            arguments.join(" ")
        )
    })
}

fn is_workspace_wide_input(path: &std::path::Path) -> bool {
    path.file_name()
        .is_some_and(|name| name == "Cargo.toml" || name == "Cargo.lock")
        || path.starts_with(".cargo")
        || path.starts_with(".config")
        || path == Path::new("rust-toolchain.toml")
        || path == Path::new("rust-toolchain")
}

fn llvm_regex_path(path: &str) -> String {
    path.chars()
        .map(|character| match character {
            ' ' => "[ ]".to_owned(),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' | '?' | '*' => {
                format!("\\{character}")
            }
            character => character.to_string(),
        })
        .collect()
}

/// Runs mutation analysis for every reviewed critical production target.
pub(crate) fn mutate(full: bool) -> Result<(), String> {
    if !full {
        return super::mutation::develop();
    }
    let execution = super::test_execution::TestExecution::load()?;
    execution.verify()?;
    let mut measurement = quality_evidence::Measurement::begin(QualityGate::Mutation)?;
    let targets = measurement
        .settings
        .policy
        .mutation
        .critical_targets
        .clone();
    let output = measurement.directory.to_string_lossy().into_owned();
    let tests = super::test_execution::mutation_arguments(
        execution.backend,
        &targets,
        &output,
        &execution.testing,
    );
    measurement.cargo(
        &tests.iter().map(String::as_str).collect::<Vec<_>>(),
        "mutation",
    )?;
    let outcomes = measurement.directory.join("mutants.out/outcomes.json");
    measurement.copy_report(&outcomes, "outcomes.json")?;
    measurement.finish()
}

/// Runs one controlled benchmark, stress, or soak profile.
pub(crate) fn performance(profile: PerformanceProfile) -> Result<(), String> {
    let measured = !matches!(profile, PerformanceProfile::Pr);
    let gate = if matches!(profile, PerformanceProfile::Soak) {
        QualityGate::PerformanceSoak
    } else {
        QualityGate::PerformanceRelease
    };
    let profile = match profile {
        PerformanceProfile::Pr => "pr",
        PerformanceProfile::Release => "release",
        PerformanceProfile::Soak => "extended-soak",
    };
    match profile {
        "pr" | "release" | "extended-soak" => {
            let settings = QualitySettings::load(&workspace_root()?)?;
            let harness = settings.policy.performance.harness.clone();
            let package = harness.package.as_str();
            let target = harness.target.as_str();
            let arguments = [
                "bench",
                flags::PACKAGE,
                package,
                "--bench",
                target,
                "--",
                profile,
            ];
            if !measured {
                return run_cargo(&arguments);
            }
            let mut measurement =
                quality_evidence::Measurement::begin_with_settings(gate, settings)?;
            measurement.cargo(&arguments, "benchmark")?;
            measurement.cargo(
                &[
                    "bench",
                    flags::PACKAGE,
                    package,
                    "--bench",
                    target,
                    "--no-run",
                    "--message-format=json",
                ],
                "build",
            )?;
            let build = fs::read_to_string(measurement.directory.join("build.stdout"))
                .map_err(|error| format!("could not read benchmark build: {error}"))?;
            let executable = cargo_discovery::benchmark_executable(&build, target)?;
            let heap_report = measurement.directory.join(reports::MASSIF);
            let heap_argument = format!("--massif-out-file={}", heap_report.display());
            measurement.run(
                constants::VALGRIND_COMMAND,
                &["--tool=massif", &heap_argument, &executable, profile],
                "massif",
            )?;
            measurement.copy_report(&heap_report, reports::MASSIF)?;
            measurement.run(
                constants::VALGRIND_COMMAND,
                &[
                    "--tool=memcheck",
                    "--leak-check=full",
                    "--errors-for-leak-kinds=definite,indirect,possible",
                    "--error-exitcode=1",
                    &executable,
                    profile,
                ],
                "memcheck",
            )?;
            measurement.finish()
        }
        _ => unreachable!("performance profile is closed by command parsing"),
    }
}
