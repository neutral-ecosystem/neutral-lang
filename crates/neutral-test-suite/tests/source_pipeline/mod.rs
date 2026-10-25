// SPDX-License-Identifier: Apache-2.0

//! File-backed source checks through the current expression-project pipeline.

use neutral_compiler::{
    CapturedExpressionProject, CapturedExpressionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, CapturedVocabularyInput, ExpressionCaptureFailure,
    ExpressionCompilationCache, ProjectCacheLimits, ProjectCaptureControls,
    ProjectCaptureLimitValues, ProjectCaptureLimits, capture_expression_project,
    compile_expression_project,
};
use neutral_core::allocation::Shared;
use neutral_core::{CancellationToken, StructuralLimits, profile::LanguageProfile};
use neutral_encoding::{
    DecodeLimits,
    expression::{decode_expression_project, encode_expression_project},
};
use neutral_ir::expression::profile::{self, ExpressionLimits, SemanticProfiles};
use neutral_ir::expression::templates::ExpansionLimits;
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity, composition::BindingValue as V,
    expression::ExpressionBody,
};
use neutral_reader::{SemanticLimits, ValidatedExpressionProject};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

mod system;

/// Registers exact source bytes with an explicit module rather than rewriting a header.
macro_rules! source {
    ($kind:literal, $name:literal, $module:literal) => {
        (
            $module,
            include_str!(concat!("fixtures/", $kind, "/", $name, ".neu")),
        )
    };
}

mod cross_module;
mod negative;
mod positive;

fn composition_limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(1_048_576, 64).unwrap(),
    ))
}

fn capture(units: &[(&str, &str)]) -> CapturedExpressionProject {
    capture_result(units).expect("current source closure captures")
}

/// Negative fixtures distinguish rejected capture from rejected complete compilation.
fn capture_result(
    units: &[(&str, &str)],
) -> Result<CapturedExpressionProject, ExpressionCaptureFailure> {
    capture_inputs(units, Vec::new())
}

/// All sources and exact vocabulary locks remain explicitly supplied by each case.
fn capture_inputs(
    units: &[(&str, &str)],
    vocabularies: Vec<CapturedVocabularyInput>,
) -> Result<CapturedExpressionProject, ExpressionCaptureFailure> {
    let sources = units
        .iter()
        .map(|(module, text)| {
            CapturedSourceInput::new(
                format!("source:{module}"),
                *module,
                text.as_bytes().to_vec(),
            )
        })
        .collect();
    let capture = CapturedProjectRequest::new(
        profile::CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        sources,
        vocabularies,
        ProjectCaptureControls::new(
            ProjectCaptureLimits::new(ProjectCaptureLimitValues {
                total_source_bytes: 1_048_576,
                source_bytes_per_unit: 1_048_576,
                source_units: 8,
                source_id_bytes: 256,
                module_id_bytes: 256,
                vocabulary_units: 8,
                vocabulary_bytes_per_unit: 1_048_576,
                total_vocabulary_bytes: 1_048_576,
                imports_per_module: 16,
                import_edges: 64,
                scc_units: 8,
                declarations: 4096,
                diagnostics: 64,
                output_bytes: 1_048_576,
            }),
            CancellationToken::new(),
        ),
    );
    capture_expression_project(CapturedExpressionProjectRequest::new(
        capture,
        SemanticProfiles::try_frozen().unwrap(),
        composition_limits(),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
}

fn semantic(ir: &neutral_ir::expression::project::ExpressionProjectIr) -> SemanticLimits {
    SemanticLimits {
        project: ir.limits,
        composition: composition_limits(),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    }
}

fn pipeline(
    units: &[(&str, &str)],
) -> (
    Shared<neutral_ir::expression::project::ExpressionProjectIr>,
    Vec<u8>,
    ValidatedExpressionProject,
) {
    pipeline_from_capture(&capture(units))
}

/// Vocabulary-backed fixtures cross the same complete independent artifact boundary.
fn pipeline_with_vocabularies(
    units: &[(&str, &str)],
    vocabularies: Vec<CapturedVocabularyInput>,
) -> (
    Shared<neutral_ir::expression::project::ExpressionProjectIr>,
    Vec<u8>,
    ValidatedExpressionProject,
) {
    pipeline_from_capture(&capture_inputs(units, vocabularies).unwrap())
}

/// Encodes a complete producer result and obtains authority afresh from its bytes.
fn pipeline_from_capture(
    captured: &CapturedExpressionProject,
) -> (
    Shared<neutral_ir::expression::project::ExpressionProjectIr>,
    Vec<u8>,
    ValidatedExpressionProject,
) {
    let token = CancellationToken::new();
    let ir = compile_expression_project(captured, &token).expect("current source compiles");
    let validated = ValidatedExpressionProject::from_ir(ir.clone(), semantic(&ir), &token).unwrap();
    let bytes = encode_expression_project(&validated, &token).unwrap();
    let decoded =
        decode_expression_project(&bytes, DecodeLimits::hard(), semantic(&ir), &token).unwrap();
    assert_eq!(decoded.ir(), validated.ir());
    (ir, bytes, decoded)
}

/// Literal expected owners never come from producer-generated symbol indexes.
fn symbol(module: &str, name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", module), name)
}

/// Reads a specific independently decoded static result rather than trusting counts or round trips.
fn materialized<'a>(project: &'a ValidatedExpressionProject, module: &str, name: &str) -> &'a V {
    let declaration = project
        .ir()
        .declarations
        .iter()
        .find(|d| d.identity == symbol(module, name))
        .unwrap();
    let Some(ExpressionBody::Value(value)) = &declaration.body else {
        panic!("{module}::{name} did not materialize")
    };
    value
}

