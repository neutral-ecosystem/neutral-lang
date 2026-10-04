// SPDX-License-Identifier: Apache-2.0

//! Repository configuration access and active test-minimum selection.

use super::{
    BTreeMap, Component, Path, PathBuf, constants, env, read_workspace_text, workspace_root,
};

pub(super) use crate::configuration_models::{Automation, RepositoryDirectory, TestRunner};
use crate::configuration_models::{Layout, TestMinimumProfile};
use serde::de::DeserializeOwned;

/// Deserializes TOML with its owning file/context in parser diagnostics.
pub(super) fn parse<T: DeserializeOwned>(content: &str, label: &str) -> Result<T, String> {
    toml::from_str(content).map_err(|error| format!("invalid {label}: {error}"))
}

/// Reads a workspace-owned typed configuration.
pub(super) fn read<T: DeserializeOwned>(root: &Path, relative: &str) -> Result<T, String> {
    parse(&read_workspace_text(root, relative)?, relative)
}

/// Checks a configuration schema independently of package and language versions.
pub(super) fn require_schema(schema: u32, context: &str) -> Result<(), String> {
    if schema != constants::CONFIG_SCHEMA_VERSION {
        return Err(format!("unsupported {context} schema: {schema}"));
    }
    Ok(())
}

/// Loads the closed automation settings with fail-closed execution defaults.
pub(super) fn automation() -> Result<Automation, String> {
    let config: Automation = read(&workspace_root()?, constants::AUTOMATION_CONFIG_FILE)?;
    require_schema(config.schema_version, "automation configuration")?;
    if !is_safe_relative_path(Path::new(&config.output.results_root))
        || !is_safe_relative_path(Path::new(&config.quality.evidence_root))
        || config.quality.advisory_max_age_seconds == 0
        || [&config.tools.cargo, &config.tools.rustc]
            .iter()
            .any(|command| command.trim().is_empty() || command.chars().any(char::is_control))
    {
        return Err("automation settings require safe relative output paths, nonempty executables, and positive advisory freshness".to_owned());
    }
    if !is_safe_relative_path(Path::new(&config.testing.config))
        || config.testing.profile.is_empty()
        || config.testing.ci_profile.is_empty()
    {
        return Err(
            "test configuration requires a safe relative path and nonempty profiles".to_owned(),
        );
    }
    Ok(config)
}

/// Selects an exact dotted section from a parsed document.
fn section<'a>(document: &'a toml::Table, name: &str) -> Option<&'a toml::Table> {
    let mut table = document;
    if !name.is_empty() {
        for component in name.split('.') {
            table = table.get(component)?.as_table()?;
        }
    }
    Some(table)
}

/// Converts parsed scalar values to the existing policy accessor representation.
fn scalar(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(text) => Some(text.clone()),
        toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => Some(value.to_string()),
        toml::Value::Array(_) => serde_json::to_string(value).ok(),
        toml::Value::Table(_) => None,
    }
}

/// Accepts only nonempty paths composed of normal workspace-relative components.
pub(super) fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Reads the closed ownership inventory through typed TOML.
pub(super) fn repository_directories(root: &Path) -> Result<Vec<RepositoryDirectory>, String> {
    let layout: Layout = read(root, constants::REPOSITORY_LAYOUT_FILE)?;
    require_schema(layout.schema_version, "repository layout")?;
    if layout.directory.is_empty() {
        return Err("repository layout has no directory entries".to_owned());
    }
    for entry in &layout.directory {
        if [&entry.path, &entry.readme, &entry.owner, &entry.lifecycle]
            .iter()
            .any(|value| value.is_empty())
        {
            return Err("repository directory has an empty required field".to_owned());
        }
    }
    Ok(layout.directory)
}

/// Reads section values through the standard TOML parser, not line splitting.
pub(super) fn configuration_section(
    content: &str,
    name: &str,
) -> Result<Vec<(String, String)>, String> {
    let document: toml::Table = parse(content, "configuration")?;
    let Some(table) = section(&document, name) else {
        return Ok(Vec::new());
    };
    table
        .iter()
        .map(|(key, value)| {
            scalar(value)
                .map(|value| (key.clone(), value))
                .ok_or_else(|| format!("configuration [{name}] {key} is not a scalar"))
        })
        .collect()
}

