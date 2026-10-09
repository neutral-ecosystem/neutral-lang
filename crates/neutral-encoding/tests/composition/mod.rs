// SPDX-License-Identifier: Apache-2.0

//! Owning-crate checks for exact composition tuples, bounds and cancellation.

use super::*;
use neutral_core::{SourceLocation, StructuralLimits, allocation::testing};
use neutral_ir::{
    LogicalModuleIdentity, project_identity, project_interface::ProjectPublicType as T,
};
use neutral_vocabulary::VocabularyLimits;

/// Supplies positive independent composition budgets.
fn policy() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ))
}

/// Builds a complete source-only project without compiler coupling.
fn fixture() -> Ir {
    let source = b"neu \"1.0\"\nmodule example\n";
    let cancel = CancellationToken::new();
    let mut ir = Ir {
        schema: profile::PROJECT_IR_SCHEMA.to_owned(),
        modules: vec![ProjectModule {
            identity: LogicalModuleIdentity::new(
                neutral_core::profile::V1_SOURCE_PROFILE,
                "example",
            ),
            imports: vec![],
        }],
        declarations: vec![],
        vocabularies: vec![],
        interface_digest: SemanticDigest::from_raw_bytes([0; 32]),
        sources: vec![ProjectSource {
            module: "example".into(),
            source_id: "source:example".into(),
            digest: SourceContentDigest::from_bytes(source),
            byte_len: source.len() as u64,
        }],
        source_maps: vec![],
        provenance: vec![],
        limits: crate::project::hard_project_limits(),
        resources: ProjectResourceFacts {
            source_units: 1,
            source_bytes: source.len() as u64,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: 0,
            import_edges: 0,
            value_nodes: 0,
        },
        vocabulary_sources: vec![],
        origins: vec![],
        composition_limits: policy().policy(),
        composition_resources: CompositionResourceFacts::from_values([0; 7]),
    };
    ir.interface_digest = project_identity::composition_interface(
        &ir,
        project_identity::IdentityLimits {
            bytes: project_identity::MAX_TRANSCRIPT_BYTES,
            nodes: 65_536,
        },
        &cancel,
    )
    .unwrap()
    .identity();
    ir
}

/// Serializes producer data directly to exercise independent hostile-input checks.
fn forge(ir: &Ir) -> Vec<u8> {
    let mut writer = CborWriter::new();
    write_ir(&mut writer, ir).unwrap();
    let mut bytes = profile::MAGIC.to_vec();
    bytes.extend(writer.finish().unwrap());
    bytes
}

/// Grants authority only after the actual complete reader accepts the fixture.
fn validated(ir: Ir) -> ValidatedCompositionProject {
    let limits = ir.limits;
    ValidatedCompositionProject::from_ir(
        Arc::try_new(ir).unwrap(),
        limits,
        policy(),
        &CancellationToken::new(),
    )
    .unwrap()
}

/// Exact artifact and payload ceilings accept equality and reject the preceding byte.
#[test]
fn composition_exact_transport_bounds() {
    let cancel = CancellationToken::new();
    let mut ir = fixture();
    // The advertised ceiling's own CBOR width contributes to the artifact size.
    loop {
        let length = forge(&ir).len() as u64;
        if ir.limits.artifact_bytes == length {
            break;
        }
        ir.limits.artifact_bytes = length;
    }
    let project = validated(ir.clone());
    let bytes = forge(&ir);
    assert_eq!(
        encode_composition_project(&project, &cancel).unwrap(),
        bytes
    );
    for wire in [
        DecodeLimits::hard().with_artifact_bytes(bytes.len()),
        DecodeLimits::hard().with_section_bytes(bytes.len() - profile::MAGIC.len()),
    ] {
        assert!(decode_composition_project(&bytes, wire, ir.limits, policy(), &cancel).is_ok());
    }
    for wire in [
        DecodeLimits::hard().with_artifact_bytes(bytes.len() - 1),
        DecodeLimits::hard().with_section_bytes(bytes.len() - profile::MAGIC.len() - 1),
    ] {
        assert_eq!(
            decode_composition_project(&bytes, wire, ir.limits, policy(), &cancel)
                .unwrap_err()
                .class(),
            C::EncodedSizeLimit
        );
    }
    let lower = ProjectLimits {
        artifact_bytes: bytes.len() as u64 - 1,
        ..ir.limits
    };
    assert_eq!(
        decode_composition_project(&bytes, DecodeLimits::hard(), lower, policy(), &cancel)
            .unwrap_err()
            .class(),
        C::EncodedSizeLimit
    );
    ir.limits.artifact_bytes -= 1;
    assert_eq!(
        decode_composition_project(
            &forge(&ir),
            DecodeLimits::hard(),
            crate::project::hard_project_limits(),
            policy(),
            &cancel
        )
        .unwrap_err()
        .class(),
        C::EncodedSizeLimit
    );
    assert_eq!(
        encode_composition_project(&validated(ir), &cancel),
        Err(E::EncodedSizeLimit)
    );
}

