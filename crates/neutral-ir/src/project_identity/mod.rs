// SPDX-License-Identifier: Apache-2.0

//! Bounded, explicitly partitioned current `/3` identity transcripts, not attestations.

mod captured;
mod derived;
pub mod expression;
mod framing;

pub use captured::{CapturedIdentityInput, CapturedIdentitySource, CapturedIdentityVocabulary};
pub use derived::{ArtifactIdentityInput, ArtifactKind};
pub use expression::{
    ExpressionDerivationContext, canonical_expression_project, captured_expression_closure,
    expression_artifact_identity, expression_derivation_identity, expression_interface,
};
use neutral_core::{CancellationToken, SemanticDigest};

/// Frozen NHT envelope shared with the core framing contract.
pub use neutral_core::HASH_TRANSCRIPT_ENVELOPE as NHT_ENVELOPE;
/// Independent hard transcript byte ceiling.
pub const MAX_TRANSCRIPT_BYTES: u64 = 64 * 1024 * 1024;
/// Independent hard framed-node ceiling.
pub const MAX_TRANSCRIPT_NODES: u64 = 1_000_000;
/// Capture control order shared by the current derivation transcript.
pub const CAPTURE_LIMIT_TAGS: [&str; 14] = [
    "total-source-bytes",
    "source-bytes-per-unit",
    "source-units",
    "source-id-bytes",
    "module-id-bytes",
    "vocabulary-units",
    "vocabulary-bytes-per-unit",
    "total-vocabulary-bytes",
    "imports-per-module",
    "import-edges",
    "scc-units",
    "declarations",
    "diagnostics",
    "output-bytes",
];

/// Independent identity work limits, intersected with hard transcript ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityLimits {
    /// Maximum complete framed transcript bytes.
    pub bytes: u64,
    /// Maximum framed nodes and input collection items.
    pub nodes: u64,
}

/// Fail-closed identity construction classifications.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityError {
    /// Byte/node/depth bound, arithmetic, or allocation failure.
    Limit,
    /// Cancellation prevented complete publication.
    Cancelled,
    /// Noncanonical or malformed structural input, not a semantic validation result.
    InvalidInput,
}

/// Typed identity and literal complete transcript suitable for reproducibility review.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityTranscript<I> {
    /// Layer-specific SHA-256 identity.
    identity: I,
    /// Full NHT envelope and domain, not only the inner payload.
    bytes: Vec<u8>,
}

impl<I: Copy> IdentityTranscript<I> {
    /// Returns the layer-specific identity without allowing substitution between layers.
    #[must_use]
    pub const fn identity(&self) -> I {
        self.identity
    }
    /// Returns the exact bytes hashed once by SHA-256.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Defines non-interchangeable digest wrappers with identical public byte access.
macro_rules! identity {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(SemanticDigest);
        impl $name {
            /// Returns the raw SHA-256 bytes of this identity layer.
            #[must_use]
            pub const fn as_bytes(self) -> [u8; 32] {
                self.0.as_bytes()
            }
        }
        impl std::fmt::Display for $name {
            /// Displays lowercase SHA-256 hexadecimal without a layer-changing conversion.
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
identity!(
    ExpressionCapturedClosureIdentity,
    "Exact expression-profile capture identity."
);
identity!(
    ExpressionLogicalIdentity,
    "Complete expression-profile meaning, including residual trees and public templates."
);
identity!(
    ExpressionInterfaceIdentity,
    "Expression public interpretive projection, distinct from complete logical identity."
);
identity!(
    ExpressionDerivationIdentity,
    "Expression logical and captured identities plus explicit processing policy."
);
identity!(
    ExpressionArtifactIdentity,
    "Expression derivation plus selected representation and public roots."
);

/// Shares bounded NHT framing while the `/3` identity module selects frozen profile/domain pairs.
///
/// Private so callers cannot invent or override protocol selectors.
fn transcript_profile<I>(
    domain: &str,
    profile: &str,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
    body: impl FnOnce(&mut framing::Writer<'_>) -> Result<(), IdentityError>,
    wrap: impl FnOnce(SemanticDigest) -> I,
) -> Result<IdentityTranscript<I>, IdentityError> {
    let mut writer = framing::Writer::new(limits, cancellation)?;
    writer.frame(NHT_ENVELOPE, |writer| {
        writer.frame(domain, |writer| {
            writer.leaf("identity-profile", profile.as_bytes())?;
            body(writer)
        })
    })?;
    writer.check()?;
    let bytes = writer.finish();
    let digest = SemanticDigest::from_transcript(&bytes);
    if cancellation.is_cancelled() {
        return Err(IdentityError::Cancelled);
    }
    Ok(IdentityTranscript {
        identity: wrap(digest),
        bytes,
    })
}
