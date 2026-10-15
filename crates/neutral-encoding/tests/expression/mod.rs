// SPDX-License-Identifier: Apache-2.0

//! Literal successor wire checks and atomic hostile-input recovery.

use super::*;
use neutral_core::allocation::testing;
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity, VocabularyIdentity,
    composition::{
        CompositionBody, CompositionBundle, CompositionDefinition, CompositionDependency,
        CompositionValue,
    },
    expression::{
        BinaryOperator, Expression, ExpressionBody, ExpressionNode, MatchArm, SelectorSegment,
        UnaryOperator,
        project::ExpressionProjectIr,
        templates::{ParameterPhase, PublicTemplate, TemplateParameter},
    },
    project::{ProjectModule, ProjectVocabularySource},
    project_interface::ProjectPublicType as T,
};

/// Decodes independently registered hex literals, never deriving expected bytes from the codec.
fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

/// Selects the three complete artifacts, excluding explicitly registered node fragments.
fn vectors() -> Vec<Vec<u8>> {
    [
        include_str!("fixtures/static.hex"),
        include_str!("fixtures/residual.hex"),
        include_str!("fixtures/template.hex"),
    ]
    .into_iter()
    .map(|value| hex(value.trim()))
    .collect()
}

/// Uses the fixture's independent positive policy with historical scalar ceilings.
fn limits(ir: &ExpressionProjectIr) -> SemanticLimits {
    SemanticLimits {
        project: ir.limits,
        composition: neutral_vocabulary::composition::CompositionLimits::from_policy(
            neutral_vocabulary::VocabularyLimits::from_structural(
                neutral_core::StructuralLimits::new(1_048_576, 64).unwrap(),
            ),
            ir.composition_limits,
        ),
        expression: ir.expression_limits,
        expansion: ir.expansion_limits,
    }
}

/// Reads raw data for controlled forgeries without obtaining validation authority.
fn raw(bytes: &[u8]) -> ExpressionProjectIr {
    let value = crate::decoder::parse_canonical_section(
        &bytes[8..],
        8,
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap();
    project::read_ir(&value, &CancellationToken::new()).unwrap()
}

/// Writes deliberately unvalidated tuples to exercise the external trust boundary.
fn forge(ir: &ExpressionProjectIr) -> Vec<u8> {
    let mut writer = CborWriter::new();
    project::write_ir(&mut writer, ir).unwrap();
    let mut bytes = b"NEUP3\r\n\x1a".to_vec();
    bytes.extend(writer.finish().unwrap());
    bytes
}

/// Catches missing sections, incorrect body wrapping and discarded templates/companions.
#[test]
fn frozen_complete_expression_artifacts_decode_and_reencode_exactly() {
    let cases = vectors();
    assert_eq!(cases.len(), 3);
    for bytes in cases {
        assert_eq!(&bytes[..9], b"NEUP3\r\n\x1a\x96");
        let ir = raw(&bytes);
        let cancel = CancellationToken::new();
        let project =
            decode_expression_project(&bytes, DecodeLimits::hard(), limits(&ir), &cancel).unwrap();
        assert_eq!(
            decode_complete(&bytes, DecodeLimits::hard(), &limits(&ir), &cancel)
                .unwrap()
                .ir(),
            &ir
        );
        assert_eq!(project.ir(), &ir);
        assert_eq!(encode_expression_project(&project, &cancel).unwrap(), bytes);
        assert_eq!(forge(&ir), bytes);
    }
}

#[test]
fn expression_and_expansion_attribution_lists_roundtrip() {
    use neutral_core::{ByteSpan, SourceContentDigest, SourceLocation};
    use neutral_ir::expression::attribution::{ExpansionAttribution, ExpressionAttribution};

    let bytes = vectors().remove(0);
    let mut ir = raw(&bytes);
    let location = SourceLocation::new(
        SourceContentDigest::from_bytes(b"source"),
        ByteSpan::new(0, 1).unwrap(),
    );
    ir.expression_attributions.push(ExpressionAttribution {
        owner: symbol(),
        node_path: vec![0, 2],
        occurrence: Some(location),
        related_public_locations: vec![location],
    });
    ir.expansion_attributions.push(ExpansionAttribution {
        owner: symbol(),
        application: location,
        definition: location,
        arguments: vec![location],
        original_axis_indices: vec![1, 3],
    });

    let mut writer = CborWriter::new();
    companions::write_attributions(&mut writer, &ir).unwrap();
    let encoded = writer.finish().unwrap();
    let parsed = crate::decoder::parse_canonical_section(
        &encoded,
        0,
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap();
    let sections = crate::project::array(&parsed).unwrap();
    assert_eq!(
        companions::read_expressions(&sections[0], &CancellationToken::new()).unwrap(),
        ir.expression_attributions
    );
    assert_eq!(
        companions::read_expansions(&sections[1], &CancellationToken::new()).unwrap(),
        ir.expansion_attributions
    );
}

/// Catches heuristic profile fallback, nonminimal widths and partial artifact acceptance.
#[test]
fn distinct_magic_exact_arity_and_canonical_framing_reject_without_fallback() {
    let bytes = vectors().remove(0);
    let ir = raw(&bytes);
    let cancel = CancellationToken::new();
    let decode =
        |bytes: &[u8]| decode_expression_project(bytes, DecodeLimits::hard(), limits(&ir), &cancel);
    for digit in *b"124" {
        let mut wrong = bytes.clone();
        wrong[4] = digit;
        assert_eq!(decode(&wrong).unwrap_err().class(), C::MalformedFrame);
    }
    assert_eq!(
        crate::composition::decode_composition_project(
            &bytes,
            DecodeLimits::hard(),
            ir.limits,
            limits(&ir).composition,
            &cancel
        )
        .unwrap_err()
        .class(),
        C::MalformedFrame
    );
    for arity in [0x95, 0x97] {
        let mut wrong = bytes.clone();
        wrong[8] = arity;
        assert!(decode(&wrong).is_err());
    }
    let mut nonminimal = bytes.clone();
    nonminimal.splice(8..9, [0x98, 22]);
    assert_eq!(decode(&nonminimal).unwrap_err().class(), C::MalformedCbor);
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(decode(&trailing).unwrap_err().class(), C::MalformedCbor);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err(), "truncated at {end}");
    }
    assert!(decode(&bytes).is_ok());
}

