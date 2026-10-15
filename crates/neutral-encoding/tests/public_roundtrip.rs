// SPDX-License-Identifier: Apache-2.0

//! Public codec entry points, linked through the package library.

use neutral_compiler::{CompilationRequest, CompilationResult, capture, compile_captured};
use neutral_core::{CancellationToken, StructuralLimits};
use neutral_encoding::{
    DecodeLimits, ProducerInfo, decode, decode_expression_project, encode,
    encode_expression_project,
};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    expression::{profile::ExpressionLimits, templates::ExpansionLimits},
    project::{
        PROJECT_IR_SCHEMA, ProjectDeclaration, ProjectIr, ProjectLimits, ProjectModule,
        ProjectResourceFacts, ProjectSource, ProjectSourceMap, ProjectValue,
    },
    project_interface::{
        ProjectInterface, ProjectPublicField, ProjectPublicSignature as Signature,
        ProjectPublicType,
    },
};
use neutral_reader::{SemanticLimits, ValidatedDocument, ValidatedProject};
use std::sync::Arc;

fn semantic_limits() -> SemanticLimits {
    SemanticLimits {
        project: ProjectLimits {
            modules: 1_000,
            declarations: 1_000,
            import_edges: 1_000,
            nodes: 10_000,
            text_bytes: 1_048_576,
            artifact_bytes: 1_048_576,
        },
        composition: neutral_vocabulary::composition::CompositionLimits::from_vocabulary(
            neutral_vocabulary::VocabularyLimits::from_structural(
                StructuralLimits::new(1_048_576, 64).unwrap(),
            ),
        ),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    }
}

#[test]
fn public_legacy_codec_roundtrips_validated_input() {
    let source = b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n";
    let captured = capture(CompilationRequest::new(
        source.to_vec(),
        StructuralLimits::new(16_384, 16).unwrap(),
        CancellationToken::new(),
    ))
    .unwrap();
    let CompilationResult::Success(artifacts) = compile_captured(&captured) else {
        panic!("fixture must compile");
    };
    let document = ValidatedDocument::from_compiler_output(artifacts).unwrap();
    let encoded = encode(&document, &ProducerInfo::new("test", "0.8.0")).unwrap();
    let decoded = decode(
        encoded.as_bytes(),
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert!(document.logically_equivalent(&decoded));
}

#[test]
fn public_expression_codec_roundtrips_frozen_artifact() {
    for fixture in [
        include_str!("expression/fixtures/static.hex"),
        include_str!("expression/fixtures/residual.hex"),
        include_str!("expression/fixtures/template.hex"),
    ] {
        let bytes = fixture
            .trim()
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        let cancel = CancellationToken::new();
        let project =
            decode_expression_project(&bytes, DecodeLimits::hard(), semantic_limits(), &cancel)
                .unwrap();
        assert_eq!(encode_expression_project(&project, &cancel).unwrap(), bytes);
    }
}

/// Builds complete project IR covering every public scalar and list variant.
fn project_ir_fixture() -> ProjectIr {
    let source = b"neu \"1.0\"\nmodule sample\npublic num answer = 42\n";
    let module = LogicalModuleIdentity::new("1.0", "sample");
    let declarations = [
        (
            "answer",
            ProjectPublicType::Num,
            ProjectValue::Number(ExactNumber::from_source("-42", 64, 64).unwrap()),
        ),
        ("enabled", ProjectPublicType::Bool, ProjectValue::Bool(true)),
        (
            "path",
            ProjectPublicType::Path,
            ProjectValue::Path("/tmp/value".into()),
        ),
        (
            "text",
            ProjectPublicType::String,
            ProjectValue::String("hello".into()),
        ),
        (
            "url",
            ProjectPublicType::Url,
            ProjectValue::Url("https://example.invalid".into()),
        ),
        (
            "values",
            ProjectPublicType::List(Box::new(ProjectPublicType::Num)),
            ProjectValue::List(vec![
                ProjectValue::Number(ExactNumber::from_source("1", 64, 64).unwrap()),
                ProjectValue::Number(ExactNumber::from_source("2", 64, 64).unwrap()),
            ]),
        ),
    ];
    let mut project_declarations = declarations
        .iter()
        .map(|(name, ty, value)| ProjectDeclaration {
            identity: ModuleSymbolIdentity::new(module.clone(), *name),
            public: true,
            signature: Signature::Binding(ty.clone()),
            value: Some(value.clone()),
            defaults: Vec::new(),
        })
        .collect::<Vec<_>>();
    let record_type = ModuleSymbolIdentity::new(module.clone(), "Container");
    project_declarations.extend([ProjectDeclaration {
        identity: record_type.clone(),
        public: true,
        signature: Signature::Record(vec![
            ProjectPublicField::new("count", ProjectPublicType::Num),
            ProjectPublicField::new("name", ProjectPublicType::String),
        ]),
        value: None,
        defaults: Vec::new(),
    }]);
    project_declarations.sort_by(|a, b| a.identity.cmp(&b.identity));
    let digest = neutral_core::SourceContentDigest::from_bytes(source);
    let limits = neutral_encoding::project::hard_project_limits();
    let mut ir = ProjectIr {
        schema: PROJECT_IR_SCHEMA.to_owned(),
        modules: vec![ProjectModule {
            identity: module.clone(),
            imports: Vec::new(),
        }],
        declarations: project_declarations.clone(),
        vocabulary_records: Vec::new(),
        public_interface: ProjectInterface::with_computed_fingerprint(Vec::new(), Vec::new())
            .unwrap(),
        sources: vec![ProjectSource {
            module: "sample".into(),
            source_id: "test:sample".into(),
            digest,
            byte_len: source.len() as u64,
        }],
        source_maps: project_declarations
            .iter()
            .map(|declaration| ProjectSourceMap {
                declaration: declaration.identity.clone(),
                location: neutral_core::SourceLocation::new(
                    digest,
                    neutral_core::ByteSpan::new(24, source.len() as u64).unwrap(),
                ),
            })
            .collect(),
        provenance: Vec::new(),
        limits,
        resources: ProjectResourceFacts {
            source_units: 1,
            source_bytes: source.len() as u64,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: project_declarations.len() as u64,
            import_edges: 0,
            value_nodes: 8,
        },
        vocabulary_sources: Vec::new(),
    };
    ir.public_interface = ir.recompute_public_interface().unwrap();
    ir
}

#[test]
fn public_project_codec_roundtrips_complete_ir() {
    let ir = project_ir_fixture();
    let limits = neutral_encoding::project::hard_project_limits();
    let cancel = CancellationToken::new();
    let project = ValidatedProject::from_ir(Arc::new(ir), limits, &cancel).unwrap();
    let bytes = neutral_encoding::project::encode_project(&project, &cancel).unwrap();
    let decoded =
        neutral_encoding::project::decode_project(&bytes, DecodeLimits::hard(), limits, &cancel)
            .unwrap();
    assert_eq!(decoded.complete_ir(), project.complete_ir());

    let mut stale_resources = bytes;
    let end = stale_resources
        .windows(2)
        .rposition(|pair| pair == [8, 0x80])
        .unwrap();
    stale_resources[end] = 7;
    assert_eq!(
        neutral_encoding::project::decode_project(
            &stale_resources,
            DecodeLimits::hard(),
            limits,
            &cancel,
        )
        .unwrap_err()
        .class(),
        neutral_encoding::DecodeErrorClass::InvalidDerivation
    );
}
