// SPDX-License-Identifier: Apache-2.0
//! Current expression-project measurements, shared by harness and smoke tests.
use neutral_compiler::{
    CapturedExpressionProject, CapturedExpressionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, ExpressionCompilationCache, ProjectCacheLimits, ProjectCaptureControls,
    ProjectCaptureLimitValues, ProjectCaptureLimits, capture_expression_project,
    compile_expression_project,
};
use neutral_core::{
    CancellationToken, StructuralLimits, allocation::Shared, profile::LanguageProfile,
};
use neutral_encoding::{
    DecodeLimits,
    expression::{decode_expression_project, encode_expression_project},
};
use neutral_ir::{
    expression::{
        profile::{self, ExpressionLimits, SemanticProfiles},
        templates::ExpansionLimits,
    },
    project_identity::{IdentityLimits, canonical_expression_project, expression_interface},
};
use neutral_reader::{
    SemanticLimits, ValidatedExpressionProject, expression::ExpressionViewRequest,
};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};
use std::{
    fmt::Write as _,
    hint::black_box,
    time::{Duration, Instant},
};
/// Immutable current source shared by direct and instrumented samples.
const SOURCE: &str = include_str!("../corpora/composition.neu");
/// Explicit capture ceiling shared by benchmark requests and identity output.
const SOURCE_BYTES: u64 = 1_048_576;
/// Increasing declaration counts used to expose growth rather than fixed-input timing.
const GROWTH: [usize; 4] = [1, 8, 64, 128];
/// Independent requests used to check concurrent isolation.
const WORKERS: usize = 8;
/// Uses the same bounded nominal contracts for every measured phase.
fn semantic() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(SOURCE_BYTES, 64).unwrap(),
    ))
}
/// Captures only supplied bytes under an exact current semantic tuple.
fn capture(source: &str) -> CapturedExpressionProject {
    capture_expression_project(CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "bench:benchmark",
                "benchmark",
                source.as_bytes().to_vec(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(ProjectCaptureLimitValues {
                    total_source_bytes: SOURCE_BYTES,
                    source_bytes_per_unit: SOURCE_BYTES,
                    source_units: 8,
                    source_id_bytes: 128,
                    module_id_bytes: 128,
                    vocabulary_units: 8,
                    vocabulary_bytes_per_unit: SOURCE_BYTES,
                    total_vocabulary_bytes: SOURCE_BYTES,
                    imports_per_module: 16,
                    import_edges: 64,
                    scc_units: 8,
                    declarations: 4096,
                    diagnostics: 64,
                    output_bytes: SOURCE_BYTES,
                }),
                CancellationToken::new(),
            ),
        ),
        SemanticProfiles::try_frozen().unwrap(),
        semantic(),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
    .unwrap()
}
/// Intersects independent reading with the producer's complete project controls.
fn limits(ir: &neutral_ir::expression::project::ExpressionProjectIr) -> SemanticLimits {
    SemanticLimits {
        project: ir.limits,
        composition: semantic(),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    }
}
/// Opens raw producer data through the independent reader before observation.
fn read(
    ir: &Shared<neutral_ir::expression::project::ExpressionProjectIr>,
) -> ValidatedExpressionProject {
    ValidatedExpressionProject::from_ir(ir.clone(), limits(ir), &CancellationToken::new()).unwrap()
}
/// Warms once, then measures exactly the reported number of operations.
pub(crate) fn measure(iterations: usize, mut operation: impl FnMut()) -> Duration {
    operation();
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    start.elapsed()
}
/// Reports direct phase, identity, cache, growth and concurrent measurements.
pub(crate) fn run(iterations: usize, mut report: impl FnMut(&str, usize, Duration)) {
    let cancel = CancellationToken::new();
    report(
        "expression-capture",
        iterations,
        measure(iterations, || {
            black_box(capture(SOURCE));
        }),
    );
    let captured = capture(SOURCE);
    report(
        "expression-compile",
        iterations,
        measure(iterations, || {
            black_box(compile_expression_project(&captured, &cancel).unwrap());
        }),
    );
    let ir = compile_expression_project(&captured, &cancel).unwrap();
    report(
        "expression-reader",
        iterations,
        measure(iterations, || {
            black_box(read(&ir));
        }),
    );
    let project = read(&ir);
    report(
        "expression-encode",
        iterations,
        measure(iterations, || {
            black_box(encode_expression_project(&project, &cancel).unwrap());
        }),
    );
    let bytes = encode_expression_project(&project, &cancel).unwrap();
    report(
        "expression-decode",
        iterations,
        measure(iterations, || {
            let decoded =
                decode_expression_project(&bytes, DecodeLimits::hard(), limits(&ir), &cancel)
                    .unwrap();
            assert_eq!(decoded.ir(), project.ir());
            black_box(decoded);
        }),
    );
    report(
        "expression-identities",
        iterations,
        measure(iterations, || {
            let policy = IdentityLimits {
                bytes: SOURCE_BYTES,
                nodes: 1_000_000,
            };
            black_box(canonical_expression_project(&ir, policy, &cancel).unwrap());
            black_box(expression_interface(&ir, policy, &cancel).unwrap());
        }),
    );
    projection_measurements(iterations, &project, &bytes, &cancel, &mut report);
    cache_measurements(iterations, &captured, &ir, &mut report);
    let start = Instant::now();
    for count in GROWTH {
        let mut source = format!(
            "neu \"{}\"\nmodule benchmark\n",
            LanguageProfile::V1_0.source_version()
        );
        for i in 0..count {
            writeln!(source, "public num value{i} = {i}").unwrap();
        }
        let c = capture(&source);
        black_box(read(&compile_expression_project(&c, &cancel).unwrap()));
    }
    report("expression-growth", GROWTH.len(), start.elapsed());
    let start = Instant::now();
    let threads = (0..WORKERS)
        .map(|_| {
            let expected = ir.clone();
            std::thread::spawn(move || {
                let result =
                    compile_expression_project(&capture(SOURCE), &CancellationToken::new())
                        .unwrap();
                assert_eq!(result, expected);
                black_box(read(&result));
            })
        })
        .collect::<Vec<_>>();
    for thread in threads {
        thread.join().unwrap();
    }
    report("expression-concurrent", WORKERS, start.elapsed());
}
/// Measures public closure and independently decoded standalone inspection separately.
fn projection_measurements(
    iterations: usize,
    project: &ValidatedExpressionProject,
    bytes: &[u8],
    cancel: &CancellationToken,
    report: &mut impl FnMut(&str, usize, Duration),
) {
    let ir = project.ir();
    let roots = ir
        .declarations
        .iter()
        .filter(|d| d.public)
        .map(|d| d.identity.clone())
        .collect::<Vec<_>>();
    let view = ExpressionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA,
        semantic_profiles: &ir.semantic_profiles,
        roots: &roots,
    };
    report(
        "expression-view",
        iterations,
        measure(iterations, || {
            black_box(project.derive_view(&view, cancel).unwrap());
        }),
    );
    report(
        "expression-probe",
        iterations,
        measure(iterations, || {
            black_box(
                neutral_probe::expression::render_standalone_expression(bytes, None, cancel)
                    .unwrap(),
            );
        }),
    );
}

