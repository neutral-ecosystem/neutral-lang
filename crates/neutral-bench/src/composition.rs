// SPDX-License-Identifier: Apache-2.0

//! Current composition-pipeline measurements, shared by the harness and smoke tests.

use neutral_compiler::{
    CapturedCompositionProject, CapturedCompositionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, CapturedVocabularyInput, CompositionCompilationCache, ProjectCacheLimits,
    ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_composition_project, compile_composition_project,
};
use neutral_core::{
    CancellationToken, StructuralLimits, VocabularyContentDigest, allocation::Shared,
    profile::LanguageProfile,
};
use neutral_encoding::{
    DecodeLimits,
    composition::{decode_composition_project, encode_composition_project},
};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::profile,
    project_identity::{
        ArtifactIdentityInput, ArtifactKind, CompositionDerivationContext, IdentityLimits,
        canonical_composition_project, composition_artifact_identity,
        composition_derivation_identity, composition_interface,
    },
};
use neutral_reader::composition::{CompositionViewRequest, ValidatedCompositionProject};
use neutral_vocabulary::{
    VocabularyLimits, VocabularyLock,
    composition::{self, CompositionLimits},
};
use std::{
    fmt::Write as _,
    hint::black_box,
    time::{Duration, Instant},
};

/// Immutable current-language corpus; package releases never select its language profile.
const SOURCE: &str = include_str!("../corpora/composition.neu");
/// Declarative defaults and restrictions interpreted independently by the reader.
const VOCABULARY: &[u8] = include_bytes!("../corpora/composition-vocabulary.json");
/// Independent scale/work ceiling for this controlled corpus.
const SOURCE_BYTES: u64 = 1_048_576;
/// Growth sizes remain fixed so runs compare the same work.
const GROWTH: [usize; 4] = [1, 8, 64, 128];
/// Independent requests exercise isolation without a shared compiler cache.
const WORKERS: usize = 8;

/// Returns explicit finite semantic controls for the current corpus.
fn limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(SOURCE_BYTES, 64)
            .unwrap()
            .with_traversal_nodes(1_000_000)
            .unwrap(),
    ))
}

/// Captures exact source units and locks without paths, acquisition or inferred defaults.
fn capture(source: &str) -> CapturedCompositionProject {
    let controls = ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: SOURCE_BYTES,
        source_bytes_per_unit: SOURCE_BYTES,
        source_units: 64,
        source_id_bytes: 128,
        module_id_bytes: 128,
        vocabulary_units: 64,
        vocabulary_bytes_per_unit: SOURCE_BYTES,
        total_vocabulary_bytes: SOURCE_BYTES,
        imports_per_module: 64,
        import_edges: 256,
        scc_units: 64,
        declarations: 512,
        diagnostics: 64,
        output_bytes: SOURCE_BYTES,
    });
    let lock = VocabularyLock::new(
        "BenchDomain",
        "1.0.0",
        composition::ENCODING_VERSION,
        composition::SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(VOCABULARY),
        vec![composition::REQUIRED_FEATURE.to_owned()],
    )
    .unwrap();
    capture_composition_project(CapturedCompositionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![
                CapturedSourceInput::new(
                    "bench:benchmark",
                    "benchmark",
                    source.as_bytes().to_vec(),
                ),
                CapturedSourceInput::new(
                    "bench:helper",
                    "helper",
                    b"neu \"1.0\"\nmodule helper\npublic num value = 7\n".to_vec(),
                ),
            ],
            vec![CapturedVocabularyInput::new(VOCABULARY.to_vec(), lock)],
            ProjectCaptureControls::new(controls, CancellationToken::new()),
        ),
        profile::REQUIRED_FEATURES
            .iter()
            .map(|feature| (*feature).to_owned())
            .collect(),
        limits(),
    ))
    .unwrap()
}

/// Independently validates current producer output rather than benchmarking unchecked IR.
fn read(
    ir: Shared<neutral_ir::composition::project::CompositionProjectIr>,
) -> ValidatedCompositionProject {
    let policy = ir.limits;
    ValidatedCompositionProject::from_ir(ir, policy, limits(), &CancellationToken::new()).unwrap()
}

/// Warms once and times a fixed operation count using a monotonic clock.
pub(crate) fn measure(iterations: usize, mut operation: impl FnMut()) -> Duration {
    operation();
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    start.elapsed()
}

/// Runs current source/vocabulary, independent consumer, identity and cache measurements.
pub(crate) fn run(iterations: usize, mut report: impl FnMut(&str, usize, Duration)) {
    let cancel = CancellationToken::new();
    report(
        "composition-capture",
        iterations,
        measure(iterations, || {
            black_box(capture(SOURCE));
        }),
    );
    let captured = capture(SOURCE);
    report(
        "composition-graph",
        iterations,
        measure(iterations, || {
            black_box(captured.module_graph(&cancel).unwrap());
        }),
    );
    report(
        "composition-compile",
        iterations,
        measure(iterations, || {
            black_box(compile_composition_project(&captured, &cancel).unwrap());
        }),
    );
    let ir = compile_composition_project(&captured, &cancel).unwrap();
    report(
        "composition-reader",
        iterations,
        measure(iterations, || {
            black_box(read(ir.clone()));
        }),
    );
    let project = read(ir.clone());
    report(
        "composition-encode",
        iterations,
        measure(iterations, || {
            black_box(encode_composition_project(&project, &cancel).unwrap());
        }),
    );
    let encoded = encode_composition_project(&project, &cancel).unwrap();
    report(
        "composition-decode",
        iterations,
        measure(iterations, || {
            let decoded = decode_composition_project(
                &encoded,
                DecodeLimits::hard(),
                ir.limits,
                limits(),
                &cancel,
            )
            .unwrap();
            assert_eq!(decoded.complete_ir(), project.complete_ir());
            black_box(decoded);
        }),
    );
    let selection = CompositionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA.to_owned(),
        roots: vec![ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(LanguageProfile::V1_0.source_version(), "benchmark"),
            "status",
        )],
    };
    report(
        "composition-view",
        iterations,
        measure(iterations, || {
            black_box(project.derive_view(&selection, &cancel).unwrap());
        }),
    );
    identity_measurements(iterations, &captured, &ir, &mut report);
    cache_measurements(iterations, &captured, &ir, &mut report);
    scale_measurements(&ir, report);
}

