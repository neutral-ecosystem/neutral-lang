// SPDX-License-Identifier: Apache-2.0

//! Complete /3 identity partitions; framing does not replace independent reader validation.

mod captured;
mod derived;
mod interface;
mod logical;
mod trees;

pub use captured::captured_expression_closure;
pub use derived::{
    ExpressionDerivationContext, expression_artifact_identity, expression_derivation_identity,
};
pub use interface::expression_interface;
pub use logical::canonical_expression_project;

use super::{IdentityError as E, framing::Writer};
use crate::expression::profile::SemanticProfiles;

/// Frozen exact-source /3 identity domain.
const CAPTURED_DOMAIN: &str = "neutral/project-captured/v3";
/// Frozen complete logical /3 identity domain.
const LOGICAL_DOMAIN: &str = "neutral/project-logical/v3";
/// Frozen public interpretive /3 identity domain.
const INTERFACE_DOMAIN: &str = "neutral/project-interface/v3";
/// Frozen explicit processing /3 identity domain.
const DERIVATION_DOMAIN: &str = "neutral/project-derivation/v3";
/// Frozen selected-representation /3 identity domain.
const ARTIFACT_DOMAIN: &str = "neutral/project-artifact/v3";

/// Shares the frozen semantic tuple without placing required features in its body.
fn semantic_profiles(w: &mut Writer<'_>, profiles: &SemanticProfiles) -> Result<(), E> {
    if !profiles.matches_frozen() {
        return Err(E::InvalidInput);
    }
    w.frame("semantic-profiles", |w| {
        for (tag, value) in [
            ("expression", &profiles.expression),
            ("evaluator", &profiles.evaluator),
            ("expansion", &profiles.expansion),
            ("decimal", &profiles.decimal),
        ] {
            w.leaf(tag, value.as_bytes())?;
        }
        Ok(())
    })
}

/// Features are the complete exact frozen list; their containing tag is partition-specific.
fn features(w: &mut Writer<'_>, tag: &str, profiles: &SemanticProfiles) -> Result<(), E> {
    if !profiles.matches_frozen() {
        return Err(E::InvalidInput);
    }
    w.items(profiles.required_features.len())?;
    w.frame(tag, |w| {
        for feature in &profiles.required_features {
            w.leaf("feature", feature.as_bytes())?;
        }
        Ok(())
    })
}
