// SPDX-License-Identifier: Apache-2.0

//! Wire regressions deliberately bypassing reader authority to construct inconsistent companions.

use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// An extra derivation vocabulary remains `InvalidDerivation`, not a logical/schema failure.
#[test]
fn hostile_derivation_vocabulary_without_logical_contract_keeps_error_class() {
    let request = neutral_compiler::CompilationRequest::new(
        b"neu \"0.1\"\nmodule example\nnum count = 42\n".to_vec(),
        neutral_core::StructuralLimits::new(4096, 64).unwrap(),
        neutral_core::CancellationToken::new(),
    );
    let Ok(neutral_compiler::CompilationResult::Success(original)) =
        neutral_compiler::compile(request)
    else {
        panic!("valid source");
    };
    let identity = VocabularyIdentity::new(
        "Fixture",
        env!("CARGO_PKG_VERSION"),
        neutral_vocabulary::VOCABULARY_SCHEMA_VERSION,
        neutral_vocabulary::VOCABULARY_ENCODING_VERSION,
        neutral_core::VocabularyContentDigest::from_bytes(b"fixture"),
        Vec::new(),
    );
    let artifacts = CompilationArtifacts::new(
        original.logical_document().clone(),
        original.source_map().clone(),
        original.provenance().to_vec(),
        original.derivation().clone().with_vocabulary(identity),
    );
    assert_wire_error(&artifacts, crate::DecodeErrorClass::InvalidDerivation);
}

/// Duplicate logical IDs retain the companion error class before semantic rejection.
#[test]
fn hostile_duplicate_logical_ids_keep_source_map_error_class() {
    let request = neutral_compiler::CompilationRequest::new(
        b"neu \"0.1\"\nmodule example\nnum first = 1\nnum second = 2\n".to_vec(),
        neutral_core::StructuralLimits::new(4096, 64).unwrap(),
        neutral_core::CancellationToken::new(),
    );
    let Ok(neutral_compiler::CompilationResult::Success(original)) =
        neutral_compiler::compile(request)
    else {
        panic!("valid source");
    };
    let logical = original.logical_document();
    let mut declarations = logical.declarations().to_vec();
    let second = &declarations[1];
    declarations[1] = Declaration::new(
        declarations[0].element_id(),
        second.symbol_identity().clone(),
        second.fingerprint(),
        second.name(),
        second.resolved_type().clone(),
        second.value().clone(),
    );
    let artifacts = CompilationArtifacts::new(
        LogicalDocument::with_record_types(
            logical.module().clone(),
            logical.record_types().to_vec(),
            declarations,
        ),
        original.source_map().clone(),
        original.provenance().to_vec(),
        original.derivation().clone(),
    );
    assert_wire_error(&artifacts, crate::DecodeErrorClass::InvalidSourceMap);
}

#[test]
fn logical_encoder_rejects_coefficients_over_the_wire_limit() {
    let module = LogicalModuleIdentity::new(neutral_ir::LANGUAGE_BEHAVIOR_VERSION, "example");
    let coefficient = "1".repeat(crate::constants::MAXIMUM_EXACT_NUMBER_DIGITS + 1);
    let number = neutral_ir::ExactNumber::from_normalized_parts(
        false,
        &coefficient,
        0,
        u64::try_from(coefficient.len()).unwrap(),
        0,
    )
    .unwrap();
    let value = LogicalValue::Number(number);
    let ty = ResolvedType::Num;
    let declaration = Declaration::new(
        neutral_ir::ElementId::new(1),
        ModuleSymbolIdentity::new(module.clone(), "answer"),
        neutral_ir::DeclarationFingerprint::for_binding(&ty, &value).unwrap(),
        "answer",
        ty,
        value,
    );
    let document = LogicalDocument::new(module, vec![declaration]);

    assert!(matches!(
        encode_logical(&document),
        Err(EncodingError::EncodedSizeLimit)
    ));
}

