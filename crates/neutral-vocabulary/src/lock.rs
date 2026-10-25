// SPDX-License-Identifier: Apache-2.0

//! Exact captured vocabulary identity locks and digest checks.

use crate::{VocabularyError, schema};
use neutral_core::VocabularyContentDigest;

/// Exact host-supplied lock facts for one captured vocabulary bundle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyLock {
    /// Expected logical vocabulary identity.
    pub(crate) identity: String,
    /// Expected exact vocabulary release.
    pub(crate) version: String,
    /// Expected bundle encoding version.
    pub(crate) encoding_version: String,
    /// Expected logical schema version.
    pub(crate) schema_version: String,
    /// Expected exact captured-byte digest.
    pub(crate) content_digest: VocabularyContentDigest,
    /// Exact normalized structural feature set.
    pub(crate) required_features: Vec<String>,
}

impl VocabularyLock {
    /// Creates validated immutable lock facts without acquiring bundle content.
    ///
    /// # Errors
    ///
    /// Returns a bounded lock error for invalid identities, versions, feature
    /// IDs, or duplicate features.
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        encoding_version: impl Into<String>,
        schema_version: impl Into<String>,
        content_digest: VocabularyContentDigest,
        mut required_features: Vec<String>,
    ) -> Result<Self, VocabularyError> {
        let identity = identity.into();
        let version = version.into();
        let encoding_version = encoding_version.into();
        let schema_version = schema_version.into();
        if !schema::is_upper_name(&identity) || schema::is_protected_name(&identity) {
            return Err(VocabularyError::InvalidIdentity);
        }
        if !schema::is_exact_release_version(&version) {
            return Err(VocabularyError::InvalidVersion);
        }
        if required_features
            .iter()
            .any(|feature| !schema::is_feature_id(feature))
        {
            return Err(VocabularyError::InvalidFeatureId);
        }
        required_features.sort();
        if required_features.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(VocabularyError::DuplicateFeature);
        }
        Ok(Self {
            identity,
            version,
            encoding_version,
            schema_version,
            content_digest,
            required_features,
        })
    }

    /// Returns the expected exact content digest.
    #[must_use]
    pub const fn content_digest(&self) -> VocabularyContentDigest {
        self.content_digest
    }

    /// Returns the exact locked logical identity.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Returns the exact locked release version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the exact locked encoding version.
    #[must_use]
    pub fn encoding_version(&self) -> &str {
        &self.encoding_version
    }

    /// Returns the exact locked logical schema version.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Returns the exact normalized locked feature set.
    #[must_use]
    pub fn required_features(&self) -> &[String] {
        &self.required_features
    }
}

impl neutral_core::allocation::TryClone for VocabularyLock {
    /// Copies an already validated lock without revalidation or allocating sorting work.
    fn try_clone(&self) -> Result<Self, neutral_core::allocation::AllocationError> {
        Ok(Self {
            identity: self.identity.try_clone()?,
            version: self.version.try_clone()?,
            encoding_version: self.encoding_version.try_clone()?,
            schema_version: self.schema_version.try_clone()?,
            content_digest: self.content_digest,
            required_features: self.required_features.try_clone()?,
        })
    }
}
