// SPDX-License-Identifier: Apache-2.0

//! Central selection of the immutable inherited language conformance bundle.

use crate::{configuration_value, constants, read_workspace_text, validate_semver};
use std::path::Path;

/// One checked immutable bundle selected independently of the package version.
pub(crate) struct ReleasedBundle {
    /// Relative root of the exact inherited release corpus.
    directory: String,
}

impl ReleasedBundle {
    /// Reads and validates the repository's explicit inherited-release selection.
    pub(crate) fn load(root: &Path) -> Result<Self, String> {
        let configuration = read_workspace_text(root, constants::CONFORMANCE_CONFIG_FILE)?;
        Self::from_configuration(root, &configuration)
    }

    /// Validates one selected release without deriving it from Cargo metadata.
    pub(crate) fn from_configuration(root: &Path, configuration: &str) -> Result<Self, String> {
        if configuration_value(configuration, "schema_version").as_deref() != Some("1") {
            return Err("unsupported conformance selection schema".to_owned());
        }
        let release = configuration_value(configuration, "inherited_release")
            .ok_or_else(|| "conformance selection has no inherited_release".to_owned())?;
        let version = release
            .strip_prefix('v')
            .ok_or_else(|| "inherited release must be a v-prefixed SemVer tag".to_owned())?;
        validate_semver(version)?;
        let directory = format!("conformance/releases/{release}");
        if !root.join(&directory).is_dir() {
            return Err(format!(
                "inherited conformance bundle is missing: {directory}"
            ));
        }
        Ok(Self { directory })
    }

    /// Resolves one known bundle member below the selected immutable root.
    pub(crate) fn member(&self, relative: &str) -> String {
        format!("{}/{relative}", self.directory)
    }
}