/// Every allocation boundary observes real cancellation before publishing any artifact.
#[test]
fn composition_cancellation_at_each_retention_boundary() {
    let project = validated(fixture());
    let cancel = CancellationToken::new();
    let (encoded, count) = testing::observe(None, || encode_composition_project(&project, &cancel));
    let bytes = encoded.unwrap();
    for index in 0..count {
        let cancel = CancellationToken::new();
        let (result, _) = testing::observe_cancellation(index, &cancel, || {
            encode_composition_project(&project, &cancel)
        });
        assert_eq!(result, Err(E::Cancelled), "encode allocation {index}");
    }
    let decode = |cancel: &CancellationToken| {
        decode_composition_project(
            &bytes,
            DecodeLimits::hard(),
            project.complete_ir().limits,
            policy(),
            cancel,
        )
    };
    let (decoded, count) = testing::observe(None, || decode(&CancellationToken::new()));
    assert!(decoded.is_ok());
    for index in 0..count {
        let cancel = CancellationToken::new();
        let (result, _) = testing::observe_cancellation(index, &cancel, || decode(&cancel));
        assert_eq!(
            result.unwrap_err().class(),
            C::Cancelled,
            "decode allocation {index}"
        );
    }
}

/// Independent reader failures retain their public size and companion classification.
#[test]
fn composition_reader_error_classes() {
    let ir = fixture();
    let bytes = forge(&ir);
    let cancel = CancellationToken::new();
    assert_eq!(
        decode_composition_project(
            &bytes,
            DecodeLimits::hard(),
            ProjectLimits {
                modules: 0,
                ..ir.limits
            },
            policy(),
            &cancel,
        )
        .unwrap_err()
        .class(),
        C::EncodedSizeLimit
    );
    let mut unknown = ir.clone();
    unknown.schema = "neutral.project-ir/unknown".into();
    assert_eq!(
        decode_composition_project(
            &forge(&unknown),
            DecodeLimits::hard(),
            unknown.limits,
            policy(),
            &cancel,
        )
        .unwrap_err()
        .class(),
        C::UnsupportedVersion
    );
    let mut corrupt = ir;
    corrupt.sources[0].source_id.clear();
    assert_eq!(
        decode_composition_project(
            &forge(&corrupt),
            DecodeLimits::hard(),
            corrupt.limits,
            policy(),
            &cancel,
        )
        .unwrap_err()
        .class(),
        C::InvalidProvenance
    );
}

/// Constructs an untrusted value at a valid byte offset for focused tuple checks.
fn node(value: CborValue) -> LocatedValue {
    LocatedValue { offset: 0, value }
}

/// Constructs an untrusted text token without normalizing its spelling.
fn string(value: &str) -> LocatedValue {
    node(CborValue::Text(value.into()))
}

/// Constructs a definite tuple with the exact listed members.
fn tuple_node(values: Vec<LocatedValue>) -> LocatedValue {
    node(CborValue::Array(values))
}

/// Parses writer output with the same canonical lexical boundary as public decoding.
fn parse(bytes: &[u8]) -> LocatedValue {
    parse_section(bytes, 0, DecodeLimits::hard(), &CancellationToken::new()).unwrap()
}

/// Scalar category tags preserve inert URLs/paths and booleans exactly on the wire.
#[test]
fn composition_scalar_wire_tags() {
    for (value, bytes) in [
        (
            V::<ClosedReference>::Bool(false),
            b"\x82\x64bool\xf4".as_slice(),
        ),
        (V::Url("u".into()), b"\x82\x63url\x61u".as_slice()),
        (V::Path("p".into()), b"\x82\x64path\x61p".as_slice()),
        (V::String("s".into()), b"\x82\x66string\x61s".as_slice()),
        (
            V::Number(ExactNumber::from_source("42", 64, 64).unwrap()),
            b"\x84\x63num\xf4\x6242\x00".as_slice(),
        ),
    ] {
        let mut writer = CborWriter::new();
        write_value(&mut writer, &value, 0).unwrap();
        assert_eq!(writer.finish().unwrap(), bytes);
        assert_eq!(
            read_value::<ClosedReference>(&parse(bytes), 0).unwrap(),
            value
        );
    }
}

