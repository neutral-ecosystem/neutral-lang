// SPDX-License-Identifier: Apache-2.0

//! Stable language-profile, capability, and default-limit contracts.

/// Frozen source header spelling for the independently selectable v0 profile.
pub const V0_SOURCE_PROFILE: &str = "0.1";
/// Frozen source header spelling reserved for the v1 profile.
pub const V1_SOURCE_PROFILE: &str = "1.0";
/// Stable diagnostic emitted when a recognized profile is not yet available.
pub const PROFILE_UNAVAILABLE_DIAGNOSTIC: &str = "NEU-PRO-001";
/// Stable diagnostic family prefix for profile-selection failures.
pub const PROFILE_DIAGNOSTIC_FAMILY: &str = "NEU-PRO";

/// Shared default captured-source byte ceiling.
pub const DEFAULT_SOURCE_BYTES: u64 = 16_777_216;
/// Shared default retained-diagnostic ceiling.
pub const DEFAULT_DIAGNOSTICS: u32 = 64;
/// Shared default decoded-string byte ceiling.
pub const DEFAULT_STRING_BYTES: u64 = 1_048_576;
/// Shared default exact-number digit ceiling.
pub const DEFAULT_NUMERIC_DIGITS: u64 = 1_000_000;
/// Shared default exact-number absolute-scale ceiling.
pub const DEFAULT_NUMERIC_SCALE: u64 = 1_000_000;
/// Shared default root-declaration ceiling.
pub const DEFAULT_DECLARATIONS: u64 = 100_000;
/// Shared default record-field ceiling.
pub const DEFAULT_RECORD_FIELDS: u64 = 100_000;
/// Shared default recursive-value depth ceiling.
pub const DEFAULT_NESTING_DEPTH: u64 = 128;
/// Shared default list-item ceiling.
pub const DEFAULT_LIST_ITEMS: u64 = 1_000_000;
/// Shared default recursive traversal-node ceiling.
pub const DEFAULT_TRAVERSAL_NODES: u64 = 1_000_000;

/// One exact source-language profile recognized by this release train.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LanguageProfile {
    /// Frozen, implemented v0.1 source behavior.
    V0_1,
    /// Reserved v1.0 source behavior, recognized but not implemented in Stage 1.
    V1_0,
}

impl LanguageProfile {
    /// Selects a recognized profile from an exact unescaped header value.
    #[must_use]
    pub const fn from_source_version(value: &str) -> Option<Self> {
        if const_str_eq(value, V0_SOURCE_PROFILE) {
            Some(Self::V0_1)
        } else if const_str_eq(value, V1_SOURCE_PROFILE) {
            Some(Self::V1_0)
        } else {
            None
        }
    }

    /// Returns the exact source-header version spelling.
    #[must_use]
    pub const fn source_version(self) -> &'static str {
        match self {
            Self::V0_1 => V0_SOURCE_PROFILE,
            Self::V1_0 => V1_SOURCE_PROFILE,
        }
    }
}

/// Availability of a recognized language profile in the current implementation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProfileAvailability {
    /// The profile can produce authoritative logical IR.
    Available,
    /// The profile is recognized but must fail closed before interpretation.
    Unavailable,
}

impl ProfileAvailability {
    /// Returns the stable lowercase reporting spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
        }
    }
}

/// One stable capability identifier reported for an implemented profile.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LanguageCapability {
    /// One captured source unit with no project acquisition.
    SingleSourceUnit,
    /// Immutable typed scalar, list, and record values.
    ImmutableData,
    /// Nominal record declarations and contextual record values.
    NominalRecords,
    /// Typed identity references between bindings.
    IdentityReferences,
    /// One exact captured data-only vocabulary bundle.
    CapturedVocabulary,
}

impl LanguageCapability {
    /// Returns the stable lowercase capability identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleSourceUnit => "single-source-unit",
            Self::ImmutableData => "immutable-data",
            Self::NominalRecords => "nominal-records",
            Self::IdentityReferences => "identity-references",
            Self::CapturedVocabulary => "captured-vocabulary",
        }
    }
}

/// Public immutable discovery record for one recognized language profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageProfileDescriptor {
    /// Exact source-level profile.
    profile: LanguageProfile,
    /// Current fail-open or fail-closed availability.
    availability: ProfileAvailability,
    /// Stable implemented capabilities, empty for unavailable profiles.
    capabilities: &'static [LanguageCapability],
}

impl LanguageProfileDescriptor {
    /// Returns the exact source-level profile.
    #[must_use]
    pub const fn profile(self) -> LanguageProfile {
        self.profile
    }

    /// Returns whether this profile can currently compile.
    #[must_use]
    pub const fn availability(self) -> ProfileAvailability {
        self.availability
    }

    /// Returns stable capabilities implemented for this profile.
    #[must_use]
    pub const fn capabilities(self) -> &'static [LanguageCapability] {
        self.capabilities
    }
}

/// Frozen capability list for the implemented v0.1 profile.
const V0_CAPABILITIES: &[LanguageCapability] = &[
    LanguageCapability::SingleSourceUnit,
    LanguageCapability::ImmutableData,
    LanguageCapability::NominalRecords,
    LanguageCapability::IdentityReferences,
    LanguageCapability::CapturedVocabulary,
];

/// Complete deterministic profile catalogue for public discovery.
const LANGUAGE_PROFILES: &[LanguageProfileDescriptor] = &[
    LanguageProfileDescriptor {
        profile: LanguageProfile::V0_1,
        availability: ProfileAvailability::Available,
        capabilities: V0_CAPABILITIES,
    },
    LanguageProfileDescriptor {
        profile: LanguageProfile::V1_0,
        availability: ProfileAvailability::Unavailable,
        capabilities: &[],
    },
];

/// Returns the complete deterministic profile and capability catalogue.
#[must_use]
pub const fn language_profiles() -> &'static [LanguageProfileDescriptor] {
    LANGUAGE_PROFILES
}

/// Returns the discovery record for one recognized profile.
#[must_use]
pub const fn language_profile(profile: LanguageProfile) -> &'static LanguageProfileDescriptor {
    match profile {
        LanguageProfile::V0_1 => &LANGUAGE_PROFILES[0],
        LanguageProfile::V1_0 => &LANGUAGE_PROFILES[1],
    }
}

/// Compares ASCII profile spellings in a constant context.
const fn const_str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}
