// SPDX-License-Identifier: Apache-2.0

//! Shared bounded current-profile source, wire and consumer fuzz exercises.
//!
//! The explicit seed registry is runtime-owned. Valid artifacts are generated once
//! per seed, then mutations can touch any section rather than just header bytes.

// Each standalone target intentionally uses a different subset of this shared harness.
#![allow(dead_code)]

use neutral_compiler::{
    CapturedExpressionProject, CapturedExpressionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_expression_project, compile_expression_project,
};
use neutral_core::{CancellationToken, StructuralLimits, profile::LanguageProfile};
use neutral_encoding::{
    DecodeLimits,
    expression::{decode_expression_project, encode_expression_project},
};
use neutral_ir::expression::{
    profile::ExpressionLimits, profile::SemanticProfiles, templates::ExpansionLimits,
};
use neutral_reader::{SemanticLimits, ValidatedExpressionProject};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};
use std::sync::OnceLock;

/// Logical module shared by the runtime-owned source seeds.
pub const SOURCE_MODULE: &str = "example";
/// Registered sources cover materialized data, deferred ports/guards and finite templates.
pub const SOURCE_SEEDS: &[&[u8]] = &[
    include_bytes!("../seeds/composition-source.neu"),
    include_bytes!("../seeds/expression-consumer.neu"),
    include_bytes!("../seeds/expression-expansion.neu"),
    include_bytes!("../seeds/expression-guards.neu"),
];
/// Input ceiling is checked before copying arbitrary fuzzer-owned source bytes.
const SOURCE_BYTES: usize = 65_536;

/// Captures one explicit current source without filesystem, registry or environment resolution.
pub fn capture(bytes: &[u8]) -> Option<CapturedExpressionProject> {
    if bytes.len() > SOURCE_BYTES {
        return None;
    }
    let limits = ProjectCaptureLimitValues {
        total_source_bytes: SOURCE_BYTES as u64,
        source_bytes_per_unit: SOURCE_BYTES as u64,
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
    capture_expression_project(CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            neutral_ir::expression::profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "fuzz:source",
                SOURCE_MODULE,
                bytes.to_vec(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(limits),
                CancellationToken::new(),
            ),
        ),
        SemanticProfiles::try_frozen().ok()?,
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(65_536, 32).ok()?,
        )),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
    .ok()
}

/// Shares finite independent reader controls across all expression targets.
pub fn semantic_limits() -> SemanticLimits {
    SemanticLimits {
        project: neutral_encoding::hard_project_limits(),
        composition: CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(65_536, 32).unwrap(),
        )),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    }
}

/// Builds a complete independently checked frame from a captured source.
pub fn encode(captured: &CapturedExpressionProject) -> Option<Vec<u8>> {
    let cancel = CancellationToken::new();
    let ir = compile_expression_project(captured, &cancel).ok()?;
    let project = ValidatedExpressionProject::from_ir(ir, semantic_limits(), &cancel).ok()?;
    encode_expression_project(&project, &cancel).ok()
}

/// Selects a registered source and mutates only its body, preserving capture's header prefix.
pub fn mutated_source(bytes: &[u8]) -> Vec<u8> {
    let selected = usize::from(bytes.first().copied().unwrap_or_default()) % SOURCE_SEEDS.len();
    let mut source = SOURCE_SEEDS[selected].to_vec();
    let prefix = format!(
        "neu {:?}\nmodule {SOURCE_MODULE}\n",
        LanguageProfile::V1_0.source_version()
    );
    assert!(source.starts_with(prefix.as_bytes()));
    for (original, replacement) in source[prefix.len()..].iter_mut().zip(bytes.iter().skip(1)) {
        *original = *replacement;
    }
    source
}

