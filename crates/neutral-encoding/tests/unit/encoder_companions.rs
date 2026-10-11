// SPDX-License-Identifier: Apache-2.0

//! Wire regressions deliberately bypassing reader authority to construct inconsistent companions.

use super::*;

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