/// Supplies one stable logical owner for raw grammar-only node checks.
fn symbol() -> neutral_ir::ModuleSymbolIdentity {
    neutral_ir::ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "m"), "x")
}
/// Supplies the same literal Boolean child in each raw grammar tuple.
fn leaf() -> Expression {
    Expression {
        ty: T::Bool,
        node: ExpressionNode::Value(CompositionValue::Bool(true)),
    }
}

/// Catches wrong arity/operator spelling and lost children for every frozen node kind.
#[test]
fn every_frozen_node_and_operator_has_exact_tuple_and_roundtrip() {
    use ExpressionNode as N;
    let cases = [
        (3, "value", N::Value(CompositionValue::Bool(true))),
        (3, "input", N::Input(symbol())),
        (
            5,
            "output",
            N::Output {
                target: symbol(),
                result_type: T::Bool,
                selector: vec![
                    SelectorSegment::Field("f".into()),
                    SelectorSegment::Index(2),
                    SelectorSegment::Alternative("a".into()),
                ],
            },
        ),
        (
            4,
            "unary",
            N::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(leaf()),
            },
        ),
        (
            5,
            "binary",
            N::Binary {
                operator: BinaryOperator::And,
                left: Box::new(leaf()),
                right: Box::new(leaf()),
            },
        ),
        (
            5,
            "if",
            N::If {
                guard: Box::new(leaf()),
                then_branch: Box::new(leaf()),
                else_branch: Box::new(leaf()),
            },
        ),
        (
            4,
            "match",
            N::Match {
                scrutinee: Box::new(leaf()),
                arms: vec![MatchArm {
                    tag: "a".into(),
                    payload_type: T::Bool,
                    body: leaf(),
                }],
            },
        ),
        (3, "text", N::Text(Box::new(leaf()))),
        (3, "interpolate", N::Interpolate(vec![leaf()])),
        (3, "list", N::List(vec![leaf()])),
        (
            3,
            "record",
            N::Record(vec![("a".into(), Some(leaf())), ("b".into(), None)]),
        ),
        (
            4,
            "variant",
            N::Variant {
                tag: "a".into(),
                payload: Box::new(leaf()),
            },
        ),
        (4, "bound", N::Bound { depth: 1, slot: 2 }),
    ];
    for (arity, tag, node) in cases {
        let expression = Expression { ty: T::Bool, node };
        let mut writer = CborWriter::new();
        trees::write_tree(&mut writer, &expression, 1).unwrap();
        let bytes = writer.finish().unwrap();
        assert_eq!(bytes[0], 0x80 + arity, "{tag}");
        let parsed = crate::decoder::parse_canonical_section(
            &bytes,
            0,
            DecodeLimits::hard(),
            &CancellationToken::new(),
        )
        .unwrap();
        let tuple = crate::project::array(&parsed).unwrap();
        assert_eq!(crate::project::text(&tuple[0]).unwrap(), tag);
        assert_eq!(
            trees::read_tree(&parsed, 1, &CancellationToken::new()).unwrap(),
            expression
        );
    }
}

