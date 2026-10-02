// SPDX-License-Identifier: Apache-2.0

//! Canonical project vocabulary validation over an already captured closure.

use crate::{CapturedProject, project_capture::scan_vocabulary_requirements};
use neutral_core::StructuralLimits;
use neutral_vocabulary::{
    ProjectVocabulary, VocabularyError, VocabularyLimits, validate_project_bundle,
};
use std::collections::BTreeMap;

/// The complete validated vocabulary set; aliases remain source-local only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectVocabularySet {
    /// Validated contracts in canonical identity order.
    vocabularies: BTreeMap<String, ProjectVocabulary>,
    /// Module-local aliases kept separate from semantic contracts.
    aliases: BTreeMap<String, BTreeMap<String, String>>,
}

impl ProjectVocabularySet {
    /// Returns canonical identities and locked revisions in identity order.
    #[must_use]
    pub fn vocabularies(&self) -> &BTreeMap<String, ProjectVocabulary> {
        &self.vocabularies
    }

    /// Resolves a module-local alias to its canonical locked contract.
    #[must_use]
    pub fn resolve(&self, module_id: &str, alias: &str) -> Option<&ProjectVocabulary> {
        let identity = self.aliases.get(module_id)?.get(alias)?;
        self.vocabularies.get(identity)
    }
}

/// Validation failure before any partially validated vocabulary set is published.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectVocabularyValidationError {
    /// A captured source requirement was malformed despite capture validation.
    InvalidRequirement,
    /// A captured bundle violated the exact lock or closed v1 schema.
    InvalidBundle(VocabularyError),
}

/// Validates every captured v1 bundle and source-local alias against exact locks.
///
/// # Errors
///
/// Rejects a malformed requirement or any bundle with no partial result.
pub fn validate_project_vocabularies(
    captured: &CapturedProject,
) -> Result<ProjectVocabularySet, ProjectVocabularyValidationError> {
    let values = captured.limits().values();
    let structural = StructuralLimits::new(values.vocabulary_bytes_per_unit, 1)
        .map_err(|_| ProjectVocabularyValidationError::InvalidRequirement)?
        .with_declarations(values.declarations)
        .map_err(|_| ProjectVocabularyValidationError::InvalidRequirement)?;
    let limits = VocabularyLimits::from_structural(structural);
    let mut vocabularies = BTreeMap::new();
    for input in captured.vocabularies() {
        let validated = validate_project_bundle(input.bytes(), input.lock(), limits)
            .map_err(ProjectVocabularyValidationError::InvalidBundle)?;
        vocabularies.insert(validated.identity().to_owned(), validated);
    }
    let mut aliases = BTreeMap::new();
    for source in captured.sources() {
        let text = std::str::from_utf8(source.bytes())
            .map_err(|_| ProjectVocabularyValidationError::InvalidRequirement)?;
        let mut local = BTreeMap::new();
        let requirements = scan_vocabulary_requirements(text)
            .map_err(|_| ProjectVocabularyValidationError::InvalidRequirement)?;
        for (identity, alias) in requirements {
            if !vocabularies.contains_key(&identity) || local.insert(alias, identity).is_some() {
                return Err(ProjectVocabularyValidationError::InvalidRequirement);
            }
        }
        aliases.insert(source.module_id().to_owned(), local);
    }
    Ok(ProjectVocabularySet {
        vocabularies,
        aliases,
    })
}

#[cfg(test)]
#[path = "../tests/project_vocabulary/mod.rs"]
mod tests;
