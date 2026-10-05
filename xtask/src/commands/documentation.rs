// SPDX-License-Identifier: Apache-2.0

//! commands / documentation responsibilities for repository automation.

use crate::{
    Command, Path, RUSTDOC_HEADER_TEMPLATE, RUSTDOC_INDEX_TEMPLATE, cargo_command, cargo_discovery,
    cargo_target_directory, constants, env, fs, output, workspace_root,
};

/// Builds every workspace crate's local API documentation.
pub(crate) fn documentation() -> Result<(), String> {
    run_rustdoc()?;
    let metadata = serde_json::to_string(&cargo_discovery::metadata(&workspace_root()?, false)?)
        .map_err(|error| format!("could not serialize Cargo metadata: {error}"))?;
    let index = render_rustdoc_index(&metadata)?;
    let output_directory = cargo_target_directory()?.join("doc");
    fs::create_dir_all(&output_directory).map_err(|error| {
        format!(
            "could not create rustdoc directory {}: {error}",
            output_directory.display()
        )
    })?;
    let output_path = output_directory.join(constants::RUSTDOC_INDEX_FILE);
    fs::write(&output_path, index)
        .map_err(|error| format!("could not write {}: {error}", output_path.display()))?;
    copy_documentation_assets(&workspace_root()?, &output_directory)?;
    output::file("workspace documentation", &output_path);
    Ok(())
}

/// Copies repository-owned brand assets into generated documentation output.
///
/// # Errors
///
/// Returns an error when a required source asset is missing or cannot be copied.
pub(crate) fn copy_documentation_assets(
    root: &Path,
    output_directory: &Path,
) -> Result<(), String> {
    let source_directory = root.join(constants::ASSET_DIRECTORY);
    let destination_directory = output_directory.join(constants::DOCUMENTATION_ASSET_DIRECTORY);
    fs::create_dir_all(&destination_directory).map_err(|error| {
        format!(
            "could not create documentation asset directory {}: {error}",
            destination_directory.display()
        )
    })?;
    for filename in constants::DOCUMENTATION_ASSET_FILES {
        let source = source_directory.join(filename);
        let destination = destination_directory.join(filename);
        if !source.is_file() {
            return Err(format!(
                "required documentation asset is missing: {}",
                source.display()
            ));
        }
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "could not copy documentation asset {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
    }
    Ok(())
}

/// Runs workspace rustdoc with the shared navigation header on every HTML page.
pub(crate) fn run_rustdoc() -> Result<(), String> {
    let header_path = workspace_root()?.join(constants::RUSTDOC_HEADER_FILE);
    if !header_path.is_file() {
        return Err(format!(
            "rustdoc navigation header is missing: {}",
            header_path.display()
        ));
    }
    let header_path = header_path
        .to_str()
        .ok_or_else(|| "rustdoc navigation header path is not valid UTF-8".to_owned())?;
    let mut rustdoc_flags = env::var(constants::CARGO_ENCODED_RUSTDOCFLAGS).unwrap_or_default();
    if !rustdoc_flags.is_empty() {
        rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    }
    rustdoc_flags.push_str(constants::RUSTDOC_HTML_HEADER_FLAG);
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str(header_path);
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str("-D");
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str("warnings");
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str("--cfg");
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str(&rustdoc_header_configuration());

    let arguments = ["doc", "--workspace", "--no-deps"];
    let cargo = cargo_command()?;
    output::invocation(&cargo, &arguments);
    let status = Command::new(&cargo)
        .current_dir(workspace_root()?)
        .env(constants::CARGO_TERM_COLOR_ENV, output::child_color())
        .env(constants::CARGO_ENCODED_RUSTDOCFLAGS, rustdoc_flags)
        .args(arguments)
        .status()
        .map_err(|error| format!("could not run {} {}: {error}", cargo, arguments.join(" ")))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} {} failed with {status}", cargo, arguments.join(" ")))
}

/// Derives a stable non-security cache token from the shared rustdoc header.
pub(crate) fn rustdoc_header_configuration() -> String {
    let mut hash = constants::RUSTDOC_HEADER_HASH_OFFSET;
    for byte in RUSTDOC_HEADER_TEMPLATE.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(constants::RUSTDOC_HEADER_HASH_PRIME);
    }
    format!("{}_{hash:016x}", constants::RUSTDOC_HEADER_CFG_PREFIX)
}

/// Embeds Cargo metadata safely into the workspace rustdoc index template.
pub(crate) fn render_rustdoc_index(metadata: &str) -> Result<String, String> {
    if !RUSTDOC_INDEX_TEMPLATE.contains(constants::CARGO_METADATA_PLACEHOLDER) {
        return Err("rustdoc index template has no Cargo metadata placeholder".to_owned());
    }
    let safe_metadata = metadata.replace('<', "\\u003c");
    Ok(RUSTDOC_INDEX_TEMPLATE.replacen(constants::CARGO_METADATA_PLACEHOLDER, &safe_metadata, 1))
}