/// Catches accepting host opcodes or changing a frozen operator token.
#[test]
fn frozen_operator_tokens_are_closed() {
    for spelling in [
        "*", "/", "+", "-", "++", "<", "<=", ">", ">=", "==", "!=", "&&", "||",
    ] {
        assert_eq!(
            trees::binary_operator(spelling).unwrap().spelling(),
            spelling
        );
    }
    assert!(trees::binary_operator("eval").is_none());
    assert!(trees::unary_operator("eval").is_none());
    assert_eq!(trees::unary_operator("-").unwrap(), UnaryOperator::Negate);
}

#[test]
fn tree_depth_and_selector_indexes_are_bounded() {
    let tree = leaf();
    let mut writer = CborWriter::new();
    assert_eq!(
        trees::write_tree(
            &mut writer,
            &tree,
            neutral_ir::expression::profile::MAX_DEPTH + 1
        )
        .unwrap_err(),
        E::EncodedSizeLimit
    );

    let mut writer = CborWriter::new();
    writer.array(5).unwrap();
    writer.text("output").unwrap();
    crate::project::write_type(&mut writer, &T::Bool, 0).unwrap();
    crate::project::write_symbol(&mut writer, &symbol()).unwrap();
    crate::project::write_type(&mut writer, &T::Bool, 0).unwrap();
    writer.array(1).unwrap();
    writer.array(2).unwrap();
    writer.text("index").unwrap();
    writer.unsigned(u64::from(u32::MAX) + 1).unwrap();
    let encoded = writer.finish().unwrap();
    let parsed = crate::decoder::parse_canonical_section(
        &encoded,
        0,
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(
        trees::read_tree(&parsed, 1, &CancellationToken::new())
            .unwrap_err()
            .class(),
        C::InvalidEncodedSchema
    );
    assert_eq!(
        trees::read_tree(
            &parsed,
            neutral_ir::expression::profile::MAX_DEPTH + 1,
            &CancellationToken::new()
        )
        .unwrap_err()
        .class(),
        C::EncodedSizeLimit
    );
}

/// Catches trusting producer annotations, stale summaries or forged original-source evidence.
#[test]
fn forged_annotations_binders_facts_and_companions_never_publish_authority() {
    let cases = vectors();
    let cancel = CancellationToken::new();
    for mutate in 0..6 {
        let mut ir = raw(&cases[1]);
        match mutate {
            0 => ir.interface_digest = neutral_core::SemanticDigest::from_raw_bytes([0; 32]),
            1 => ir.expression_resources.residual_nodes += 1,
            2 => ir.expression_attributions[0].node_path = vec![99],
            3 => ir.sources[0].source_id.clear(),
            4 => {
                let Some(ExpressionBody::Expression(tree)) = &mut ir.declarations[2].body else {
                    panic!()
                };
                let ExpressionNode::If { else_branch, .. } = &mut tree.node else {
                    panic!()
                };
                else_branch.ty = T::String;
            }
            _ => {
                let Some(ExpressionBody::Expression(tree)) = &mut ir.declarations[2].body else {
                    panic!()
                };
                tree.node = ExpressionNode::Bound { depth: 0, slot: 0 };
            }
        }
        assert!(
            decode_expression_project(&forge(&ir), DecodeLimits::hard(), limits(&ir), &cancel)
                .is_err()
        );
        let valid = raw(&cases[1]);
        assert!(
            decode_expression_project(&cases[1], DecodeLimits::hard(), limits(&valid), &cancel)
                .is_ok()
        );
    }
}

/// Changes one exact frozen tuple token while preserving minimal CBOR and its byte length.
fn replace_token(bytes: &mut [u8], tuple_prefix: &[u8]) {
    let position = bytes
        .windows(tuple_prefix.len())
        .position(|part| part == tuple_prefix)
        .unwrap();
    bytes[position + tuple_prefix.len() - 1] = b'@';
}

/// Catches treating unknown body/node/operator/phase/profile tokens as permissive defaults.
#[test]
fn unknown_expression_tokens_and_invalid_body_sums_fail_closed() {
    let cases = vectors();
    let cancel = CancellationToken::new();
    for (case, token) in [
        (0, b"\x82\x65value".as_slice()),
        (1, b"\x85\x62if".as_slice()),
        (1, b"\x61+".as_slice()),
        (2, b"\x84\x65bound".as_slice()),
        (2, b"\x66static".as_slice()),
    ] {
        let ir = raw(&cases[case]);
        let mut bytes = cases[case].clone();
        replace_token(&mut bytes, token);
        assert_eq!(
            decode_expression_project(&bytes, DecodeLimits::hard(), limits(&ir), &cancel)
                .unwrap_err()
                .class(),
            C::InvalidEncodedSchema
        );
    }
    let mut ir = raw(&cases[0]);
    ir.semantic_profiles.required_features.swap(0, 1);
    assert_eq!(
        decode_expression_project(&forge(&ir), DecodeLimits::hard(), limits(&ir), &cancel)
            .unwrap_err()
            .class(),
        C::UnsupportedVersion
    );
    let mut ir = raw(&cases[0]);
    ir.declarations[0].body = None;
    assert!(
        decode_expression_project(&forge(&ir), DecodeLimits::hard(), limits(&ir), &cancel).is_err()
    );
}

/// Catches relaxing any independent host artifact or section limit.
#[test]
fn exact_wire_limits_and_one_over_fail_independently() {
    let bytes = vectors().remove(0);
    let ir = raw(&bytes);
    let cancel = CancellationToken::new();
    for wire in [
        DecodeLimits::hard().with_artifact_bytes(bytes.len()),
        DecodeLimits::hard().with_section_bytes(bytes.len() - 8),
        DecodeLimits::hard().with_nesting_depth(6),
        DecodeLimits::hard().with_container_items(22),
        DecodeLimits::hard().with_text_bytes(25),
        DecodeLimits::hard().with_byte_string_bytes(32),
        DecodeLimits::hard().with_traversal_nodes(154),
    ] {
        assert!(decode_expression_project(&bytes, wire, limits(&ir), &cancel).is_ok());
    }
    for wire in [
        DecodeLimits::hard().with_artifact_bytes(bytes.len() - 1),
        DecodeLimits::hard().with_section_bytes(bytes.len() - 9),
        DecodeLimits::hard().with_nesting_depth(5),
        DecodeLimits::hard().with_container_items(21),
        DecodeLimits::hard().with_text_bytes(24),
        DecodeLimits::hard().with_byte_string_bytes(31),
        DecodeLimits::hard().with_traversal_nodes(153),
    ] {
        assert_eq!(
            decode_expression_project(&bytes, wire, limits(&ir), &cancel)
                .unwrap_err()
                .class(),
            C::EncodedSizeLimit
        );
    }
    let mut policy = limits(&ir);
    policy.project.artifact_bytes = bytes.len() as u64 - 1;
    assert_eq!(
        decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel)
            .unwrap_err()
            .class(),
        C::EncodedSizeLimit
    );
}

