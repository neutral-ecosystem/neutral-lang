// SPDX-License-Identifier: Apache-2.0

//! Exact headless consumer negotiation, separate from source/compiler and authoring activation.

use super::{ConsumerError as E, ConsumerLimits, validation::Budget};
use crate::composition::ValidatedCompositionProject;
use neutral_core::{CancellationToken, profile::ProfileAvailability};
use neutral_ir::composition::profile::{self, CompositionProfileDescriptor};

/// Frozen reader consumer protocol, not a Cargo/package version.
pub const CONSUMER_PROFILE: &str = "neutral.consumer/1";
/// Implemented typed entry discovery capability.
pub const ENTRY_DISCOVERY_FEATURE: &str = "entry-discovery-v1";
/// Implemented typed value/type member selection capability.
pub const MEMBER_SELECTORS_FEATURE: &str = "member-selectors-v1";
/// Implemented bounded safe value-origin capability.
pub const VALUE_ATTRIBUTION_FEATURE: &str = "value-attribution-v1";
/// Exact sorted required consumer feature set.
pub const CONSUMER_FEATURES: &[&str] = &[
    ENTRY_DISCOVERY_FEATURE,
    MEMBER_SELECTORS_FEATURE,
    VALUE_ATTRIBUTION_FEATURE,
];

/// Explicit protocol selection; required features are never optional hints.
#[derive(Clone, Copy)]
pub struct ConsumerCompatibilityRequest<'a> {
    /// Exact consumer profile.
    pub profile: &'a str,
    /// Exact source/capture/project/view/wire/identity tuple and project features.
    pub artifact: CompositionProfileDescriptor<'a>,
    /// Sorted unique bundle schema capabilities; empty is valid for source-only artifacts.
    pub vocabulary_schemas: &'a [&'a str],
    /// Exact sorted required consumer features.
    pub required_features: &'a [&'a str],
    /// Optional separately negotiated authoring profile; none is sufficient for headless use.
    pub authoring_profile: Option<&'a str>,
}

/// Immutable public implementation capabilities, never a claim of runtime effects or JSON transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumerCompatibility {
    /// Exact supported consumer profile.
    pub profile: &'static str,
    /// Exact supported captured-project tuple.
    pub artifact: CompositionProfileDescriptor<'static>,
    /// Independently supported native/adapted bundle schemas, not wildcard revisions.
    pub vocabulary_schemas: &'static [&'static str],
    /// Implemented public reader features.
    pub features: &'static [&'static str],
    /// Typed API availability, distinct from standalone source and authoring availability.
    pub availability: ProfileAvailability,
    /// Supported optional authoring profile; not available at this boundary yet.
    pub authoring_profile: Option<&'static str>,
}

/// Implemented typed boundary catalogue; authoring and executable capabilities are independent.
const COMPATIBILITIES: &[ConsumerCompatibility] = &[ConsumerCompatibility {
    profile: CONSUMER_PROFILE,
    artifact: profile::COMPOSITION_PROFILE,
    vocabulary_schemas: &[
        neutral_vocabulary::PROJECT_VOCABULARY_SCHEMA_VERSION,
        profile::VOCABULARY_SCHEMA_VERSION,
    ],
    features: CONSUMER_FEATURES,
    availability: ProfileAvailability::Available,
    authoring_profile: None,
}];

/// Returns exact supported headless tuples/features without package-version inference or allocation.
#[must_use]
pub const fn consumer_compatibilities() -> &'static [ConsumerCompatibility] {
    COMPATIBILITIES
}

/// Negotiates the exact typed public protocol before interpretation or result publication.
///
/// # Errors
/// Rejects unavailable tuples/authoring, unknown or missing features, malformed lists, controls or cancellation.
pub fn negotiate_consumer(
    request: &ConsumerCompatibilityRequest<'_>,
    limits: ConsumerLimits,
    cancel: &CancellationToken,
) -> Result<&'static ConsumerCompatibility, E> {
    let limits = limits.effective(profile::MAX_VIEW_WORK)?;
    let mut budget = Budget::new(limits, cancel);
    budget.text(request.profile)?;
    let a = &request.artifact;
    for text in [
        a.source_profile,
        a.capture_profile,
        a.project_ir,
        a.view,
        a.result,
        a.encoding,
        a.identity,
    ] {
        budget.text(text)?;
    }
    budget.list(a.required_features, limits.features)?;
    budget.list(request.required_features, limits.features)?;
    budget.list(request.vocabulary_schemas, limits.features)?;
    if let Some(authoring) = request.authoring_profile {
        budget.text(authoring)?;
    }
    let supported = &COMPATIBILITIES[0];
    if request.profile != supported.profile
        || request.artifact != supported.artifact
        || request.required_features != supported.features
        || request.authoring_profile != supported.authoring_profile
        || request
            .vocabulary_schemas
            .iter()
            .any(|schema| !supported.vocabulary_schemas.contains(schema))
    {
        return Err(E::Unsupported);
    }
    budget.work(1)?;
    Ok(supported)
}

impl ConsumerCompatibility {
    /// Checks decoded authority against an already negotiated request; no caller label grants authority.
    ///
    /// # Errors
    /// Fails for incompatible actual tuple/bundle schemas, controls or cancellation.
    pub fn validate_project(
        &self,
        project: &ValidatedCompositionProject,
        request: &ConsumerCompatibilityRequest<'_>,
        limits: ConsumerLimits,
        cancel: &CancellationToken,
    ) -> Result<(), E> {
        let supported = negotiate_consumer(request, limits, cancel)?;
        if self != supported || project.profile_facts() != supported.artifact {
            return Err(E::Unsupported);
        }
        let limits = limits.effective(project.effective_composition_limits().work)?;
        let mut budget = Budget::new(limits, cancel);
        for bundle in &project.complete_ir().vocabularies {
            let schema = bundle.identity.schema_version();
            budget.work(schema.len() as u64 + 1)?;
            if !request.vocabulary_schemas.contains(&schema) {
                return Err(E::Unsupported);
            }
        }
        budget.work(1)
    }
}