#[test]
fn logical_encoder_checks_recursive_depth_at_every_node_kind() {
    let module = LogicalModuleIdentity::new(neutral_ir::LANGUAGE_BEHAVIOR_VERSION, "example");
    let nominal = NominalTypeIdentity::new(module.clone(), "Person");
    let vocabulary_type = VocabularyTypeIdentity::new("Fixture", "Person");

    let mut deep_type = ResolvedType::Num;
    for _ in 0..125 {
        deep_type = ResolvedType::list(deep_type);
    }
    assert_logical_depth_rejected(&module, deep_type, LogicalValue::Null);

    let mut nominal_type = ResolvedType::Record(nominal.clone());
    for _ in 0..123 {
        nominal_type = ResolvedType::list(nominal_type);
    }
    let mut vocabulary_resolved_type = ResolvedType::VocabularyRecord(vocabulary_type.clone());
    for _ in 0..124 {
        vocabulary_resolved_type = ResolvedType::list(vocabulary_resolved_type);
    }
    assert_logical_depth_rejected(&module, nominal_type, LogicalValue::Null);
    assert_logical_depth_rejected(&module, vocabulary_resolved_type, LogicalValue::Null);

    let mut nested_list = LogicalValue::Null;
    for _ in 0..63 {
        nested_list = LogicalValue::List(vec![nested_list]);
    }
    assert_logical_depth_rejected(&module, ResolvedType::Num, nested_list);

    let mut nested_record = LogicalValue::Null;
    let mut nested_vocabulary_record = LogicalValue::Null;
    for _ in 0..50 {
        nested_record = LogicalValue::Record(neutral_ir::RecordValue::new(
            nominal.clone(),
            vec![neutral_ir::RecordValueField::new("child", nested_record)],
        ));
        nested_vocabulary_record =
            LogicalValue::VocabularyRecord(neutral_ir::VocabularyRecordValue::new(
                vocabulary_type.clone(),
                vec![neutral_ir::RecordValueField::new(
                    "child",
                    nested_vocabulary_record,
                )],
            ));
    }
    assert_logical_depth_rejected(&module, ResolvedType::Num, nested_record);
    assert_logical_depth_rejected(&module, ResolvedType::Num, nested_vocabulary_record);

    let mut nested_vocabulary_leaf = LogicalValue::VocabularyRecord(
        neutral_ir::VocabularyRecordValue::new(vocabulary_type.clone(), Vec::new()),
    );
    for _ in 0..62 {
        nested_vocabulary_leaf = LogicalValue::List(vec![nested_vocabulary_leaf]);
    }
    assert_logical_depth_rejected(&module, ResolvedType::Num, nested_vocabulary_leaf);

    let reference = LogicalValue::Reference(neutral_ir::IdentityReference::new(
        neutral_ir::ElementId::new(1),
        ModuleSymbolIdentity::new(module.clone(), "target"),
        ResolvedType::Num,
    ));
    let mut nested_reference = reference;
    for _ in 0..62 {
        nested_reference = LogicalValue::List(vec![nested_reference]);
    }
    assert_logical_depth_rejected(&module, ResolvedType::Num, nested_reference);
}

fn assert_logical_depth_rejected(
    module: &LogicalModuleIdentity,
    resolved_type: ResolvedType,
    value: LogicalValue,
) {
    let description = format!("{resolved_type:?} {value:?}");
    let fingerprint = neutral_ir::DeclarationFingerprint::for_binding(&resolved_type, &value)
        .expect("test fingerprint should fit");
    let declaration = Declaration::new(
        neutral_ir::ElementId::new(1),
        ModuleSymbolIdentity::new(module.clone(), "deep"),
        fingerprint,
        "deep",
        resolved_type,
        value,
    );
    if !matches!(
        encode_logical(&LogicalDocument::new(module.clone(), vec![declaration])),
        Err(EncodingError::EncodedSizeLimit)
    ) {
        panic!("depth limit should reject {description}");
    }
}

#[test]
fn frame_encoder_rejects_a_section_over_the_artifact_limit() {
    let sections = [
        Vec::new(),
        vec![0; crate::constants::MAXIMUM_ARTIFACT_BYTES + 1],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];

    assert!(matches!(
        assemble_frame(0, sections),
        Err(EncodingError::EncodedSizeLimit)
    ));
}

#[test]
fn section_encoders_propagate_every_writer_size_failure() {
    let request = neutral_compiler::CompilationRequest::new(
        b"neu \"0.1\"\nmodule example\nnum count = 42\n".to_vec(),
        neutral_core::StructuralLimits::new(4096, 64).unwrap(),
        neutral_core::CancellationToken::new(),
    );
    let Ok(neutral_compiler::CompilationResult::Success(artifacts)) =
        neutral_compiler::compile(request)
    else {
        panic!("valid source");
    };
    let element_id = artifacts.logical_document().declarations()[0].element_id();
    let artifacts = artifacts
        .as_ref()
        .clone()
        .with_field_provenance(vec![FieldProvenanceRecord::new(
            element_id,
            vec!["field".to_owned()],
            neutral_ir::ValueOrigin::ExplicitRecordField,
        )])
        .with_reuse_provenance(vec![ReuseProvenanceRecord::new(
            element_id,
            vec!["reuse".to_owned()],
            element_id,
        )])
        .with_reference_provenance(vec![ReferenceProvenanceRecord::new(
            element_id,
            vec!["reference".to_owned()],
            element_id,
        )]);
    let mut budget = TraversalBudget::new();
    let capabilities = derive_capabilities(&artifacts, &mut budget).unwrap();
    let logical = encode_logical(artifacts.logical_document()).unwrap();
    let source_map = encode_source_map(artifacts.source_map()).unwrap();
    let provenance = encode_provenance(&artifacts).unwrap();
    let derivation = encode_derivation(artifacts.derivation()).unwrap();
    let producer = ProducerInfo::new("test", env!("CARGO_PKG_VERSION")).with_build("fixture");
    let envelope = encode_envelope(
        &artifacts,
        &producer,
        capabilities,
        [&logical, &source_map, &provenance, &derivation],
    )
    .unwrap();

    assert_writer_limit_sweep(logical.len(), || {
        encode_logical(artifacts.logical_document())
    });
    for fixture in positive_fixtures() {
        let source = fs::read(&fixture).unwrap();
        let vocabulary = fixture
            .components()
            .any(|part| part.as_os_str() == "vocabulary");
        let fixture_artifacts = compile_fixture(&source, vocabulary);
        let fixture_logical = encode_logical(fixture_artifacts.logical_document()).unwrap();
        assert_writer_limit_sweep(fixture_logical.len(), || {
            encode_logical(fixture_artifacts.logical_document())
        });
    }
    assert_writer_limit_sweep(source_map.len(), || {
        encode_source_map(artifacts.source_map())
    });
    assert_writer_limit_sweep(provenance.len(), || encode_provenance(&artifacts));
    assert_writer_limit_sweep(derivation.len(), || {
        encode_derivation(artifacts.derivation())
    });
    let vocabulary_identity = VocabularyIdentity::new(
        "Fixture",
        "0.1.0",
        neutral_vocabulary::VOCABULARY_SCHEMA_VERSION,
        neutral_vocabulary::VOCABULARY_ENCODING_VERSION,
        neutral_core::VocabularyContentDigest::from_bytes(b"fixture"),
        Vec::new(),
    );
    let vocabulary_derivation = artifacts
        .derivation()
        .clone()
        .with_vocabulary(vocabulary_identity);
    let vocabulary_derivation_bytes = encode_derivation(&vocabulary_derivation).unwrap();
    assert_writer_limit_sweep(vocabulary_derivation_bytes.len(), || {
        encode_derivation(&vocabulary_derivation)
    });
    assert_writer_limit_sweep(envelope.len(), || {
        encode_envelope(
            &artifacts,
            &producer,
            capabilities,
            [&logical, &source_map, &provenance, &derivation],
        )
    });
}