/// Reference values retain the exact owner and cannot enter closed defaults.
#[test]
fn composition_references_preserve_identity_and_closedness() {
    let identity = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(neutral_core::profile::V1_SOURCE_PROFILE, "example"),
        "target",
    );
    let value = V::Reference(identity);
    let mut writer = CborWriter::new();
    write_value(&mut writer, &value, 0).unwrap();
    let bytes = writer.finish().unwrap();
    let parsed = parse(&bytes);
    assert_eq!(
        read_value::<ModuleSymbolIdentity>(&parsed, 0).unwrap(),
        value
    );
    assert_eq!(
        read_value::<ClosedReference>(&parsed, 0)
            .unwrap_err()
            .class(),
        C::InvalidEncodedSchema
    );
}

/// Builds a deep payload using one CBOR layer per semantic variant layer.
fn nested(depth: usize) -> V<ClosedReference> {
    let mut value = V::Null;
    for _ in 0..depth {
        value = V::Variant {
            tag: "next".into(),
            payload: Box::new(value),
        };
    }
    value
}

/// Materializes the corresponding untrusted tuple without bypassing depth inside the reader.
fn nested_node(depth: usize) -> LocatedValue {
    let mut value = tuple_node(vec![string("null")]);
    for _ in 0..depth {
        value = tuple_node(vec![string("variant"), string("next"), value]);
    }
    value
}

/// All recursive categories count depth once and accept the final permitted occurrence.
#[test]
fn composition_value_depth_bounds() {
    for depth in [PROJECT_MAX_DEPTH - 1, PROJECT_MAX_DEPTH] {
        let values = [
            V::List(vec![nested(depth)]),
            V::Record(vec![("field".into(), Some(nested(depth)))]),
            V::Variant {
                tag: "next".into(),
                payload: Box::new(nested(depth)),
            },
        ];
        let nodes = [
            tuple_node(vec![string("List"), tuple_node(vec![nested_node(depth)])]),
            tuple_node(vec![
                string("record"),
                tuple_node(vec![tuple_node(vec![string("field"), nested_node(depth)])]),
            ]),
            tuple_node(vec![string("variant"), string("next"), nested_node(depth)]),
        ];
        for (value, node) in values.into_iter().zip(nodes) {
            let mut writer = CborWriter::new();
            let written = write_value(&mut writer, &value, 0);
            let read = read_value::<ClosedReference>(&node, 0);
            if depth < PROJECT_MAX_DEPTH {
                written.unwrap();
                assert_eq!(read.unwrap(), value);
            } else {
                assert_eq!(written, Err(E::EncodedSizeLimit));
                assert_eq!(read.unwrap_err().class(), C::EncodedSizeLimit);
            }
        }
    }
}

/// Builds a record field around an independently supplied default-state tuple.
fn body_node(default: Vec<LocatedValue>) -> LocatedValue {
    tuple_node(vec![
        string("record"),
        tuple_node(vec![tuple_node(vec![
            string("field"),
            tuple_node(vec![string("bool")]),
            string("optional"),
            tuple_node(vec![
                tuple_node(vec![]),
                node(CborValue::Null),
                node(CborValue::Null),
                node(CborValue::Null),
                node(CborValue::Null),
            ]),
            tuple_node(default),
        ])]),
    ])
}

/// Default flags must agree with the presence or absence of their value slot.
#[test]
fn composition_default_flags_are_not_coerced() {
    for default in [
        vec![node(CborValue::Boolean(true))],
        vec![
            node(CborValue::Boolean(false)),
            tuple_node(vec![string("null")]),
        ],
    ] {
        assert_eq!(
            read_body(&body_node(default)).unwrap_err().class(),
            C::InvalidEncodedSchema
        );
    }
}

