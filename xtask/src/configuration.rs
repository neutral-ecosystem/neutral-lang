// SPDX-License-Identifier: Apache-2.0

//! Repository configuration access and active test-minimum selection.

use super::{
    BTreeMap, Component, Path, PathBuf, constants, env, fs, read_workspace_text, workspace_root,
};

/// One declared top-level responsibility boundary in the repository layout.
pub(super) struct RepositoryDirectory {
    /// Relative root of the owned directory.
    pub(super) path: String,
    /// Tracked ownership README inside that directory.
    pub(super) readme: String,
    /// Human owner classification used by repository policy.
    pub(super) owner: String,
    /// Declared lifecycle for the directory contents.
    pub(super) lifecycle: String,
}

/// Accepts only nonempty paths composed of normal workspace-relative components.
pub(super) fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Reads every declared top-level directory from the layout inventory.
pub(super) fn repository_directories(root: &Path) -> Result<Vec<RepositoryDirectory>, String> {
    let layout = read_workspace_text(root, constants::REPOSITORY_LAYOUT_FILE)?;
    let mut directories = Vec::new();
    for entry in layout.split("[[directory]]").skip(1) {
        let field = |key: &str| {
            configuration_value(entry, key)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("repository directory has no {key}"))
        };
        directories.push(RepositoryDirectory {
            path: field("path")?,
            readme: field("readme")?,
            owner: field("owner")?,
            lifecycle: field("lifecycle")?,
        });
    }
    if directories.is_empty() {
        return Err("repository layout has no directory entries".to_owned());
    }
    Ok(directories)
}

/// Reads scalar key/value pairs from one exact TOML section.
pub(super) fn configuration_section(
    content: &str,
    section: &str,
) -> Result<Vec<(String, String)>, String> {
    let heading = format!("[{section}]");
    let mut selected = false;
    let mut values = Vec::new();
    for line in content.lines().map(str::trim) {
        if line.starts_with('[') {
            selected = line == heading;
            continue;
        }
        if !selected || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line
            .split_once('=')
            .ok_or_else(|| format!("invalid [{section}] entry: {line}"))?;
        values.push((
            name.trim().to_owned(),
            value.trim().trim_matches('"').to_owned(),
        ));
    }
    Ok(values)
}

