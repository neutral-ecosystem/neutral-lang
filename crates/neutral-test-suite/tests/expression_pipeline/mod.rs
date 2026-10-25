// SPDX-License-Identifier: Apache-2.0

//! File-backed `/3` source → complete wire → independent public interpretation.

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
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::BindingValue as V,
    expression::{
        ExpressionBody,
        profile::{self, ExpressionLimits, SemanticProfiles},
        templates::ExpansionLimits,
    },
};
use neutral_reader::{
    SemanticLimits, ValidatedExpressionProject,
    expression::{
        ExpressionInspectionLimits, ExpressionViewLimits, ExpressionViewRequest, GuardCondition,
    },
};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

mod bounds;
mod cache;
mod consumer;
mod inspection;
mod projection;
mod registered;
mod source_qualification;

/// Stable logical symbol; source IDs and host paths never participate in meaning.
fn symbol(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "pipeline"), name)
}

/// Independent finite test policies; production defaults and package versions are not test oracles.
fn composition_limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(1_048_576, 64).unwrap(),
    ))
}

/// Captures exact source bytes with the same explicit current profiles as the CLI.
fn capture(source: &[u8]) -> CapturedExpressionProject {
    capture_with_vocabularies(source, vec![])
}

/// Captures an exact explicit catalogue; test helpers never fetch or infer a missing lock.
fn capture_with_vocabularies(
    source: &[u8],
    vocabularies: Vec<neutral_compiler::CapturedVocabularyInput>,
) -> CapturedExpressionProject {
    capture_sources(
        vec![CapturedSourceInput::new(
            "source:pipeline",
            "pipeline",
            source.to_vec(),
        )],
        vocabularies,
    )
}

/// Multi-unit capture shares the same independent controls as single-file pipeline cases.
fn capture_sources(
    sources: Vec<CapturedSourceInput>,
    vocabularies: Vec<neutral_compiler::CapturedVocabularyInput>,
) -> CapturedExpressionProject {
    capture_expression_project(CapturedExpressionProjectRequest::new(
        capture_envelope(
            sources,
            vocabularies,
            profile::CAPTURE_REQUEST_VERSION,
            CancellationToken::new(),
        ),
        SemanticProfiles::try_frozen().unwrap(),
        composition_limits(),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
    .unwrap()
}

/// Shares only explicit capture plumbing; literal corpus controls and results remain independent.
fn capture_envelope(
    sources: Vec<CapturedSourceInput>,
    vocabularies: Vec<neutral_compiler::CapturedVocabularyInput>,
    version: &str,
    cancellation: CancellationToken,
) -> CapturedProjectRequest {
    let bytes = 1_048_576;
    CapturedProjectRequest::new(
        version,
        LanguageProfile::V1_0,
        sources,
        vocabularies,
        ProjectCaptureControls::new(
            ProjectCaptureLimits::new(ProjectCaptureLimitValues {
                total_source_bytes: bytes,
                source_bytes_per_unit: bytes,
                source_units: 8,
                source_id_bytes: 256,
                module_id_bytes: 256,
                vocabulary_units: 8,
                vocabulary_bytes_per_unit: bytes,
                total_vocabulary_bytes: bytes,
                imports_per_module: 16,
                import_edges: 64,
                scc_units: 8,
                declarations: 4096,
                diagnostics: 64,
                output_bytes: bytes,
            }),
            cancellation,
        ),
    )
}

/// Compiles real files, then obtains fresh reader authority only from their encoded bytes.
fn pipeline(source: &[u8]) -> (Vec<u8>, ValidatedExpressionProject) {
    pipeline_from_capture(&capture(source))
}

/// Exercises transport independently of source capture and its explicit vocabulary inputs.
fn pipeline_from_capture(
    captured: &CapturedExpressionProject,
) -> (Vec<u8>, ValidatedExpressionProject) {
    let cancel = CancellationToken::new();
    let ir = compile_expression_project(captured, &cancel).unwrap();
    let limits = SemanticLimits {
        project: ir.limits,
        composition: composition_limits(),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    };
    let produced = ValidatedExpressionProject::from_ir(ir, limits, &cancel).unwrap();
    let bytes = encode_expression_project(&produced, &cancel).unwrap();
    let decoded = decode_expression_project(&bytes, DecodeLimits::hard(), limits, &cancel).unwrap();
    assert_eq!(produced.ir(), decoded.ir());
    (bytes, decoded)
}

/// Borrows the final materialized value rather than evaluating an expression implicitly.
fn value<'a>(project: &'a ValidatedExpressionProject, name: &str) -> &'a V {
    let declaration = project
        .ir()
        .declarations
        .iter()
        .find(|d| d.identity == symbol(name))
        .unwrap();
    let Some(ExpressionBody::Value(value)) = &declaration.body else {
        panic!("{name} is not static")
    };
    value
}

