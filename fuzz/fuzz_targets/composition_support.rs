// SPDX-License-Identifier: Apache-2.0

//! Bounded successor pipeline input shared by source, hostile wire and probe fuzzers.

use neutral_compiler::{
    CapturedCompositionProject, CapturedCompositionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, CapturedVocabularyInput, ProjectCaptureControls,
    ProjectCaptureLimitValues, ProjectCaptureLimits, capture_composition_project,
    compile_composition_project,
};
use neutral_core::allocation::Shared as Arc;
use neutral_core::{CancellationToken, StructuralLimits, profile::LanguageProfile};
use neutral_encoding::composition::encode_composition_project;
use neutral_ir::composition::profile;
use neutral_ir::project_identity::{
    ArtifactIdentityInput, ArtifactKind, CapturedIdentityInput, CapturedIdentitySource,
    CapturedIdentityVocabulary, IdentityLimits,
};
use neutral_reader::composition::CompositionIdentityContext;
use neutral_reader::composition::ValidatedCompositionProject;
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};
use std::sync::OnceLock;

/// Finite independent successor semantic controls, never selected by package version.
pub fn limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 32).unwrap(),
    ))
}
/// Captures arbitrary source under explicit no-acquisition successor authority.
pub fn capture(bytes: &[u8]) -> Option<CapturedCompositionProject> {
    capture_with_vocabularies(bytes, Vec::new())
}
/// Shares finite host controls with exact multi-bundle captures, never acquiring missing inputs.
pub fn capture_with_vocabularies(
    bytes: &[u8],
    vocabularies: Vec<CapturedVocabularyInput>,
) -> Option<CapturedCompositionProject> {
    let values = ProjectCaptureLimitValues {
        total_source_bytes: 65_536,
        source_bytes_per_unit: 65_536,
        source_units: 4,
        source_id_bytes: 128,
        module_id_bytes: 128,
        vocabulary_units: 4,
        vocabulary_bytes_per_unit: 65_536,
        total_vocabulary_bytes: 65_536,
        imports_per_module: 16,
        import_edges: 64,
        scc_units: 4,
        declarations: 256,
        diagnostics: 16,
        output_bytes: 1_048_576,
    };
    capture_composition_project(CapturedCompositionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "fuzz:source",
                "example",
                bytes.to_vec(),
            )],
            vocabularies,
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(values),
                CancellationToken::new(),
            ),
        ),
        profile::REQUIRED_FEATURES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        limits(),
    ))
    .ok()
}
/// Produces checked successor bytes only when whole source compilation succeeds.
pub fn encode(captured: &CapturedCompositionProject) -> Option<Vec<u8>> {
    let cancel = CancellationToken::new();
    let ir = compile_composition_project(captured, &cancel).ok()?;
    let project =
        ValidatedCompositionProject::from_ir(Arc::clone(&ir), ir.limits, limits(), &cancel)
            .expect("successful successor compiler output must independently validate");
    let sources: Vec<_> = captured
        .sources()
        .iter()
        .map(|s| CapturedIdentitySource {
            module: s.module_id(),
            source_id: s.source_id(),
            digest: s.digest(),
            byte_len: s.bytes().len() as u64,
        })
        .collect();
    let vocabularies: Vec<_> = captured
        .vocabularies()
        .iter()
        .map(|v| {
            let l = v.lock();
            CapturedIdentityVocabulary {
                identity: l.identity(),
                version: l.version(),
                encoding_version: l.encoding_version(),
                schema_version: l.schema_version(),
                digest: l.content_digest(),
                byte_len: v.bytes().len() as u64,
                required_features: l.required_features(),
            }
        })
        .collect();
    let context = CompositionIdentityContext {
        capture: CapturedIdentityInput {
            profile: LanguageProfile::V1_0.source_version(),
            sources: &sources,
            vocabularies: &vocabularies,
        },
        required_features: captured.required_features(),
        producer: env!("CARGO_PKG_NAME"),
        producer_version: env!("CARGO_PKG_VERSION"),
        capture_limits: captured.identity_capture_limits(),
    };
    let policy = IdentityLimits {
        bytes: neutral_ir::project_identity::MAX_TRANSCRIPT_BYTES,
        nodes: neutral_ir::project_identity::MAX_TRANSCRIPT_NODES,
    };
    match project.identities(&context, policy, &cancel) {
        Ok(identities) => {
            identities
                .artifact(
                    &ArtifactIdentityInput {
                        kind: ArtifactKind::Project,
                        format: profile::ENCODING,
                        roots: &[],
                        options: &[],
                    },
                    policy,
                    &cancel,
                )
                .expect("complete successor identity must frame a complete artifact");
        }
        Err(neutral_reader::composition::CompositionIdentityReadError::Identity(
            neutral_ir::project_identity::IdentityError::Limit,
        )) => {}
        Err(error) => panic!("successful project identities are invalid: {error:?}"),
    }
    Some(
        encode_composition_project(&project, &cancel).expect("bounded reviewed output must encode"),
    )
}
/// Mutates a valid, independently checked binary seed so wire fuzzing reaches beyond magic rejection.
pub fn mutated_seed(bytes: &[u8]) -> Vec<u8> {
    static SEED: OnceLock<Vec<u8>> = OnceLock::new();
    let mut seed = SEED
        .get_or_init(|| {
            let captured = capture(include_bytes!("../seeds/composition-source.neu")).unwrap();
            encode(&captured).unwrap()
        })
        .clone();
    // First two input bytes choose a region; remaining bytes replace it. Empty
    // input exercises the complete valid reader/probe path on every campaign.
    if let Some((&first, tail)) = bytes.split_first() {
        let offset =
            (usize::from(first) * 257 + tail.first().map_or(0, |n| usize::from(*n))) % seed.len();
        for (destination, replacement) in seed[offset..].iter_mut().zip(tail.iter().skip(1)) {
            *destination = *replacement;
        }
    }
    seed
}
