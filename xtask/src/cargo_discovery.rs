// SPDX-License-Identifier: Apache-2.0

//! Cargo-owned workspace, output-path, and resolved dependency discovery.

use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, NodeDep, Package, PackageId};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// Queries the selected Cargo executable without modifying its lockfile or using the network.
pub(super) fn metadata(root: &Path, dependencies: bool) -> Result<Metadata, String> {
    let mut options = vec!["--locked".to_owned(), "--offline".to_owned()];
    if dependencies {
        let compiler = crate::command_output(&crate::configuration::rustc_command()?, &["-vV"])?;
        let host = compiler
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .ok_or("Rust compiler did not identify its host platform")?;
        options.extend(["--filter-platform".to_owned(), host.to_owned()]);
    }
    let mut command = MetadataCommand::new();
    command
        .cargo_path(crate::configuration::cargo_command()?)
        .current_dir(root)
        .manifest_path(root.join(crate::constants::WORKSPACE_MANIFEST_FILE))
        .other_options(options);
    if !dependencies {
        command.no_deps();
    }
    let metadata = command.exec().map_err(|error| format!("Cargo discovery failed (run `cargo fetch --locked` if dependencies are missing): {error}"))?;
    if metadata.workspace_root.as_std_path() != root {
        return Err(format!(
            "Cargo resolved a different workspace: {}",
            metadata.workspace_root
        ));
    }
    Ok(metadata)
}

/// Returns Cargo's resolved member manifests, including globbed and root-package members.
pub(super) fn manifests(root: &Path) -> Result<Vec<PathBuf>, String> {
    let metadata = metadata(root, false)?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("could not resolve workspace root: {error}"))?;
    let mut manifests = BTreeSet::new();
    for package in metadata.workspace_packages() {
        let manifest = package
            .manifest_path
            .canonicalize()
            .map_err(|error| format!("could not resolve {}: {error}", package.manifest_path))?;
        if !manifest.starts_with(&canonical_root) || !manifests.insert(manifest) {
            return Err(format!(
                "workspace package is external or repeated: {}",
                package.name
            ));
        }
    }
    if manifests.is_empty() {
        return Err("Cargo workspace has no packages".to_owned());
    }
    Ok(manifests.into_iter().collect())
}

/// Finds a workspace package by its declared name, not its directory or dependency alias.
pub(super) fn package<'a>(metadata: &'a Metadata, name: &str) -> Result<&'a Package, String> {
    metadata
        .workspace_packages()
        .into_iter()
        .find(|package| package.name.as_str() == name)
        .ok_or_else(|| format!("Cargo workspace has no package {name}"))
}

/// Resolves a graph package ID to its real declared package name.
fn package_name(metadata: &Metadata, id: &PackageId) -> Result<String, String> {
    metadata
        .packages
        .iter()
        .find(|package| &package.id == id)
        .map(|package| package.name.to_string())
        .ok_or_else(|| format!("Cargo graph references an unknown package {id}"))
}

/// Matches Cargo traversal: normal edges, optionally build edges and root-only development edges.
fn selected(dependency: &NodeDep, all: bool, root: bool) -> bool {
    dependency.dep_kinds.iter().any(|kind| {
        kind.kind == DependencyKind::Normal
            || (all && (kind.kind != DependencyKind::Development || root))
    })
}

/// Returns the direct normal dependency names from one resolved Cargo graph.
pub(super) fn direct_dependencies(
    metadata: &Metadata,
    name: &str,
) -> Result<BTreeSet<String>, String> {
    let id = &package(metadata, name)?.id;
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or("Cargo metadata has no resolved dependency graph")?;
    let node = resolve
        .nodes
        .iter()
        .find(|node| &node.id == id)
        .ok_or_else(|| format!("Cargo graph has no node for {name}"))?;
    node.deps
        .iter()
        .filter(|dependency| selected(dependency, false, true))
        .map(|dependency| package_name(metadata, &dependency.pkg))
        .collect()
}

/// Traverses a resolved dependency closure once per ID, retaining the root package.
pub(super) fn closure(
    metadata: &Metadata,
    name: &str,
    all: bool,
) -> Result<BTreeSet<String>, String> {
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or("Cargo metadata has no resolved dependency graph")?;
    let root = package(metadata, name)?.id.clone();
    let mut pending = vec![root.clone()];
    let mut visited = BTreeSet::new();
    let mut names = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        names.insert(package_name(metadata, &id)?);
        let node = resolve
            .nodes
            .iter()
            .find(|node| node.id == id)
            .ok_or_else(|| format!("Cargo graph has no node for {id}"))?;
        pending.extend(
            node.deps
                .iter()
                .filter(|dependency| selected(dependency, all, id == root))
                .map(|dependency| dependency.pkg.clone()),
        );
    }
    Ok(names)
}

/// Finds a benchmark executable through Cargo's typed build-message stream.
pub(super) fn benchmark_executable(output: &str, target: &str) -> Result<String, String> {
    for message in cargo_metadata::Message::parse_stream(output.as_bytes()) {
        let message = message.map_err(|error| format!("invalid Cargo build message: {error}"))?;
        if let cargo_metadata::Message::CompilerArtifact(artifact) = message
            && artifact.target.name == target
            && let Some(executable) = artifact.executable
        {
            return Ok(executable.to_string());
        }
    }
    Err(format!(
        "Cargo did not identify the benchmark executable for {target}"
    ))
}

#[cfg(test)]
#[path = "../tests/unit/cargo_discovery.rs"]
mod tests;