/// Measures fixed growth sizes and independent concurrent requests with checked results.
fn scale_measurements(
    ir: &Shared<neutral_ir::composition::project::CompositionProjectIr>,
    mut report: impl FnMut(&str, usize, Duration),
) {
    let cancel = CancellationToken::new();
    let start = Instant::now();
    for count in GROWTH {
        let mut source = format!(
            "neu \"{}\"\nmodule benchmark\nuse BenchDomain as domain\n",
            LanguageProfile::V1_0.source_version()
        );
        for index in 0..count {
            writeln!(source, "public num value{index} = {index}").unwrap();
        }
        let growing = capture(&source);
        black_box(read(
            compile_composition_project(&growing, &cancel).unwrap(),
        ));
    }
    report("composition-growth", GROWTH.len(), start.elapsed());
    let start = Instant::now();
    // Owned workers tear down their thread state before Memcheck inspects process exit.
    let workers = (0..WORKERS)
        .map(|_| {
            let expected = ir.clone();
            std::thread::spawn(move || {
                let request = capture(SOURCE);
                let result =
                    compile_composition_project(&request, &CancellationToken::new()).unwrap();
                assert_eq!(result, expected);
                black_box(read(result));
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        worker.join().unwrap();
    }
    report("composition-concurrent", WORKERS, start.elapsed());
}

/// Times all current identity partitions on the same compiled project.
fn identity_measurements(
    iterations: usize,
    captured: &CapturedCompositionProject,
    ir: &Shared<neutral_ir::composition::project::CompositionProjectIr>,
    mut report: impl FnMut(&str, usize, Duration),
) {
    let cancel = CancellationToken::new();
    report(
        "composition-identities",
        iterations,
        measure(iterations, || {
            let policy = IdentityLimits {
                bytes: SOURCE_BYTES,
                nodes: 1_000_000,
            };
            let captured_id = captured.identity_transcript(policy, &cancel).unwrap();
            let logical = canonical_composition_project(ir, policy, &cancel).unwrap();
            black_box(composition_interface(ir, policy, &cancel).unwrap());
            let derivation = composition_derivation_identity(
                logical.identity(),
                &CompositionDerivationContext {
                    captured: captured_id.identity(),
                    producer: "neutral-bench",
                    producer_version: env!("CARGO_PKG_VERSION"),
                    capture_limits: captured.identity_capture_limits(),
                    project_limits: ir.limits,
                    composition_limits: ir.composition_limits,
                },
                policy,
                &cancel,
            )
            .unwrap();
            black_box(
                composition_artifact_identity(
                    derivation.identity(),
                    &ArtifactIdentityInput {
                        kind: ArtifactKind::Project,
                        format: profile::ENCODING,
                        roots: &[],
                        options: &[],
                    },
                    policy,
                    &cancel,
                )
                .unwrap(),
            );
        }),
    );
}

/// Compares real warm and changed-unit cache execution against clean compilation.
fn cache_measurements(
    iterations: usize,
    captured: &CapturedCompositionProject,
    ir: &Shared<neutral_ir::composition::project::CompositionProjectIr>,
    mut report: impl FnMut(&str, usize, Duration),
) {
    let cancel = CancellationToken::new();
    let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 64,
        source_bytes: SOURCE_BYTES,
    })
    .unwrap();
    cache.compile(captured, &cancel).unwrap();
    report(
        "composition-cache-warm",
        iterations,
        measure(iterations, || {
            let (cached, facts) = cache.compile(captured, &cancel).unwrap();
            assert_eq!(facts.parsed_units, 0);
            assert_eq!(&cached, ir);
            black_box(cached);
        }),
    );
    let changed = capture(&SOURCE.replace("42", "43"));
    let changed_ir = compile_composition_project(&changed, &cancel).unwrap();
    assert_ne!(&changed_ir, ir);
    let mut switch = false;
    report(
        "composition-cache-changed",
        iterations,
        measure(iterations, || {
            switch = !switch;
            let (input, expected) = if switch {
                (&changed, &changed_ir)
            } else {
                (captured, ir)
            };
            let (cached, facts) = cache.compile(input, &cancel).unwrap();
            assert_eq!((facts.parsed_units, facts.reused_units), (1, 1));
            assert_eq!(&cached, expected);
            black_box(cached);
        }),
    );
}

#[cfg(test)]
#[path = "../tests/composition/mod.rs"]
mod tests;