/// Builds an explicit numeric expectation independently from source compilation and evaluation.
fn number(literal: &str) -> V {
    V::Number(ExactNumber::from_source(literal, 128, 128).unwrap())
}

/// Current real sources retain exact spans and round-trip through independently validated `/3` bytes.
#[test]
fn property_current_sources_capture_compile_and_round_trip() {
    let (ir, bytes, decoded) = pipeline(&[
        source!("positive", "consumer", "app::consumer"),
        source!("positive", "orphan", "app::orphan"),
        source!("positive", "shared", "app::shared"),
    ]);
    assert!(bytes.starts_with(b"NEUP3\r\n\x1a"));
    assert_eq!(ir.modules.len(), 3);
    assert_eq!(ir.declarations.len(), 8);
    assert_eq!(decoded.ir(), ir.as_ref());
    assert_eq!(
        materialized(&decoded, "app::consumer", "answer"),
        &number("42")
    );
    assert_eq!(
        materialized(&decoded, "app::consumer", "copied"),
        &V::Record(vec![
            ("label".into(), Some(V::String("private-value".into()))),
            (
                "ports".into(),
                Some(V::List(vec![number("80"), number("443")]))
            ),
        ])
    );
    assert_eq!(
        materialized(&decoded, "app::consumer", "pointer"),
        &V::Reference(symbol("app::shared", "api"))
    );
    for source_map in &ir.source_maps {
        let source = ir
            .sources
            .iter()
            .find(|source| source.digest == source_map.location.source())
            .unwrap();
        assert!(source_map.location.span().start() < source_map.location.span().end());
        assert!(source_map.location.span().end() <= source.byte_len);
    }
}

/// Reversing supplied source order leaves the canonical complete artifact unchanged.
#[test]
fn property_source_order_does_not_change_artifact() {
    let units = [
        source!("positive", "consumer", "app::consumer"),
        source!("positive", "orphan", "app::orphan"),
        source!("positive", "shared", "app::shared"),
    ];
    let (_, expected, _) = pipeline(&units);
    let reversed = units.into_iter().rev().collect::<Vec<_>>();
    let (_, actual, _) = pipeline(&reversed);
    assert_eq!(actual, expected);
}

/// Cache hits reuse only unchanged syntax, and a failed changed generation leaves the prior one usable.
#[test]
fn current_source_cache_reuses_and_recovers_atomically() {
    let units = [
        source!("positive", "consumer", "app::consumer"),
        source!("positive", "shared", "app::shared"),
    ];
    let captured = capture(&units);
    let mut cache = ExpressionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: 1_048_576,
    })
    .unwrap();
    let token = CancellationToken::new();
    let (first, _) = cache.compile_expression(&captured, &token).unwrap();
    let (warm, stats) = cache.compile_expression(&captured, &token).unwrap();
    assert_eq!(warm, first);
    assert_eq!(stats.parsed_units, 0);
    assert_eq!(stats.reused_units, 2);
    let updated_input = capture(&[
        source!("positive", "consumer", "app::consumer"),
        source!("positive", "shared-updated", "app::shared"),
    ]);
    let (updated, stats) = cache.compile_expression(&updated_input, &token).unwrap();
    assert_ne!(updated, first);
    assert_eq!(stats.parsed_units, 1);
    assert_eq!(stats.reused_units, 1);
    let (clean_updated, _, decoded_updated) = pipeline_from_capture(&updated_input);
    assert_eq!(updated, clean_updated);
    assert_eq!(
        materialized(&decoded_updated, "app::consumer", "answer"),
        &number("43")
    );
    assert_eq!(
        materialized(&decoded_updated, "app::consumer", "copied"),
        &V::Record(vec![
            ("label".into(), Some(V::String("private-updated".into()))),
            (
                "ports".into(),
                Some(V::List(vec![number("80"), number("443")]))
            ),
        ])
    );
    let changed = capture(&[
        source!("negative", "cache-invalid", "app::consumer"),
        source!("positive", "shared", "app::shared"),
    ]);
    assert!(cache.compile_expression(&changed, &token).is_err());
    assert_eq!(
        cache.compile_expression(&updated_input, &token).unwrap().0,
        updated
    );
}

/// A unique temporary root for executable checks shared with the composition contract suite.
pub(super) struct TestRoot(pub(super) PathBuf);
impl TestRoot {
    pub(super) fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "neutral-source-pipeline-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn executable(name: &str) -> PathBuf {
    let output = Command::new(env!("CARGO"))
        .args(["build", "--package", name, "--message-format=json"])
        .output()
        .expect("cargo build");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|message| {
            message["reason"] == "compiler-artifact"
                && message["target"]["name"] == name
                && message["executable"].is_string()
        })
        .and_then(|message| message["executable"].as_str().map(PathBuf::from))
        .expect("Cargo reports executable")
}