/// Exercises real warm and changed-input parser generations against clean output.
fn cache_measurements(
    iterations: usize,
    captured: &CapturedExpressionProject,
    ir: &Shared<neutral_ir::expression::project::ExpressionProjectIr>,
    report: &mut impl FnMut(&str, usize, Duration),
) {
    let cancel = CancellationToken::new();
    let mut cache = ExpressionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: SOURCE_BYTES,
    })
    .unwrap();
    cache.compile_expression(captured, &cancel).unwrap();
    report(
        "expression-cache-warm",
        iterations,
        measure(iterations, || {
            let (actual, stats) = cache.compile_expression(captured, &cancel).unwrap();
            assert_eq!(stats.parsed_units, 0);
            assert_eq!(&actual, ir);
            black_box(actual);
        }),
    );
    let changed = capture(&SOURCE.replace("42", "43"));
    let changed_ir = compile_expression_project(&changed, &cancel).unwrap();
    assert_ne!(&changed_ir, ir);
    let mut use_changed = false;
    report(
        "expression-cache-changed",
        iterations,
        measure(iterations, || {
            use_changed = !use_changed;
            let (input, expected) = if use_changed {
                (&changed, &changed_ir)
            } else {
                (captured, ir)
            };
            let (actual, stats) = cache.compile_expression(input, &cancel).unwrap();
            assert_eq!((stats.parsed_units, stats.reused_units), (1, 0));
            assert_eq!(&actual, expected);
            black_box(actual);
        }),
    );
}
#[cfg(test)]
#[path = "../tests/pipeline/mod.rs"]
mod tests;
