// SPDX-License-Identifier: Apache-2.0

//! commands / analysis responsibilities for repository automation.

use crate::constants::{flags, reports};
use crate::{
    FuzzMode, Path, PathBuf, PerformanceProfile, cargo_discovery, constants, fs,
    is_safe_relative_path, output, quality_array, quality_evidence, quality_output_path,
    quality_value, run_active_test_filter, run_cargo, workspace_root,
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
    let mut measurement = quality_evidence::Measurement::begin("fuzz")?;
    let targets = quality_array("fuzz", "targets")?;
    let seconds = quality_value("fuzz", "minimum_seconds_per_target")?
        .parse::<u64>()
        .map_err(|error| format!("invalid fuzz budget: {error}"))?;
    if seconds == 0 {
        return Err("fuzz budget must be positive".to_owned());
    }
    let root = workspace_root()?;
    let corpus_root = PathBuf::from(quality_value("fuzz", "corpus_root")?);
    let seed_root = PathBuf::from(quality_value("fuzz", "seed_root")?);
    if !is_safe_relative_path(&corpus_root) || !is_safe_relative_path(&seed_root) {
        return Err("fuzz corpus and seed roots must be safe workspace-relative paths".to_owned());
    }
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
        if !is_safe_relative_path(Path::new(&target)) {
            return Err(format!("unsafe fuzz target name: {target}"));
        }
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

/// Runs workspace coverage and enforces every configured percentage threshold.
pub(crate) fn coverage() -> Result<(), String> {
    let mut measurement = quality_evidence::Measurement::begin("coverage")?;
    let lines = quality_value("coverage", "minimum_line_percent")?;
    let functions = quality_value("coverage", "minimum_function_percent")?;
    let regions = quality_value("coverage", "minimum_region_percent")?;
    let exclusions = quality_value("coverage", "exclusion_regex")?;
    let html_index = quality_output_path("coverage", "html_output")?;
    let html = html_index
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "coverage HTML output must be below a report directory".to_owned())?;
    let json = quality_output_path("coverage", "json_output")?;
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
    measurement.cargo(
        &["llvm-cov", flags::WORKSPACE, "--all-targets", "--no-report"],
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

/// Runs mutation analysis for the configured critical production target.
pub(crate) fn mutate() -> Result<(), String> {
    let target = quality_value("mutation", "critical_target")?;
    let mut measurement = quality_evidence::Measurement::begin("mutation")?;
    let output = measurement.directory.to_string_lossy().into_owned();
    measurement.cargo(
        &["mutants", "--file", &target, "--output", &output],
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
        "performance-soak"
    } else {
        "performance-release"
    };
    let profile = match profile {
        PerformanceProfile::Pr => "pr",
        PerformanceProfile::Release => "release",
        PerformanceProfile::Soak => "extended-soak",
    };
    match profile {
        "pr" | "release" | "extended-soak" => {
            let harness = quality_value("performance", "harness")?;
            let (package, target) = harness
                .split_once('/')
                .ok_or_else(|| "quality performance harness must be package/target".to_owned())?;
            if package != constants::NEUTRAL_BENCH
                || target.is_empty()
                || !target
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            {
                return Err(format!("invalid quality performance harness: {harness}"));
            }
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
            let mut measurement = quality_evidence::Measurement::begin(gate)?;
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
