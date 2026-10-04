// SPDX-License-Identifier: Apache-2.0

//! Hostile complete-project wire validation through the owning codec.

use super::*;

/// Builds an independently valid project without invoking the compiler.
fn fixture() -> ProjectIr {
    let bytes = b"neu \"1.0\"\nmodule sample\npublic num answer = 42\n";
    let module = LogicalModuleIdentity::new(neutral_core::profile::V1_SOURCE_PROFILE, "sample");
    let symbol = ModuleSymbolIdentity::new(module.clone(), "answer");
    let digest = SourceContentDigest::from_bytes(bytes);
    let mut ir = ProjectIr {
        schema: PROJECT_IR_SCHEMA.to_owned(),
        modules: vec![ProjectModule {
            identity: module,
            imports: Vec::new(),
        }],
        declarations: vec![ProjectDeclaration {
            identity: symbol.clone(),
            public: true,
            signature: ProjectPublicSignature::Binding(T::Num),
            value: Some(V::Number(ExactNumber::from_source("42", 64, 64).unwrap())),
            defaults: Vec::new(),
        }],
        vocabulary_records: Vec::new(),
        public_interface: ProjectInterface::with_computed_fingerprint(Vec::new(), Vec::new())
            .unwrap(),
        sources: vec![ProjectSource {
            module: "sample".to_owned(),
            source_id: "unit:sample".to_owned(),
            digest,
            byte_len: bytes.len() as u64,
        }],
        source_maps: vec![ProjectSourceMap {
            declaration: symbol,
            location: SourceLocation::new(digest, ByteSpan::new(24, bytes.len() as u64).unwrap()),
        }],
        provenance: Vec::new(),
        limits: hard_project_limits(),
        resources: ProjectResourceFacts {
            source_units: 1,
            source_bytes: bytes.len() as u64,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: 1,
            import_edges: 0,
            value_nodes: 1,
        },
        vocabulary_sources: Vec::new(),
    };
    ir.public_interface = ir.recompute_public_interface().unwrap();
    ir
}

/// Serializes intentionally invalid content bypassing the public validated encoder.
fn forge(ir: &ProjectIr) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend(write_ir(ir, None).unwrap());
    bytes
}

/// Decodes a wire input under transport and explicit consumer hard bounds.
fn read(bytes: &[u8]) -> Result<ValidatedProject, DecodeError> {
    decode_project(
        bytes,
        DecodeLimits::hard(),
        hard_project_limits(),
        &CancellationToken::new(),
    )
}

/// Every proper prefix and extra trailing byte fails without partial authority.
#[test]
fn security_project_wire_truncation_and_trailing_data() {
    let ir = fixture();
    let bytes = forge(&ir);
    assert_eq!(read(&bytes).unwrap().complete_ir().as_ref(), &ir);
    for end in 0..bytes.len() {
        assert!(read(&bytes[..end]).is_err(), "prefix {end}");
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        read(&trailing).unwrap_err().class(),
        DecodeErrorClass::MalformedCbor
    );
}

/// Unknown/extra/missing fields fail; equivalent integer widths do not change meaning.
#[test]
fn security_project_wire_closed_schema_and_lexical_limits() {
    let bytes = forge(&fixture());
    for header in [0x8b, 0x8d, 0x9f, 0xbf, 0xff] {
        let mut corrupt = bytes.clone();
        corrupt[MAGIC.len()] = header;
        assert!(read(&corrupt).is_err());
    }
    let mut unknown = fixture();
    unknown.schema = "neutral.project-ir/9".to_owned();
    assert_eq!(
        read(&forge(&unknown)).unwrap_err().class(),
        DecodeErrorClass::UnsupportedVersion
    );
    let mut noncanonical = bytes.clone();
    noncanonical.splice(MAGIC.len()..=MAGIC.len(), [0x98, 12]);
    let decoded = read(&noncanonical).unwrap();
    assert_eq!(
        encode_project(&decoded, &CancellationToken::new()).unwrap(),
        bytes
    );
    let mut huge = MAGIC.to_vec();
    huge.extend([0x9b]);
    huge.extend(u64::MAX.to_be_bytes());
    assert_eq!(
        read(&huge).unwrap_err().class(),
        DecodeErrorClass::EncodedSizeLimit
    );
}