fn assert_writer_limit_sweep(
    encoded_length: usize,
    encode: impl Fn() -> Result<Vec<u8>, EncodingError>,
) {
    for limit in 0..encoded_length {
        assert!(matches!(
            CborWriter::with_test_section_limit(limit, &encode),
            Err(EncodingError::EncodedSizeLimit)
        ));
    }
}

fn positive_fixtures() -> Vec<PathBuf> {
    fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "neu") {
                files.push(path);
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let mut files = Vec::new();
    collect(
        &root.join("conformance/releases/v0.1.0/specs/fixtures/positive"),
        &mut files,
    );
    files.sort();
    files
}

fn compile_fixture(source: &[u8], with_vocabulary: bool) -> CompilationArtifacts {
    let limits = neutral_core::StructuralLimits::new(16_384, 16).unwrap();
    let request = neutral_compiler::CompilationRequest::new(
        source.to_vec(),
        limits,
        neutral_core::CancellationToken::new(),
    );
    let request = if with_vocabulary {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let bundle = fs::read(root.join(
            "conformance/releases/v0.1.0/specs/fixtures/vocabulary/bundles/positive/comprehensive.json",
        ))
        .unwrap();
        let lock = neutral_vocabulary::VocabularyLock::new(
            "Fixture",
            "0.1.0",
            neutral_vocabulary::VOCABULARY_ENCODING_VERSION,
            neutral_vocabulary::VOCABULARY_SCHEMA_VERSION,
            neutral_core::VocabularyContentDigest::from_bytes(&bundle),
            Vec::new(),
        )
        .unwrap();
        request.with_captured_vocabulary(bundle, lock)
    } else {
        request
    };
    let Ok(neutral_compiler::CompilationResult::Success(artifacts)) =
        neutral_compiler::compile(request)
    else {
        panic!("positive fixture must compile");
    };
    artifacts.as_ref().clone()
}

/// Encodes raw parts without reader authority and checks the independent decoder classification.
fn assert_wire_error(artifacts: &CompilationArtifacts, expected: crate::DecodeErrorClass) {
    let mut budget = TraversalBudget::new();
    let capabilities = derive_capabilities(artifacts, &mut budget).unwrap();
    let logical = encode_logical(artifacts.logical_document()).unwrap();
    let source = encode_source_map(artifacts.source_map()).unwrap();
    let provenance = encode_provenance(artifacts).unwrap();
    let derivation = encode_derivation(artifacts.derivation()).unwrap();
    // Frozen captured-vocabulary bit: companion failures precede a stale capability mask too.
    for extra_capabilities in [0, capability(5)] {
        let mask = capabilities | extra_capabilities;
        let envelope = encode_envelope(
            artifacts,
            &ProducerInfo::new("test", env!("CARGO_PKG_VERSION")),
            mask,
            [&logical, &source, &provenance, &derivation],
        )
        .unwrap();
        let encoded = assemble_frame(
            mask,
            [
                envelope,
                logical.clone(),
                source.clone(),
                provenance.clone(),
                derivation.clone(),
            ],
        )
        .unwrap();
        assert_eq!(
            crate::decode(
                encoded.as_bytes(),
                crate::DecodeLimits::hard(),
                &neutral_core::CancellationToken::new()
            )
            .unwrap_err()
            .class(),
            expected
        );
    }
}
