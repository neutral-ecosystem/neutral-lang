// SPDX-License-Identifier: Apache-2.0

//! Formatting-preserving edits to precisely selected TOML values.

use toml_edit::{DocumentMut, Item, Value};

/// Parses an editable document while preserving whitespace, comments, and key order.
pub(crate) fn parse(content: &str, label: &str) -> Result<DocumentMut, String> {
    content
        .parse()
        .map_err(|error| format!("invalid {label}: {error}"))
}

/// Replaces an existing string only after checking the expected value, retaining its decoration.
pub(crate) fn replace_string(
    item: &mut Item,
    expected: &str,
    requested: &str,
    context: &str,
) -> Result<(), String> {
    let value = item
        .as_value_mut()
        .ok_or_else(|| format!("{context} must be a string"))?;
    if value.as_str() != Some(expected) {
        return Err(format!("{context} has no matching value {expected}"));
    }
    let decoration = value.decor().clone();
    *value = Value::from(requested);
    *value.decor_mut() = decoration;
    Ok(())
}

/// Changes the central package version without touching dependency or contract versions.
pub(crate) fn package_version(
    content: &str,
    current: &str,
    requested: &str,
) -> Result<String, String> {
    let mut document = parse(content, "Cargo.toml")?;
    let item = document
        .get_mut("workspace")
        .and_then(|item| item.get_mut("package"))
        .and_then(|item| item.get_mut("version"))
        .ok_or("root Cargo.toml has no workspace package version")?;
    replace_string(item, current, requested, "workspace package version")?;
    Ok(document.to_string())
}

/// Updates exactly one source-free lock record per workspace package; registry namesakes are untouched.
pub(crate) fn lock_versions(
    content: &str,
    names: &std::collections::BTreeSet<String>,
    current: &str,
    requested: &str,
) -> Result<String, String> {
    let mut document = parse(content, "Cargo.lock")?;
    let packages = document
        .get_mut("package")
        .and_then(Item::as_array_of_tables_mut)
        .ok_or("Cargo.lock has no package records")?;
    let mut found = std::collections::BTreeSet::new();
    for package in packages.iter_mut() {
        let name = package
            .get("name")
            .and_then(Item::as_str)
            .ok_or("Cargo.lock package has no name")?
            .to_owned();
        if package.contains_key("source") || !names.contains(&name) {
            continue;
        }
        if !found.insert(name.clone()) {
            return Err(format!("Cargo.lock has repeated workspace package {name}"));
        }
        let item = package
            .get_mut("version")
            .ok_or_else(|| format!("Cargo.lock has no version for {name}"))?;
        replace_string(
            item,
            current,
            requested,
            &format!("Cargo.lock workspace package {name}"),
        )?;
    }
    if &found != names {
        return Err(format!(
            "Cargo.lock is missing workspace packages: {:?}",
            names.difference(&found).collect::<Vec<_>>()
        ));
    }
    Ok(document.to_string())
}

#[cfg(test)]
#[path = "../../tests/unit/manifest_updates.rs"]
mod tests;