/// Source, type, import, public fingerprint and resource forgeries are independently rejected.
#[test]
fn security_project_wire_invalid_companions_and_meaning() {
    let ir = fixture();
    let mut cases = Vec::new();
    let mut c = ir.clone();
    c.resources.value_nodes += 1;
    cases.push((c, DecodeErrorClass::InvalidDerivation));
    let mut c = ir.clone();
    c.source_maps.clear();
    cases.push((c, DecodeErrorClass::InvalidSourceMap));
    let mut c = ir.clone();
    c.source_maps[0].location =
        SourceLocation::new(c.sources[0].digest, ByteSpan::new(0, u64::MAX).unwrap());
    cases.push((c, DecodeErrorClass::InvalidSourceMap));
    let mut c = ir.clone();
    c.declarations[0].value = Some(V::Bool(false));
    cases.push((c, DecodeErrorClass::InvalidLogicalIr));
    let mut c = ir.clone();
    c.declarations.push(c.declarations[0].clone());
    cases.push((c, DecodeErrorClass::InvalidLogicalIr));
    let mut c = ir.clone();
    c.modules[0].imports.push("missing".to_owned());
    cases.push((c, DecodeErrorClass::InvalidLogicalIr));
    let mut c = ir.clone();
    c.modules[0].imports.push("sample".to_owned());
    cases.push((c, DecodeErrorClass::InvalidLogicalIr));
    let mut c = ir.clone();
    c.public_interface = ProjectInterface::from_parts(
        Vec::new(),
        Vec::new(),
        SemanticDigest::from_raw_bytes([0; 32]),
    );
    cases.push((c, DecodeErrorClass::InvalidLogicalIr));
    let mut c = ir.clone();
    c.provenance.push(ProjectProvenance {
        from: c.declarations[0].identity.clone(),
        to: ModuleSymbolIdentity::new(c.modules[0].identity.clone(), "absent"),
        kind: ProjectPublicEdgeKind::Value,
        location: c.source_maps[0].location,
    });
    cases.push((c, DecodeErrorClass::InvalidSourceMap));
    for (corrupt, expected) in cases {
        assert_eq!(read(&forge(&corrupt)).unwrap_err().class(), expected);
    }
}

/// Every lexical caller bound intersects hard bounds; exact byte ceilings pass.
#[test]
fn security_project_wire_bounds_and_cancellation() {
    let ir = fixture();
    let bytes = forge(&ir);
    let token = CancellationToken::new();
    assert!(
        decode_project(
            &bytes,
            DecodeLimits::hard().with_artifact_bytes(bytes.len()),
            ir.limits,
            &token
        )
        .is_ok()
    );
    assert!(
        decode_project(
            &bytes,
            DecodeLimits::hard().with_section_bytes(bytes.len() - MAGIC.len()),
            ir.limits,
            &token
        )
        .is_ok()
    );
    for wire in [
        DecodeLimits::hard().with_artifact_bytes(bytes.len() - 1),
        DecodeLimits::hard().with_section_bytes(bytes.len() - MAGIC.len() - 1),
        DecodeLimits::hard().with_container_items(1),
        DecodeLimits::hard().with_text_bytes(1),
        DecodeLimits::hard().with_byte_string_bytes(31),
        DecodeLimits::hard().with_nesting_depth(1),
        DecodeLimits::hard().with_traversal_nodes(1),
    ] {
        assert_eq!(
            decode_project(&bytes, wire, ir.limits, &token)
                .unwrap_err()
                .class(),
            DecodeErrorClass::EncodedSizeLimit
        );
    }
    for limits in [
        ProjectLimits {
            modules: 0,
            ..ir.limits
        },
        ProjectLimits {
            declarations: 0,
            ..ir.limits
        },
        ProjectLimits {
            nodes: 0,
            ..ir.limits
        },
        ProjectLimits {
            text_bytes: 0,
            ..ir.limits
        },
    ] {
        assert_eq!(
            decode_project(&bytes, DecodeLimits::hard(), limits, &token)
                .unwrap_err()
                .class(),
            DecodeErrorClass::EncodedSizeLimit
        );
    }
    let reader = read(&bytes).unwrap();
    token.cancel();
    assert_eq!(
        encode_project(&reader, &token),
        Err(EncodingError::Cancelled)
    );
    assert_eq!(
        decode_project(&bytes, DecodeLimits::hard(), ir.limits, &token)
            .unwrap_err()
            .class(),
        DecodeErrorClass::Cancelled
    );
}

/// Unknown discriminators and duplicate record fields are retained then rejected.
#[test]
fn security_project_wire_unknown_tags_and_duplicate_fields() {
    let ir = fixture();
    let bytes = forge(&ir);
    let mut unknown = bytes.clone();
    let position = unknown.windows(3).position(|s| s == b"num").unwrap();
    unknown[position..position + 3].copy_from_slice(b"bad");
    assert_eq!(
        read(&unknown).unwrap_err().class(),
        DecodeErrorClass::InvalidEncodedSchema
    );
    let mut corrupt = ir;
    corrupt.declarations[0].signature = ProjectPublicSignature::Record(vec![
        ProjectPublicField::new("field", T::Num),
        ProjectPublicField::new("field", T::Num),
    ]);
    corrupt.declarations[0].value = None;
    assert_eq!(
        read(&forge(&corrupt)).unwrap_err().class(),
        DecodeErrorClass::InvalidLogicalIr
    );
}

/// Arbitrary single-byte mutations always either fail or publish a revalidated project.
#[test]
fn property_project_wire_mutations_never_bypass_validation() {
    let bytes = forge(&fixture());
    for position in 0..bytes.len() {
        let mut mutated = bytes.clone();
        mutated[position] ^= 0xff;
        if let Ok(reader) = read(&mutated) {
            ValidatedProject::from_ir(
                Arc::clone(reader.complete_ir()),
                hard_project_limits(),
                &CancellationToken::new(),
            )
            .unwrap();
        }
    }
}

