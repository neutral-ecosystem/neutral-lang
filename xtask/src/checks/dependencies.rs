// SPDX-License-Identifier: Apache-2.0

//! checks / dependencies responsibilities for repository automation.

use crate::{
    BTreeMap, BTreeSet, Path, cargo_discovery, collect_regular_files, constants, fs,
    read_workspace_text, workspace_package_directory, workspace_root,
};

/// Checks all declared package and effect boundaries against Cargo's graph.
pub(crate) fn check_boundaries() -> Result<(), String> {
    let workspace_root = workspace_root()?;
    verify_pure_source_effects(&workspace_root)?;
    verify_release_path_independence(&workspace_root)?;
    let policy = direct_dependency_policy();
    let metadata = cargo_discovery::metadata(&workspace_root, true)?;

    for (package, allowed_dependencies) in &policy {
        let dependencies = cargo_discovery::direct_dependencies(&metadata, package)?;
        validate_direct_dependencies(package, &dependencies, allowed_dependencies)?;
    }

    let compiler_closure = cargo_discovery::closure(&metadata, constants::NEUTRAL_COMPILER, false)?;
    validate_allowed_packages(
        &format!("{} pure compilation closure", constants::NEUTRAL_COMPILER),
        &compiler_closure,
        &set([
            constants::NEUTRAL_COMPILER,
            constants::NEUTRAL_CORE,
            constants::NEUTRAL_IR,
            constants::NEUTRAL_VOCABULARY,
            "block-buffer",
            "cfg-if",
            "cpufeatures",
            "crypto-common",
            "digest",
            "generic-array",
            "sha2",
            "trybox",
            "triomphe",
            "typenum",
            "version_check",
        ]),
    )?;

    let probe_closure = cargo_discovery::closure(&metadata, constants::NEUTRAL_PROBE, true)?;
    validate_allowed_packages(
        "neutral-probe dependency tree",
        &probe_closure,
        &set([
            constants::NEUTRAL_PROBE,
            constants::NEUTRAL_CORE,
            constants::NEUTRAL_ENCODING,
            constants::NEUTRAL_IR,
            constants::NEUTRAL_READER,
            constants::NEUTRAL_VOCABULARY,
            "block-buffer",
            "cfg-if",
            "cpufeatures",
            "crypto-common",
            "digest",
            "generic-array",
            "sha2",
            "trybox",
            "triomphe",
            "typenum",
            "version_check",
        ]),
    )?;

    crate::output::pass("dependency boundaries");
    Ok(())
}

/// Rejects ambient host APIs from the captured-compilation source closure.
pub(crate) fn verify_pure_source_effects(root: &Path) -> Result<(), String> {
    for package in [
        constants::NEUTRAL_CORE,
        constants::NEUTRAL_IR,
        constants::NEUTRAL_VOCABULARY,
        constants::NEUTRAL_COMPILER,
    ] {
        let mut files = Vec::new();
        collect_regular_files(
            &workspace_package_directory(root, package)?.join("src"),
            &mut files,
        )?;
        for file in files
            .iter()
            .filter(|path| path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs"))
        {
            let content = fs::read_to_string(file)
                .map_err(|error| format!("could not read {}: {error}", file.display()))?;
            for forbidden in [
                "std::fs",
                "std::env",
                "std::net",
                "std::process",
                "std::time",
                "Command::new",
                "SystemTime",
                "Instant::now",
            ] {
                if content.contains(forbidden) {
                    return Err(format!(
                        "pure compilation source {} contains ambient API {forbidden}",
                        file.display()
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Rejects user-specific absolute paths from release configuration and adapters.
pub(crate) fn verify_release_path_independence(root: &Path) -> Result<(), String> {
    for relative in [
        constants::RELEASE_CONFIG_FILE,
        "xtask/src/release/plan.rs",
        "xtask/src/release/approval.rs",
        "xtask/src/release/release_metadata.rs",
        "xtask/src/commands/distribution.rs",
        "scripts/linux/release.sh",
        "scripts/win/release.ps1",
    ] {
        let content = read_workspace_text(root, relative)?;
        for forbidden in [
            "/home/",
            "C:\\Users\\",
            "$HOME",
            "${HOME}",
            "%USERPROFILE%",
            "~/",
        ] {
            if content.contains(forbidden) {
                return Err(format!(
                    "release surface {relative} contains user-specific path marker {forbidden}"
                ));
            }
        }
    }
    Ok(())
}

/// Returns the exact normal-dependency policy for every workspace package.
pub(crate) fn direct_dependency_policy() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    BTreeMap::from([
        (constants::NEUTRAL_BENCH, set([])),
        (
            constants::NEUTRAL_CLI,
            set([
                constants::NEUTRAL_COMPILER,
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_ENCODING,
                constants::NEUTRAL_READER,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (
            constants::NEUTRAL_COMPILER,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (constants::NEUTRAL_CORE, set(["sha2", "trybox", "triomphe"])),
        (
            constants::NEUTRAL_ENCODING,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_READER,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (constants::NEUTRAL_IR, set([constants::NEUTRAL_CORE])),
        (
            constants::NEUTRAL_PROBE,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_ENCODING,
                constants::NEUTRAL_READER,
            ]),
        ),
        (
            constants::NEUTRAL_READER,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (constants::NEUTRAL_TEST_SUITE, set([])),
        (constants::NEUTRAL_TEST_SUPPORT, set([])),
        (
            constants::NEUTRAL_VOCABULARY,
            set([constants::NEUTRAL_CORE, constants::NEUTRAL_IR]),
        ),
        (
            constants::XTASK,
            set([
                "sha2",
                "serde",
                "serde_json",
                "toml",
                "toml_edit",
                "cargo_metadata",
                "nextest-metadata",
            ]),
        ),
    ])
}

/// Rejects direct dependencies that differ from the package's exact policy.
pub(crate) fn validate_direct_dependencies(
    package: &str,
    actual: &BTreeSet<String>,
    allowed: &BTreeSet<&str>,
) -> Result<(), String> {
    let allowed = allowed.iter().map(|name| (*name).to_owned()).collect();
    validate_exact_packages(&format!("{package} direct dependencies"), actual, &allowed)
}

/// Compares an observed package set with an exact expected package set.
pub(crate) fn validate_exact_packages(
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
pub(crate) fn validate_allowed_packages(
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
pub(crate) fn set<const N: usize>(values: [&'static str; N]) -> BTreeSet<&'static str> {
    values.into_iter().collect()
}