/// Complete record and variant bodies retain dormant defaults and every restriction slot.
#[test]
fn composition_bodies_preserve_contracts() {
    let number = |text| ExactNumber::from_source(text, 64, 64).unwrap();
    let record = B::Record(vec![
        CompositionField {
            name: "amount".into(),
            ty: T::Num,
            presence: Presence::Required,
            restrictions: FieldRestrictions {
                choices: Some(vec![V::Number(number("2"))]),
                minimum: Some(number("1")),
                maximum: Some(number("3")),
                ..FieldRestrictions::default()
            },
            default: None,
        },
        CompositionField {
            name: "label".into(),
            ty: T::String,
            presence: Presence::Optional,
            restrictions: FieldRestrictions {
                choices: Some(vec![V::String("a".into())]),
                min_length: Some(1),
                max_length: Some(3),
                ..FieldRestrictions::default()
            },
            default: None,
        },
        CompositionField {
            name: "ready".into(),
            ty: T::Bool,
            presence: Presence::Defaulted,
            restrictions: FieldRestrictions::default(),
            default: Some(V::Bool(true)),
        },
    ]);
    let variant = B::Variant(vec![
        CompositionAlternative {
            tag: "text".into(),
            ty: T::String,
        },
        CompositionAlternative {
            tag: "number".into(),
            ty: T::Num,
        },
    ]);
    for body in [record, variant] {
        let mut writer = CborWriter::new();
        write_body(&mut writer, &body).unwrap();
        assert_eq!(read_body(&parse(&writer.finish().unwrap())).unwrap(), body);
    }
}

/// Builds an origin tuple without depending on the origin writer's tag selection.
fn origin(path: Vec<LocatedValue>, kind: &str, attribution: LocatedValue) -> LocatedValue {
    let mut writer = CborWriter::new();
    write_symbol(
        &mut writer,
        &ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(neutral_core::profile::V1_SOURCE_PROFILE, "example"),
            "value",
        ),
    )
    .unwrap();
    tuple_node(vec![
        parse(&writer.finish().unwrap()),
        tuple_node(path),
        string(kind),
        attribution,
    ])
}

/// Closed origin paths and attribution categories reject unknown tokens of otherwise valid arity.
#[test]
fn composition_origin_tags_are_closed() {
    for path in [
        vec![tuple_node(vec![string("unknown")])],
        vec![tuple_node(vec![
            string("unknown"),
            node(CborValue::Unsigned(0)),
        ])],
    ] {
        assert_eq!(
            read_origin(&origin(path, "supplied", node(CborValue::Null)))
                .unwrap_err()
                .class(),
            C::InvalidEncodedSchema
        );
    }
    let location = SourceLocation::new(
        SourceContentDigest::from_bytes(b"source"),
        ByteSpan::new(0, 6).unwrap(),
    );
    let mut writer = CborWriter::new();
    write_location(&mut writer, location).unwrap();
    for attribution in [
        tuple_node(vec![string("unknown"), parse(&writer.finish().unwrap())]),
        tuple_node(vec![
            string("unknown"),
            string("Fixture"),
            string("1.0.0"),
            string("Record"),
            string("field"),
            node(CborValue::Null),
        ]),
    ] {
        assert_eq!(
            read_origin(&origin(vec![], "supplied", attribution))
                .unwrap_err()
                .class(),
            C::InvalidEncodedSchema
        );
    }
    assert_eq!(
        read_origin(&origin(vec![], "explicit-null", node(CborValue::Null)))
            .unwrap()
            .kind,
        K::ExplicitNull
    );
}

/// Valid origin paths and evidence preserve every occurrence kind independently of values.
#[test]
fn composition_origins_preserve_paths_and_evidence() {
    let binding = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(neutral_core::profile::V1_SOURCE_PROFILE, "example"),
        "value",
    );
    for (kind, attribution) in [
        (
            K::Supplied,
            Some(A::Source(SourceLocation::new(
                SourceContentDigest::from_bytes(b"source"),
                ByteSpan::new(0, 6).unwrap(),
            ))),
        ),
        (K::ExplicitNull, None),
        (K::OmittedOptional, None),
        (
            K::Defaulted,
            Some(A::Vocabulary {
                identity: "Fixture".into(),
                version: "1.0.0".into(),
                type_name: "Record".into(),
                field_name: "field".into(),
                span: Some(ByteSpan::new(1, 4).unwrap()),
            }),
        ),
    ] {
        let origin = CompositionOrigin {
            binding: binding.clone(),
            path: vec![P::Field("items".into()), P::Element(2), P::Payload],
            kind,
            attribution,
        };
        let mut writer = CborWriter::new();
        write_origin(&mut writer, &origin).unwrap();
        assert_eq!(
            read_origin(&parse(&writer.finish().unwrap())).unwrap(),
            origin
        );
    }
}