/// Captured output bytes are an independent producer cap, not a value-node heuristic.
#[test]
fn security_project_wire_captured_output_boundary() {
    let mut ir = fixture();
    for _ in 0..3 {
        ir.limits.artifact_bytes = forge(&ir).len() as u64;
    }
    let bytes = forge(&ir);
    assert_eq!(bytes.len() as u64, ir.limits.artifact_bytes);
    let reader = read(&bytes).unwrap();
    assert_eq!(
        encode_project(&reader, &CancellationToken::new()).unwrap(),
        bytes
    );
    let mut smaller = ir;
    smaller.limits.artifact_bytes -= 1;
    assert_eq!(
        read(&forge(&smaller)).unwrap_err().class(),
        DecodeErrorClass::EncodedSizeLimit
    );
    let reader = ValidatedProject::from_ir(
        Arc::new(smaller),
        hard_project_limits(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(
        encode_project(&reader, &CancellationToken::new()),
        Err(EncodingError::EncodedSizeLimit)
    );
}

/// Untrusted original-source IDs obey caller text limits just like schema names.
#[test]
fn security_project_wire_source_ids_and_nested_depth() {
    let mut ir = fixture();
    ir.limits.text_bytes = ir.resources.source_bytes;
    ir.sources[0].source_id = "x".repeat(usize::try_from(ir.limits.text_bytes).unwrap() + 1);
    assert_eq!(
        read(&forge(&ir)).unwrap_err().class(),
        DecodeErrorClass::InvalidSourceMap
    );
    let mut ir = fixture();
    let mut ty = T::Num;
    for _ in 0..=PROJECT_MAX_DEPTH {
        ty = T::List(Box::new(ty));
    }
    ir.declarations[0].signature = ProjectPublicSignature::Binding(ty);
    assert_eq!(write_ir(&ir, None), Err(EncodingError::EncodedSizeLimit));
    let mut w = CborWriter::new();
    for _ in 0..=crate::constants::MAXIMUM_NESTING_DEPTH {
        w.array(1).unwrap();
    }
    w.null().unwrap();
    let mut bytes = MAGIC.to_vec();
    bytes.extend(w.finish().unwrap());
    assert_eq!(
        read(&bytes).unwrap_err().class(),
        DecodeErrorClass::EncodedSizeLimit
    );
}

/// Selected vocabulary values retain transitive interpretation schemas but not private types.
#[test]
fn integration_project_wire_transitive_vocabulary_view() {
    let mut ir = fixture();
    let nominal = |name: &str| T::VocabularyNominal {
        identity: "Catalogue".to_owned(),
        version: "1.0.0".to_owned(),
        name: name.to_owned(),
    };
    let record = |name: &str, public, fields| ProjectVocabularyRecord {
        identity: "Catalogue".to_owned(),
        version: "1.0.0".to_owned(),
        name: name.to_owned(),
        public,
        fields,
    };
    ir.vocabulary_records = vec![
        record("Hidden", false, Vec::new()),
        record("Inner", true, vec![("label".to_owned(), T::String)]),
        record("Outer", true, vec![("nested".to_owned(), nominal("Inner"))]),
    ];
    ir.declarations[0].signature = ProjectPublicSignature::Binding(nominal("Outer"));
    ir.declarations[0].value = Some(V::Record(vec![(
        "nested".to_owned(),
        V::Record(vec![("label".to_owned(), V::String("visible".to_owned()))]),
    )]));
    ir.public_interface = ProjectInterface::from_parts_with_vocabularies(
        vec![ProjectPublicVocabulary::new(
            "Catalogue",
            "1.0.0",
            vec!["Inner".to_owned(), "Outer".to_owned()],
        )],
        Vec::new(),
        Vec::new(),
        ir.public_interface.fingerprint(),
    );
    ir.public_interface = ir.recompute_public_interface().unwrap();
    ir.vocabulary_sources = vec![ProjectVocabularySource {
        identity: "Catalogue".to_owned(),
        version: "1.0.0".to_owned(),
        digest: VocabularyContentDigest::from_bytes(b"captured vocabulary evidence"),
        byte_len: 28,
    }];
    ir.resources.vocabulary_units = 1;
    ir.resources.vocabulary_bytes = 28;
    ir.resources.value_nodes = 3;
    let decoded = read(&forge(&ir)).unwrap();
    let view = decoded
        .derive_view(
            &neutral_ir::project::ViewRequest {
                schema: neutral_ir::project::PROJECT_VIEW_SCHEMA.to_owned(),
                roots: vec![ir.declarations[0].identity.clone()],
            },
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(
        view.vocabulary_records()
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["Inner", "Outer"]
    );
    assert_eq!(decoded.complete_ir().as_ref(), &ir);
    assert_eq!(
        encode_project(&decoded, &CancellationToken::new()).unwrap(),
        forge(&ir)
    );
}