/// Reads one unsectioned scalar from constrained TOML-like configuration.
pub(super) fn configuration_value(content: &str, key: &str) -> Option<String> {
    content
        .lines()
        .map(str::trim)
        .take_while(|line| !line.starts_with('['))
        .find_map(|line| {
            let (candidate, value) = line.split_once('=')?;
            (candidate.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
        })
}

/// Reads one required local automation default from its declared section.
pub(super) fn automation_value(section: &str, key: &str) -> Result<String, String> {
    let root = workspace_root()?;
    let configuration = read_workspace_text(&root, constants::AUTOMATION_CONFIG_FILE)?;
    if configuration_value(&configuration, "schema_version").as_deref() != Some("1") {
        return Err("unsupported automation configuration schema".to_owned());
    }
    quality_value_from(&configuration, section, key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("automation configuration has no [{section}] {key} value"))
}

/// Selects a configured executable with an optional per-invocation override.
fn configured_command(environment: &str, key: &str) -> Result<String, String> {
    let command = match env::var(environment) {
        Ok(command) => command,
        Err(env::VarError::NotPresent) => automation_value("tools", key)?,
        Err(error) => return Err(format!("invalid {environment}: {error}")),
    };
    if command.trim().is_empty() || command.chars().any(char::is_control) {
        return Err(format!("{environment} must name one nonempty executable"));
    }
    Ok(command)
}

/// Returns the selected Cargo executable for all xtask subprocesses.
pub(super) fn cargo_command() -> Result<String, String> {
    configured_command(constants::CARGO_COMMAND_ENV, "cargo")
}

/// Returns the selected Rust compiler executable for environment and release checks.
pub(super) fn rustc_command() -> Result<String, String> {
    configured_command(constants::RUSTC_COMMAND_ENV, "rustc")
}

/// Resolves Cargo's standard target directory, including its native override.
pub(super) fn cargo_target_directory() -> Result<PathBuf, String> {
    let configured =
        env::var_os("CARGO_TARGET_DIR").map_or_else(|| PathBuf::from("target"), PathBuf::from);
    if configured.as_os_str().is_empty() {
        return Err("CARGO_TARGET_DIR must not be empty".to_owned());
    }
    if configured.is_absolute() {
        Ok(configured)
    } else {
        Ok(workspace_root()?.join(configured))
    }
}

/// Reads one scalar value from a section of the quality configuration.
pub(super) fn quality_value(section: &str, key: &str) -> Result<String, String> {
    let configuration_path = workspace_root()?.join(constants::QUALITY_GATES_FILE);
    let configuration = fs::read_to_string(&configuration_path)
        .map_err(|error| format!("could not read {}: {error}", configuration_path.display()))?;
    quality_value_from(&configuration, section, key)
        .ok_or_else(|| format!("quality configuration has no [{section}] {key} value"))
}

/// Extracts one scalar value from a simple TOML section without interpreting it.
pub(super) fn quality_value_from(configuration: &str, section: &str, key: &str) -> Option<String> {
    let heading = format!("[{section}]");
    let mut selected = false;
    for line in configuration.lines().map(str::trim) {
        if line.starts_with('[') {
            selected = line == heading;
            continue;
        }
        if !selected || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (candidate, value) = line.split_once('=')?;
        if candidate.trim() == key {
            return Some(value.trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// Reads one quoted-string array from the quality configuration.
pub(super) fn quality_array(section: &str, key: &str) -> Result<Vec<String>, String> {
    let configuration = read_workspace_text(&workspace_root()?, constants::QUALITY_GATES_FILE)?;
    configuration_array_from(&configuration, section, key)
}

/// Reads one nonempty quoted-string array from a named configuration section.
pub(super) fn configuration_array_from(
    configuration: &str,
    section: &str,
    key: &str,
) -> Result<Vec<String>, String> {
    let heading = format!("[{section}]");
    let mut selected = false;
    let mut collecting = false;
    let mut value = String::new();
    for line in configuration.lines().map(str::trim) {
        if !collecting && line.starts_with('[') {
            selected = line == heading;
            continue;
        }
        if !selected || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if collecting {
            value.push_str(line);
        } else {
            let Some((candidate, first)) = line.split_once('=') else {
                continue;
            };
            if candidate.trim() != key {
                continue;
            }
            value.push_str(first.trim());
            collecting = true;
        }
        if value.ends_with(']') {
            break;
        }
    }
    if !collecting {
        return Err(format!("configuration [{section}] has no {key} value"));
    }
    let inner = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("configuration [{section}] {key} must be an array"))?;
    let values = inner
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("configuration [{section}] {key} has an invalid item"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.is_empty() {
        Err(format!("configuration [{section}] {key} array is empty"))
    } else {
        Ok(values)
    }
}

/// Returns the durable current test-minimum profile.
pub(super) fn active_test_profile() -> &'static str {
    constants::CURRENT_TEST_PROFILE
}

/// Reads one test-minimum configuration owned by the workspace.
pub(super) fn test_minimums(profile: &str) -> Result<BTreeMap<String, usize>, String> {
    let configuration_path = workspace_root()?.join(constants::TEST_SUITES_FILE);
    let configuration = fs::read_to_string(&configuration_path)
        .map_err(|error| format!("could not read {}: {error}", configuration_path.display()))?;
    let mut minimums = BTreeMap::new();
    for (name, value) in configuration_section(&configuration, &format!("{profile}.minimum"))? {
        let minimum = value
            .parse::<usize>()
            .map_err(|error| format!("invalid test minimum for {name}: {error}"))?;
        minimums.insert(name, minimum);
    }

    if minimums.is_empty() {
        Err(format!("{profile} test-minimum configuration is empty"))
    } else {
        Ok(minimums)
    }
}