/// Reads a root scalar without inheriting values from nested sections.
pub(super) fn configuration_value(content: &str, key: &str) -> Option<String> {
    quality_value_from(content, "", key)
}

/// Reads a typed automation setting through its owning structure.
pub(super) fn automation_value(name: &str, key: &str) -> Result<String, String> {
    let config = automation()?;
    let value = match (name, key) {
        ("tools", "cargo") => config.tools.cargo,
        ("tools", "rustc") => config.tools.rustc,
        ("output", "results_root") => config.output.results_root,
        ("quality", "evidence_root") => config.quality.evidence_root,
        ("quality", "advisory_max_age_seconds") => {
            config.quality.advisory_max_age_seconds.to_string()
        }
        _ => {
            return Err(format!(
                "automation configuration has no [{name}] {key} value"
            ));
        }
    };
    if value.is_empty() {
        return Err(format!("automation configuration [{name}] {key} is empty"));
    }
    Ok(value)
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

/// Resolves Cargo's actual target directory, including configuration-file overrides.
pub(super) fn cargo_target_directory() -> Result<PathBuf, String> {
    Ok(crate::cargo_discovery::metadata(&workspace_root()?, false)?
        .target_directory
        .into_std_path_buf())
}

/// Reads one required quality setting with parser errors retained.
pub(super) fn quality_value(name: &str, key: &str) -> Result<String, String> {
    let content = read_workspace_text(&workspace_root()?, constants::QUALITY_GATES_FILE)?;
    let document: toml::Table = parse(&content, constants::QUALITY_GATES_FILE)?;
    section(&document, name)
        .and_then(|table| table.get(key))
        .and_then(scalar)
        .ok_or_else(|| format!("quality configuration [{name}] has no scalar {key}"))
}

/// Compatibility accessor backed by parsed TOML rather than raw text.
pub(super) fn quality_value_from(content: &str, name: &str, key: &str) -> Option<String> {
    let document = toml::from_str::<toml::Table>(content).ok()?;
    scalar(section(&document, name)?.get(key)?)
}

/// Reads one quoted-string array from the quality configuration.
pub(super) fn quality_array(section: &str, key: &str) -> Result<Vec<String>, String> {
    let configuration = read_workspace_text(&workspace_root()?, constants::QUALITY_GATES_FILE)?;
    configuration_array_from(&configuration, section, key)
}

/// Reads escaped/commented/multiline string arrays without splitting comma-containing values.
pub(super) fn configuration_array_from(
    content: &str,
    name: &str,
    key: &str,
) -> Result<Vec<String>, String> {
    let document: toml::Table = parse(content, "configuration")?;
    let values = section(&document, name)
        .and_then(|table| table.get(key))
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("configuration [{name}] {key} must be an array"))?;
    let values = values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("configuration [{name}] {key} has an invalid item"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.is_empty() {
        return Err(format!("configuration [{name}] {key} array is empty"));
    }
    Ok(values)
}

/// Returns the durable current test-minimum profile.
pub(super) fn active_test_profile() -> &'static str {
    constants::CURRENT_TEST_PROFILE
}

/// Reads typed test counts with extensible named category profiles.
pub(super) fn test_minimums(profile: &str) -> Result<BTreeMap<String, usize>, String> {
    let profiles: BTreeMap<String, TestMinimumProfile> =
        read(&workspace_root()?, constants::TEST_SUITES_FILE)?;
    let profile = profiles
        .get(profile)
        .ok_or_else(|| format!("missing {profile} test-minimum configuration"))?;
    if profile.minimum.is_empty() {
        return Err("test-minimum configuration is empty".to_owned());
    }
    Ok(profile.minimum.clone())
}

#[cfg(test)]
#[path = "../tests/unit/configuration.rs"]
mod tests;