/// Declared definitions must group by exact owner and maintain canonical unique name order.
#[test]
fn composition_catalogue_definitions_are_exact_and_ordered() {
    let source = ProjectVocabularySource {
        identity: "Fixture".into(),
        version: "1.0.0".into(),
        digest: VocabularyContentDigest::from_bytes(b"bundle"),
        byte_len: 6,
    };
    let mut p = catalogues(vec![tuple_node(vec![
        string("Fixture"),
        string("1.0.0"),
        tuple_node(vec![]),
    ])]);
    if let CborValue::Array(catalogues) = &mut p[4].value
        && let CborValue::Array(catalogue) = &mut catalogues[0].value
    {
        catalogue[4] = tuple_node(vec![string("First")]);
    }
    let definition = |name, public| {
        tuple_node(vec![
            string("Fixture"),
            string("1.0.0"),
            string(name),
            node(CborValue::Boolean(public)),
            tuple_node(vec![string("record"), tuple_node(vec![])]),
        ])
    };
    p[3] = tuple_node(vec![definition("First", true), definition("Second", false)]);
    let bundles = read_vocabularies(&p, std::slice::from_ref(&source)).unwrap();
    assert_eq!(bundles[0].definitions.len(), 2);
    assert_eq!(bundles[0].definitions[0].name, "First");
    assert!(bundles[0].definitions[0].public);
    assert_eq!(bundles[0].definitions[1].name, "Second");
    assert!(!bundles[0].definitions[1].public);
    for names in [["First", "First"], ["Second", "First"]] {
        p[3] = tuple_node(vec![
            definition(names[0], true),
            definition(names[1], false),
        ]);
        assert_eq!(
            read_vocabularies(&p, std::slice::from_ref(&source))
                .unwrap_err()
                .class(),
            C::InvalidEncodedSchema
        );
    }
}

/// Supplies one catalogue, dependency owner and source for independent ownership joins.
fn catalogues(dependencies: Vec<LocatedValue>) -> [LocatedValue; 16] {
    let mut p = std::array::from_fn(|_| node(CborValue::Null));
    p[3] = tuple_node(vec![]);
    p[4] = tuple_node(vec![tuple_node(vec![
        string("Fixture"),
        string("1.0.0"),
        string(neutral_vocabulary::composition::SCHEMA_VERSION),
        tuple_node(vec![string(
            neutral_vocabulary::composition::REQUIRED_FEATURE,
        )]),
        tuple_node(vec![]),
    ])]);
    p[12] = tuple_node(dependencies);
    p
}

/// Exact cardinality and every owner component must match before zipping catalogue companions.
#[test]
fn composition_catalogue_companions_match_exactly() {
    let source = ProjectVocabularySource {
        identity: "Fixture".into(),
        version: "1.0.0".into(),
        digest: VocabularyContentDigest::from_bytes(b"bundle"),
        byte_len: 6,
    };
    let dep = || tuple_node(vec![string("Fixture"), string("1.0.0"), tuple_node(vec![])]);
    let p = catalogues(vec![dep()]);
    assert_eq!(
        read_vocabularies(&p, std::slice::from_ref(&source))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        read_vocabularies(&catalogues(vec![]), std::slice::from_ref(&source))
            .unwrap_err()
            .class(),
        C::InvalidEncodedSchema
    );
    assert_eq!(
        read_vocabularies(&p, &[]).unwrap_err().class(),
        C::InvalidEncodedSchema
    );
    for (identity, version) in [("Other", "1.0.0"), ("Fixture", "2.0.0")] {
        let p = catalogues(vec![tuple_node(vec![
            string(identity),
            string(version),
            tuple_node(vec![]),
        ])]);
        assert_eq!(
            read_vocabularies(&p, std::slice::from_ref(&source))
                .unwrap_err()
                .class(),
            C::InvalidEncodedSchema
        );
        let mut other = source.clone();
        other.identity = identity.into();
        other.version = version.into();
        assert_eq!(
            read_vocabularies(&catalogues(vec![dep()]), &[other])
                .unwrap_err()
                .class(),
            C::InvalidEncodedSchema
        );
    }
}
