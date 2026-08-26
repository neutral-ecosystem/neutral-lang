// SPDX-License-Identifier: Apache-2.0

//! Dependency-boundary checks for the Neutral workspace.
//!
//! This automation-only crate checks the resolved Cargo graph rather than
//! trusting package documentation. It is intentionally outside production
//! dependency graphs and does not implement Neutral language behavior.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    path::PathBuf,
    process::Command,
};

/// Runs an `xtask` subcommand.
///
/// # Errors
///
/// Returns an error when the command is invalid or the resolved dependency
/// graph violates the boundary policy.
pub fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();

    match arguments.as_slice() {
        [command, action] if command == "boundary" && action == "check" => check_boundaries(),
        [command, action] if command == "boundary" && action == "help" => {
            println!("usage: cargo xtask boundary check");
            Ok(())
        }
        _ => Err("usage: cargo xtask boundary check".to_owned()),
    }
}

/// Checks all declared package and effect boundaries against Cargo's graph.
fn check_boundaries() -> Result<(), String> {
    let workspace_root = workspace_root()?;
    let policy = direct_dependency_policy();

    for (package, allowed_dependencies) in &policy {
        let dependencies = direct_dependencies(&workspace_root, package)?;
        validate_direct_dependencies(package, &dependencies, allowed_dependencies)?;
    }

    let compiler_closure = package_names(&tree_output(
        &workspace_root,
        "neutral-compiler",
        "normal",
        None,
    )?);
    validate_allowed_packages(
        "neutral-compiler pure compilation closure",
        &compiler_closure,
        &set([
            "neutral-compiler",
            "neutral-core",
            "neutral-ir",
            "neutral-vocabulary",
        ]),
    )?;

    let probe_closure = package_names(&tree_output(&workspace_root, "neutral-probe", "all", None)?);
    validate_allowed_packages(
        "neutral-probe dependency tree",
        &probe_closure,
        &set([
            "neutral-probe",
            "neutral-core",
            "neutral-ir",
            "neutral-reader",
            "neutral-vocabulary",
        ]),
    )?;

    println!("dependency boundaries: pass");
    Ok(())
}

/// Returns the workspace root derived from the `xtask` package location.
fn workspace_root() -> Result<PathBuf, String> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "xtask manifest has no workspace-root parent".to_owned())
}

/// Returns the exact normal-dependency policy for every workspace package.
fn direct_dependency_policy() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    BTreeMap::from([
        ("neutral-bench", set([])),
        (
            "neutral-cli",
            set(["neutral-compiler", "neutral-core", "neutral-reader"]),
        ),
        (
            "neutral-compiler",
            set(["neutral-core", "neutral-ir", "neutral-vocabulary"]),
        ),
        ("neutral-core", set([])),
        ("neutral-ir", set(["neutral-core"])),
        ("neutral-probe", set(["neutral-core", "neutral-reader"])),
        (
            "neutral-reader",
            set(["neutral-core", "neutral-ir", "neutral-vocabulary"]),
        ),
        ("neutral-test-suite", set([])),
        ("neutral-test-support", set([])),
        ("neutral-vocabulary", set(["neutral-core", "neutral-ir"])),
        ("xtask", set([])),
    ])
}

/// Resolves the direct normal dependencies of one workspace package.
fn direct_dependencies(
    workspace_root: &PathBuf,
    package: &str,
) -> Result<BTreeSet<String>, String> {
    let output = tree_output(workspace_root, package, "normal", Some(1))?;
    let mut packages = package_names(&output);
    packages.remove(package);
    Ok(packages)
}

/// Runs `cargo tree` and returns its UTF-8 output for one package.
fn tree_output(
    workspace_root: &PathBuf,
    package: &str,
    edges: &str,
    depth: Option<u8>,
) -> Result<String, String> {
    let mut command = Command::new("cargo");
    command.current_dir(workspace_root).args([
        "tree",
        "--locked",
        "--package",
        package,
        "--edges",
        edges,
        "--prefix",
        "none",
    ]);

    if let Some(depth) = depth {
        command.args(["--depth", &depth.to_string()]);
    }

    let output = command
        .output()
        .map_err(|error| format!("could not run cargo tree for {package}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo tree failed for {package}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo tree emitted non-UTF-8 output for {package}: {error}"))
}

/// Extracts package names from Cargo's no-prefix tree output.
fn package_names(tree: &str) -> BTreeSet<String> {
    tree.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let version_or_kind = words.next()?;
            version_or_kind.starts_with('v').then(|| name.to_owned())
        })
        .collect()
}

/// Rejects direct dependencies that differ from the package's exact policy.
fn validate_direct_dependencies(
    package: &str,
    actual: &BTreeSet<String>,
    allowed: &BTreeSet<&str>,
) -> Result<(), String> {
    let allowed = allowed.iter().map(|name| (*name).to_owned()).collect();
    validate_exact_packages(&format!("{package} direct dependencies"), actual, &allowed)
}

/// Compares an observed package set with an exact expected package set.
fn validate_exact_packages(
    boundary: &str,
    actual: &BTreeSet<String>,
    expected: &BTreeSet<String>,
) -> Result<(), String> {
    if actual == expected {
        return Ok(());
    }

    let unexpected = actual.difference(expected).collect::<Vec<_>>();
    let missing = expected.difference(actual).collect::<Vec<_>>();
    Err(format!(
        "{boundary} violates the policy; unexpected: {unexpected:?}; missing: {missing:?}"
    ))
}

/// Rejects packages outside an allowlist for a named dependency boundary.
fn validate_allowed_packages(
    boundary: &str,
    actual: &BTreeSet<String>,
    allowed: &BTreeSet<&str>,
) -> Result<(), String> {
    let unexpected = actual
        .iter()
        .filter(|package| !allowed.contains(package.as_str()))
        .collect::<Vec<_>>();

    if unexpected.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{boundary} contains forbidden packages: {unexpected:?}"
        ))
    }
}

/// Builds a sorted package-name set from an array literal.
fn set<const N: usize>(values: [&'static str; N]) -> BTreeSet<&'static str> {
    values.into_iter().collect()
}

#[cfg(test)]
/// Tests for dependency-boundary policy failures.
mod tests {
    use super::{set, validate_allowed_packages, validate_direct_dependencies};
    use std::collections::BTreeSet;

    #[test]
    /// Verifies that a compiler-to-CLI edge violates the direct-edge policy.
    fn rejects_a_forbidden_direct_compiler_dependency() {
        let actual = BTreeSet::from([
            "neutral-cli".to_owned(),
            "neutral-core".to_owned(),
            "neutral-ir".to_owned(),
            "neutral-vocabulary".to_owned(),
        ]);

        assert!(
            validate_direct_dependencies(
                "neutral-compiler",
                &actual,
                &set(["neutral-core", "neutral-ir", "neutral-vocabulary"]),
            )
            .is_err()
        );
    }

    #[test]
    /// Verifies that a compiler package in the probe closure is rejected.
    fn rejects_a_compiler_dependency_in_the_probe_closure() {
        let actual = BTreeSet::from([
            "neutral-compiler".to_owned(),
            "neutral-core".to_owned(),
            "neutral-probe".to_owned(),
            "neutral-reader".to_owned(),
        ]);

        assert!(
            validate_allowed_packages(
                "neutral-probe dependency tree",
                &actual,
                &set([
                    "neutral-probe",
                    "neutral-core",
                    "neutral-ir",
                    "neutral-reader",
                    "neutral-vocabulary",
                ]),
            )
            .is_err()
        );
    }
}
