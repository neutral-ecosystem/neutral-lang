// SPDX-License-Identifier: Apache-2.0

//! checks / traceability responsibilities for repository automation.

use crate::{
    BTreeSet, Path, ReleasedBundle, collect_regular_files, constants, read_workspace_text,
    verify_frozen_input_digests, workspace_root,
};

/// Checks accepted IDs, completed syntax items, and fixture/oracle inventories.
pub(crate) fn check_traceability() -> Result<(), String> {
    let root = workspace_root()?;
    let bundle = ReleasedBundle::load(&root)?;
    check_traceability_bundle(
        &bundle.member("specs/REQUIREMENTS.md"),
        &bundle.member("specs/contracts/syntax.md"),
        &bundle.member("specs/contracts/syntax-checklist.md"),
        &bundle.member("specs/TRACEABILITY.md"),
        &bundle.member("conformance/manifest.toml"),
        &bundle.member("specs/fixtures"),
        &bundle.member("conformance/oracles"),
    )?;
    verify_frozen_input_digests(&root, &bundle.member("specs/contracts/freeze.toml"))
}

/// Checks the version-independent conformance inventory of an installed portable package.
pub(crate) fn check_portable_traceability_at(root: &Path) -> Result<(), String> {
    let manifest = read_workspace_text(root, constants::PORTABLE_CONFORMANCE_MANIFEST_FILE)?;
    ensure_inventory_registered(root, constants::PORTABLE_FIXTURE_DIRECTORY, &manifest)?;
    ensure_inventory_registered(root, constants::PORTABLE_ORACLE_DIRECTORY, &manifest)?;
    ensure_registered_paths_exist(root, &manifest)?;
    crate::output::pass("traceability coherence");
    Ok(())
}

/// Checks one self-contained contract, fixture, and oracle bundle.
pub(crate) fn check_traceability_bundle(
    requirements_file: &str,
    syntax_file: &str,
    checklist_file: &str,
    traceability_file: &str,
    manifest_file: &str,
    fixture_directory: &str,
    oracle_directory: &str,
) -> Result<(), String> {
    let root = workspace_root()?;
    let requirements = read_workspace_text(&root, requirements_file)?;
    let syntax = read_workspace_text(&root, syntax_file)?;
    let checklist = read_workspace_text(&root, checklist_file)?;
    let traceability = read_workspace_text(&root, traceability_file)?;
    let manifest = read_workspace_text(&root, manifest_file)?;

    let requirement_ids = contract_ids(&requirements, "NL-");
    let syntax_ids = contract_ids(&syntax, "SYN-");
    let checklist_ids = contract_ids(&checklist, "SYN-");
    ensure_ids_covered("requirements", &requirement_ids, &traceability)?;
    ensure_ids_covered("syntax", &syntax_ids, &traceability)?;
    if syntax_ids != checklist_ids {
        return Err("master syntax and implementation checklist IDs differ".to_owned());
    }
    ensure_syntax_complete(syntax_file, &syntax)?;
    ensure_syntax_complete(checklist_file, &checklist)?;
    ensure_inventory_registered(&root, fixture_directory, &manifest)?;
    ensure_inventory_registered(&root, oracle_directory, &manifest)?;
    ensure_registered_paths_exist(&root, &manifest)?;

    crate::output::pass("traceability coherence");
    Ok(())
}

/// Extracts unique contract identifiers beginning with `prefix`.
pub(crate) fn contract_ids(content: &str, prefix: &str) -> BTreeSet<String> {
    content
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
        .filter(|token| token.starts_with(prefix))
        .map(str::to_owned)
        .collect()
}

/// Requires every accepted identifier to occur in the evidence index.
pub(crate) fn ensure_ids_covered(
    category: &str,
    identifiers: &BTreeSet<String>,
    traceability: &str,
) -> Result<(), String> {
    if identifiers.is_empty() {
        return Err(format!("{category} contains no contract identifiers"));
    }
    let missing = identifiers
        .iter()
        .filter(|identifier| !traceability.contains(identifier.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "traceability is missing {category} IDs: {}",
            missing.join(", ")
        ))
    }
}

/// Rejects an unchecked master syntax item after traceability closure.
pub(crate) fn ensure_syntax_complete(path: &str, content: &str) -> Result<(), String> {
    let unchecked = content
        .lines()
        .filter(|line| line.trim_start().starts_with("- [ ]") && line.contains("SYN-"))
        .collect::<Vec<_>>();
    if unchecked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{path} contains {} unchecked syntax items",
            unchecked.len()
        ))
    }
}

/// Requires every normative file beneath `directory` to occur in the manifest.
pub(crate) fn ensure_inventory_registered(
    root: &Path,
    directory: &str,
    manifest: &str,
) -> Result<(), String> {
    let mut files = Vec::new();
    collect_regular_files(&root.join(directory), &mut files)?;
    let missing = files
        .iter()
        .filter_map(|path| path.strip_prefix(root).ok())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .filter(|path| {
            matches!(
                Path::new(path).extension().and_then(|value| value.to_str()),
                Some("neu" | "json" | "toml")
            )
        })
        .filter(|path| !manifest.contains(path))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "conformance manifest omits normative files: {}",
            missing.join(", ")
        ))
    }
}

/// Requires every workspace-relative path registered by the manifest to exist.
pub(crate) fn ensure_registered_paths_exist(root: &Path, manifest: &str) -> Result<(), String> {
    let missing = manifest
        .split('"')
        .filter(|value| {
            value.starts_with("conformance/releases/") || value.starts_with("portable/")
        })
        .filter(|value| !root.join(value).is_file())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "manifest paths do not exist: {}",
            missing.join(", ")
        ))
    }
}