/// Catches silently defaulting zero producer or caller controls in any policy partition.
#[test]
fn zero_controls_fail_before_complete_publication() {
    let bytes = vectors().remove(0);
    let original = raw(&bytes);
    let policy = limits(&original);
    let cancel = CancellationToken::new();
    for partition in 0..4 {
        let mut ir = raw(&bytes);
        let mut caller = policy;
        match partition {
            0 => {
                ir.limits.nodes = 0;
                caller.project.nodes = 0;
            }
            1 => {
                ir.composition_limits.work = 0;
                caller.composition.work = 0;
            }
            2 => {
                ir.expression_limits.nodes = 0;
                caller.expression.nodes = 0;
            }
            _ => {
                ir.expansion_limits.definitions = 0;
                caller.expansion.definitions = 0;
            }
        }
        assert_eq!(
            decode_expression_project(&forge(&ir), DecodeLimits::hard(), policy, &cancel)
                .unwrap_err()
                .class(),
            C::EncodedSizeLimit
        );
        assert_eq!(
            decode_expression_project(&bytes, DecodeLimits::hard(), caller, &cancel)
                .unwrap_err()
                .class(),
            C::EncodedSizeLimit
        );
    }
    assert!(decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel).is_ok());
}

/// Catches allocation aborts, missed cancellation checkpoints and poisoned subsequent decodes.
#[test]
fn all_observed_decode_reservations_fail_atomically_and_recover() {
    for bytes in vectors() {
        let ir = raw(&bytes);
        let policy = limits(&ir);
        let decode = |cancel: &CancellationToken| {
            decode_expression_project(&bytes, DecodeLimits::hard(), policy, cancel)
        };
        let (result, count) = testing::observe(None, || decode(&CancellationToken::new()));
        assert!(result.is_ok());
        for index in 0..count {
            let (result, _) = testing::observe(Some(index), || decode(&CancellationToken::new()));
            assert_eq!(
                result.unwrap_err().class(),
                C::EncodedSizeLimit,
                "allocation {index}"
            );
            let cancel = CancellationToken::new();
            let (result, _) = testing::observe_cancellation(index, &cancel, || decode(&cancel));
            assert_eq!(
                result.unwrap_err().class(),
                C::Cancelled,
                "cancellation {index}"
            );
            assert!(decode(&CancellationToken::new()).is_ok());
        }
    }
}