/// Generates each seed once; a machine-word offset lets mutations reach every encoded section.
pub fn mutated_seed(bytes: &[u8]) -> Vec<u8> {
    static SEEDS: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
    let seeds = SEEDS.get_or_init(|| {
        SOURCE_SEEDS
            .iter()
            .enumerate()
            .map(|(index, source)| {
                encode(&capture(source).expect("registered source capture"))
                    .unwrap_or_else(|| panic!("registered source {index} compiles and validates"))
            })
            .collect()
    });
    let selected = usize::from(bytes.first().copied().unwrap_or_default()) % seeds.len();
    let mut seed = seeds[selected].clone();
    let offset_input = bytes.get(1..).unwrap_or_default();
    let mut word = [0; std::mem::size_of::<usize>()];
    let used = word.len().min(offset_input.len());
    word[..used].copy_from_slice(&offset_input[..used]);
    let offset = usize::from_le_bytes(word) % seed.len();
    for (destination, replacement) in seed[offset..].iter_mut().zip(&offset_input[used..]) {
        *destination = *replacement;
    }
    seed
}

/// Independently validates hostile bytes before any consumer can inspect or evaluate them.
pub fn decode(bytes: &[u8]) -> Option<ValidatedExpressionProject> {
    decode_expression_project(
        bytes,
        DecodeLimits::hard(),
        semantic_limits(),
        &CancellationToken::new(),
    )
    .ok()
}

#[path = "consumer_support.rs"]
pub mod consumer_support;

/// Walks public guarded dependencies and a redacted view without evaluating or scheduling them.
pub fn inspect_public(project: &ValidatedExpressionProject) {
    use neutral_ir::composition::project::CompositionSignature;
    use neutral_reader::expression::{
        ExpressionInspectionLimits, ExpressionViewLimits, ExpressionViewRequest,
    };
    let cancel = CancellationToken::new();
    let roots = project
        .ir()
        .declarations
        .iter()
        .filter(|d| d.public && matches!(d.signature, CompositionSignature::Binding(_)))
        .map(|d| d.identity.clone())
        .collect::<Vec<_>>();
    for root in &roots {
        let _ = project.dependencies(root, ExpressionInspectionLimits::default(), &cancel);
    }
    let _ = project.derive_view_with_limits(
        &ExpressionViewRequest {
            schema: neutral_ir::expression::profile::PROJECT_VIEW_SCHEMA,
            semantic_profiles: &project.ir().semantic_profiles,
            roots: &roots,
        },
        ExpressionViewLimits::default(),
        &cancel,
    );
}

/// Chooses a real public nominal entry contract when available, instead of a binding name as a type.
pub fn probe(bytes: &[u8]) -> Option<neutral_probe::expression::ExpressionProbeSummary> {
    use neutral_ir::{
        LogicalModuleIdentity, ModuleSymbolIdentity, composition::project::CompositionSignature,
        project_interface::ProjectPublicType as T,
    };
    use neutral_reader::{
        consumer::{ENTRY_REQUEST_SCHEMA, EntryRequest, EntrySelection},
        expression::ExpressionViewLimits,
    };
    let decoded = decode(bytes);
    let fallback = T::Nominal(ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(LanguageProfile::V1_0.source_version(), SOURCE_MODULE),
        "Plan",
    ));
    let contract = decoded
        .as_ref()
        .and_then(|p| {
            p.ir().declarations.iter().find_map(|d| match &d.signature {
                CompositionSignature::Binding(
                    ty @ (T::Nominal(_) | T::VocabularyNominal { .. }),
                ) if d.public => Some(ty),
                _ => None,
            })
        })
        .unwrap_or(&fallback);
    let profiles = SemanticProfiles::try_frozen().unwrap();
    neutral_probe::expression::inspect_expression_encoded(
        bytes,
        DecodeLimits::hard(),
        semantic_limits(),
        &neutral_probe::expression::ExpressionEntryProbeRequest {
            semantic_profiles: &profiles,
            entry: EntryRequest {
                schema: ENTRY_REQUEST_SCHEMA,
                contract,
                selection: EntrySelection::Enumerate,
            },
            limits: ExpressionViewLimits::default(),
        },
        &CancellationToken::new(),
    )
    .ok()
}