/// Independent decimal expectations distinguish exact folding from deterministic rounded division.
#[test]
fn integration_expression_source_wire_static_values_match_literal_results() {
    let (_, project) = pipeline(include_bytes!("fixtures/static.neu"));
    for (name, literal) in [
        ("exact", "0.125"),
        ("third", "0.3333333333333333333333333333333333"),
        ("precedence", "14"),
    ] {
        assert_eq!(
            value(&project, name),
            &V::Number(ExactNumber::from_source(literal, 128, 128).unwrap())
        );
    }
    assert_eq!(value(&project, "short_circuit"), &V::Bool(false));
    assert_eq!(value(&project, "label"), &V::String("count=5".into()));
}

/// Static matrices preserve generated values and checked original application/axis lineage over wire.
#[test]
fn integration_expression_source_wire_expansion_retains_values_and_lineage() {
    let (_, project) = pipeline(include_bytes!("fixtures/expansion.neu"));
    assert_eq!(
        value(&project, "choices__0"),
        &V::Variant {
            tag: "ready".into(),
            payload: Box::new(V::Record(vec![(
                "count".into(),
                Some(V::Number(ExactNumber::from_source("3", 128, 128).unwrap()))
            )]))
        }
    );
    assert!(
        !project
            .ir()
            .declarations
            .iter()
            .any(|d| d.identity == symbol("choices__1"))
    );
    assert_eq!(project.ir().public_templates.len(), 1);
    assert_eq!(project.ir().expansion_attributions.len(), 1);
    assert_eq!(
        project.ir().expansion_attributions[0].original_axis_indices,
        [0]
    );
}

/// Complete decoding preserves public interpretive closure, not private implementation or a schedule.
#[test]
fn integration_expression_decoded_view_retains_guarded_outputs_not_prerequisite_edges() {
    let (_, project) = pipeline(include_bytes!("fixtures/guarded.neu"));
    let cancel = CancellationToken::new();
    let owner = symbol("entry");
    let edges = project
        .dependencies(&owner, ExpressionInspectionLimits::default(), &cancel)
        .unwrap();
    assert_eq!(edges.edges().len(), 2);
    assert_eq!(edges.edges()[0].node_path, [1, 0]);
    assert_eq!(edges.edges()[0].guards.len(), 0);
    assert_eq!(edges.edges()[1].node_path, [1, 1]);
    assert_eq!(edges.edges()[1].guards[0].node_path, [1, 0]);
    assert_eq!(
        edges.edges()[1].guards[0].condition,
        GuardCondition::Boolean(true)
    );
    let roots = [owner];
    let view = project
        .derive_view_with_limits(
            &ExpressionViewRequest {
                schema: profile::PROJECT_VIEW_SCHEMA,
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            ExpressionViewLimits::default(),
            &cancel,
        )
        .unwrap();
    let names = view
        .declarations()
        .map(|d| d.identity.declaration_name())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "Operation",
            "Plan",
            "Result",
            "enabled",
            "entry",
            "producer"
        ]
    );
    assert_eq!(view.availability_obligations().count(), 1);
    assert!(!format!("{view:?}").contains("do-not-publish"));
}