/// Catches writer allocation aborts and cancellation before the completed artifact is returned.
#[test]
fn all_observed_encode_reservations_fail_atomically_and_recover() {
    for bytes in vectors() {
        let ir = raw(&bytes);
        let project = decode_expression_project(
            &bytes,
            DecodeLimits::hard(),
            limits(&ir),
            &CancellationToken::new(),
        )
        .unwrap();
        let encode = |cancel: &CancellationToken| encode_expression_project(&project, cancel);
        let (result, count) = testing::observe(None, || encode(&CancellationToken::new()));
        assert_eq!(result.unwrap(), bytes);
        for index in 0..count {
            let (result, _) = testing::observe(Some(index), || encode(&CancellationToken::new()));
            assert_eq!(result, Err(E::EncodedSizeLimit), "allocation {index}");
            let cancel = CancellationToken::new();
            let (result, _) = testing::observe_cancellation(index, &cancel, || encode(&cancel));
            assert_eq!(result, Err(E::Cancelled), "cancellation {index}");
            assert_eq!(encode(&CancellationToken::new()).unwrap(), bytes);
        }
    }
}

#[test]
fn expression_module_imports_preserve_order_in_the_wire_tuple_and_raw_ir() {
    let cases = vectors();
    let mut ir = raw(&cases[0]);
    ir.modules = vec![ProjectModule {
        identity: LogicalModuleIdentity::new("1.0", "main"),
        imports: vec!["alpha".into(), "beta".into()],
    }];

    let parsed = parse_written_ir(&ir);
    let tuple = crate::project::array(&parsed).unwrap();
    let modules = crate::project::array(&tuple[1]).unwrap();
    let module = crate::project::tuple::<2>(&modules[0]).unwrap();
    let imports = crate::project::array(&module[1]).unwrap();
    assert_eq!(
        imports
            .iter()
            .map(|value| crate::project::text(value).unwrap())
            .collect::<Vec<_>>(),
        ["alpha", "beta"]
    );

    let roundtrip = project::read_ir(&parsed, &CancellationToken::new()).unwrap();
    assert_eq!(roundtrip.modules, ir.modules);
}

#[test]
fn expression_vocabulary_catalogues_preserve_features_visibility_and_dependencies() {
    let cases = vectors();
    let mut ir = raw(&cases[0]);
    let digest = neutral_core::VocabularyContentDigest::from_raw_bytes([7; 32]);
    ir.vocabularies = vec![CompositionBundle {
        identity: VocabularyIdentity::new(
            "Example",
            "1.0.0",
            neutral_vocabulary::VOCABULARY_SCHEMA_VERSION,
            neutral_vocabulary::composition::ENCODING_VERSION,
            digest,
            vec!["feature-alpha".into(), "feature-zeta".into()],
        ),
        dependencies: vec![CompositionDependency {
            identity: "Base".into(),
            version: "2.1.0".into(),
        }],
        definitions: vec![
            CompositionDefinition {
                name: "PrivateThing".into(),
                public: false,
                body: CompositionBody::Record(Vec::new()),
            },
            CompositionDefinition {
                name: "PublicThing".into(),
                public: true,
                body: CompositionBody::Record(Vec::new()),
            },
        ],
    }];
    ir.vocabulary_sources = vec![ProjectVocabularySource {
        identity: "Example".into(),
        version: "1.0.0".into(),
        digest,
        byte_len: 123,
    }];

    let parsed = parse_written_ir(&ir);
    let tuple = crate::project::array(&parsed).unwrap();
    let definitions = crate::project::array(&tuple[3]).unwrap();
    assert_eq!(definitions.len(), 2);
    for ((definition, name), public) in definitions
        .iter()
        .zip(["PrivateThing", "PublicThing"])
        .zip([false, true])
    {
        let definition = crate::project::tuple::<5>(definition).unwrap();
        assert_eq!(crate::project::text(&definition[2]).unwrap(), name);
        assert_eq!(crate::project::boolean(&definition[3]).unwrap(), public);
        let body = crate::project::tuple::<2>(&definition[4]).unwrap();
        assert_eq!(crate::project::text(&body[0]).unwrap(), "record");
        assert!(crate::project::array(&body[1]).unwrap().is_empty());
    }

    let catalogues = crate::project::array(&tuple[4]).unwrap();
    let catalogue = crate::project::tuple::<5>(&catalogues[0]).unwrap();
    assert_eq!(
        crate::project::array(&catalogue[3])
            .unwrap()
            .iter()
            .map(|feature| crate::project::text(feature).unwrap())
            .collect::<Vec<_>>(),
        ["feature-alpha", "feature-zeta"]
    );
    assert_eq!(
        crate::project::array(&catalogue[4])
            .unwrap()
            .iter()
            .map(|name| crate::project::text(name).unwrap())
            .collect::<Vec<_>>(),
        ["PublicThing"]
    );

    let dependency_groups = crate::project::array(&tuple[12]).unwrap();
    let group = crate::project::tuple::<3>(&dependency_groups[0]).unwrap();
    let dependencies = crate::project::array(&group[2]).unwrap();
    let dependency = crate::project::tuple::<2>(&dependencies[0]).unwrap();
    assert_eq!(crate::project::text(&dependency[0]).unwrap(), "Base");
    assert_eq!(crate::project::text(&dependency[1]).unwrap(), "2.1.0");

    let roundtrip = project::read_ir(&parsed, &CancellationToken::new()).unwrap();
    assert_eq!(roundtrip.vocabularies, ir.vocabularies);
    assert_eq!(roundtrip.vocabulary_sources, ir.vocabulary_sources);
}

#[test]
fn public_template_parameter_defaults_preserve_trees_phases_types_and_order() {
    let cases = vectors();
    let mut ir = raw(&cases[2]);
    ir.public_templates = vec![PublicTemplate {
        identity: ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "main"), "choose"),
        parameters: vec![
            TemplateParameter {
                name: "mode".into(),
                phase: ParameterPhase::Static,
                ty: T::Bool,
                default: Some(bool_tree(true)),
            },
            TemplateParameter {
                name: "label".into(),
                phase: ParameterPhase::Deferred,
                ty: T::String,
                default: None,
            },
        ],
        result_type: T::Bool,
        body: bool_tree(true),
    }];

    let roundtrip = project::read_ir(&parse_written_ir(&ir), &CancellationToken::new()).unwrap();
    let template = &roundtrip.public_templates[0];
    assert_eq!(template.parameters.len(), 2);
    assert_eq!(template.parameters[0].name, "mode");
    assert_eq!(template.parameters[0].phase, ParameterPhase::Static);
    assert_eq!(template.parameters[0].ty, T::Bool);
    assert_eq!(template.parameters[0].default, Some(bool_tree(true)));
    assert_eq!(template.parameters[1].name, "label");
    assert_eq!(template.parameters[1].phase, ParameterPhase::Deferred);
    assert_eq!(template.parameters[1].ty, T::String);
    assert_eq!(template.parameters[1].default, None);
    assert_eq!(template.result_type, T::Bool);
    assert_eq!(template.body, bool_tree(true));
}

/// Canonically parses the exact raw expression IR produced by the wire writer.
fn parse_written_ir(ir: &ExpressionProjectIr) -> crate::decoder::LocatedValue {
    let bytes = forge(ir);
    crate::decoder::parse_canonical_section(
        &bytes[8..],
        8,
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap()
}

/// Constructs the same typed literal expression for tree and default assertions.
fn bool_tree(value: bool) -> Expression {
    Expression {
        ty: T::Bool,
        node: ExpressionNode::Value(CompositionValue::Bool(value)),
    }
}
