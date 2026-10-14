// SPDX-License-Identifier: Apache-2.0

//! Complete frozen /3 identity vectors and independent partition/boundary regressions.

use neutral_core::{
    ByteSpan, CancellationToken, SemanticDigest, SourceContentDigest, SourceLocation,
};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        BindingValue as V, CompositionBody, CompositionField, FieldPresence, FieldRestrictions,
        ValueOriginKind,
        project::{
            CompositionAttribution, CompositionOrigin, CompositionPolicy, CompositionResourceFacts,
            CompositionSignature as S,
        },
    },
    expression::{
        BinaryOperator, Expression, ExpressionBody, ExpressionNode as N, MatchArm, SelectorSegment,
        UnaryOperator,
        attribution::{ExpansionAttribution, ExpressionAttribution},
        profile::{self, ExpressionLimits, SemanticProfiles},
        project::{ExpressionDeclaration, ExpressionProjectIr, ExpressionResourceFacts},
        templates::{ExpansionLimits, ParameterPhase, PublicTemplate, TemplateParameter},
    },
    project::{
        ProjectLimits, ProjectModule, ProjectResourceFacts, ProjectSource, ProjectSourceMap,
    },
    project_identity::{
        ArtifactIdentityInput, ArtifactKind, CapturedIdentityInput, CapturedIdentitySource,
        ExpressionDerivationContext, IdentityError, IdentityLimits, IdentityTranscript,
        canonical_expression_project, captured_expression_closure, expression_artifact_identity,
        expression_derivation_identity, expression_interface,
    },
    project_interface::ProjectPublicType as T,
};

/// Complete residual/template transcript and partition-variation oracles.
const VECTORS: &str = include_str!("fixtures/expression/complete-identity-vectors.json");
/// Complete minimal-static transcript oracles.
const STATIC_VECTORS: &str = include_str!("fixtures/expression/identity-vectors.json");
/// Exact captured source bytes of the frozen residual project.
const RESIDUAL_SOURCE: &[u8] = include_bytes!("fixtures/expression/deferred.neu");
/// Exact captured source bytes of the frozen public-template project.
const TEMPLATE_SOURCE: &[u8] = include_bytes!("fixtures/expression/public-template.neu");
/// Independent controls large enough for every frozen complete transcript.
const LIMITS: IdentityLimits = IdentityLimits {
    bytes: 1_048_576,
    nodes: 100_000,
};
/// Explicit frozen producer controls, never discovered from the test environment.
const CAPTURE_LIMITS: [u64; 14] = [
    1_048_576, 1_048_576, 1000, 1024, 1024, 1000, 1_048_576, 1_048_576, 1000, 1000, 1000, 1000,
    1000, 1_048_576,
];

/// Constructs the literal source symbol used by the frozen fixtures.
fn symbol(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new("1.0", "example::expressions"),
        name,
    )
}

/// Parses bounded exact decimal data without host floating-point expectations.
fn number(value: &str) -> ExactNumber {
    ExactNumber::from_source(value, 4096, 4096).unwrap()
}

/// Builds a typed literal leaf for independent identity framing checks.
fn numeric(value: &str) -> Expression {
    Expression {
        ty: T::Num,
        node: N::Value(V::Number(number(value))),
    }
}

/// Retains an exact input leaf without supplying any runtime value.
fn input(name: &str, ty: T) -> Expression {
    Expression {
        ty,
        node: N::Input(symbol(name)),
    }
}

/// Builds one named static parameter with explicit absent default.
fn parameter(name: &str) -> TemplateParameter {
    TemplateParameter {
        name: name.into(),
        phase: ParameterPhase::Static,
        ty: T::Num,
        default: None,
    }
}

/// Retains a complete public binding signature and tagged /3 body.
fn declaration(name: &str, ty: T, body: ExpressionBody) -> ExpressionDeclaration {
    ExpressionDeclaration {
        identity: symbol(name),
        public: true,
        signature: S::Binding(ty),
        body: Some(body),
    }
}

/// Uses only checked original-byte spans, including template and argument occurrences.
fn location(digest: SourceContentDigest, start: u64, end: u64) -> SourceLocation {
    SourceLocation::new(digest, ByteSpan::new(start, end).unwrap())
}

/// The only complete frozen projects are constructed literally, including every companion role.
#[allow(clippy::too_many_lines)] // Keep the independent complete fixture literal visible together.
fn project(template: bool) -> ExpressionProjectIr {
    let source = if template {
        TEMPLATE_SOURCE
    } else {
        RESIDUAL_SOURCE
    };
    let digest = SourceContentDigest::from_bytes(source);
    let declarations = if template {
        vec![declaration(
            "result",
            T::Num,
            ExpressionBody::Value(V::Number(number("1"))),
        )]
    } else {
        vec![
            declaration("count", T::Num, ExpressionBody::Input),
            declaration("enabled", T::Bool, ExpressionBody::Input),
            declaration(
                "result",
                T::Num,
                ExpressionBody::Expression(Expression {
                    ty: T::Num,
                    node: N::If {
                        guard: Box::new(input("enabled", T::Bool)),
                        then_branch: Box::new(Expression {
                            ty: T::Num,
                            node: N::Binary {
                                operator: BinaryOperator::Add,
                                left: Box::new(input("count", T::Num)),
                                right: Box::new(numeric("1")),
                            },
                        }),
                        else_branch: Box::new(numeric("0")),
                    },
                }),
            ),
        ]
    };
    let source_maps = if template {
        vec![("Identity", 39, 102), ("result", 104, 154)]
    } else {
        vec![("count", 65, 87), ("enabled", 39, 64), ("result", 89, 144)]
    }
    .into_iter()
    .map(|(name, start, end)| ProjectSourceMap {
        declaration: symbol(name),
        location: location(digest, start, end),
    })
    .collect();
    let expression_attributions = if template {
        vec![(vec![], 95, 100)]
    } else {
        vec![
            (vec![], 109, 144),
            (vec![0], 112, 119),
            (vec![1], 122, 131),
            (vec![1, 0], 122, 127),
            (vec![1, 1], 130, 131),
            (vec![2], 141, 142),
        ]
    }
    .into_iter()
    .map(|(node_path, start, end)| ExpressionAttribution {
        owner: symbol(if template { "Identity" } else { "result" }),
        node_path,
        occurrence: Some(location(digest, start, end)),
        related_public_locations: vec![],
    })
    .collect();
    ExpressionProjectIr {
        schema: profile::PROJECT_IR_SCHEMA.into(),
        modules: vec![ProjectModule {
            identity: LogicalModuleIdentity::new("1.0", "example::expressions"),
            imports: vec![],
        }],
        declarations,
        vocabularies: vec![],
        interface_digest: SemanticDigest::from_raw_bytes(
            hex(if template {
                "2ed550d461d3c23aa7f3e361179240a41f5c2bac30d7bce8c205c3d73815b312"
            } else {
                "a29abbf55eb9cfc3b315af7d909499d50f44cd8593d445e3fb4abf7ad9206791"
            })
            .try_into()
            .unwrap(),
        ),
        sources: vec![ProjectSource {
            module: "example::expressions".into(),
            source_id: "source:identity-fixture".into(),
            digest,
            byte_len: source.len() as u64,
        }],
        source_maps,
        provenance: vec![],
        limits: ProjectLimits {
            modules: 1000,
            declarations: 1000,
            import_edges: 1000,
            nodes: 1000,
            text_bytes: 1_048_576,
            artifact_bytes: 1_048_576,
        },
        resources: ProjectResourceFacts {
            source_units: 1,
            source_bytes: source.len() as u64,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: if template { 1 } else { 3 },
            import_edges: 0,
            value_nodes: if template { 1 } else { 2 },
        },
        vocabulary_sources: vec![],
        origins: vec![],
        composition_limits: CompositionPolicy::from_values([
            1000, 1_048_576, 1000, 1000, 64, 1000, 1000, 1000, 1000, 1000, 1000, 64, 64, 1000,
            100_000,
        ]),
        composition_resources: CompositionResourceFacts::from_values([
            0,
            0,
            0,
            0,
            0,
            0,
            if template { 1 } else { 2 },
        ]),
        semantic_profiles: SemanticProfiles::try_frozen().unwrap(),
        public_templates: if template {
            vec![PublicTemplate {
                identity: symbol("Identity"),
                parameters: vec![parameter("value")],
                result_type: T::Num,
                body: Expression {
                    ty: T::Num,
                    node: N::Bound { depth: 0, slot: 0 },
                },
            }]
        } else {
            vec![]
        },
        expression_limits: ExpressionLimits {
            nodes: 1000,
            syntax_depth: 64,
            evaluation_depth: 64,
            bindings: 1000,
            dependency_edges: 1000,
            work: 100_000,
            retained_bytes: 1_048_576,
            output_bytes: 1_048_576,
            coefficient_digits: 4096,
            absolute_scale: 4096,
            diagnostics: 1000,
        },
        expansion_limits: ExpansionLimits {
            definitions: 1000,
            applications: 1000,
            parameters: 64,
            axes: 16,
            axis_items: 1000,
            product_tuples: 1000,
            aggregate_tuples: 1000,
            expanded_bindings: 1000,
            expanded_nodes: 1000,
            depth: 64,
            work: 100_000,
            retained_bytes: 1_048_576,
            output_bytes: 1_048_576,
        },
        expression_resources: if template {
            ExpressionResourceFacts {
                residual_nodes: 1,
                residual_depth: 1,
                public_templates: 1,
                retained_expression_text_bytes: 8,
                ..ExpressionResourceFacts::default()
            }
        } else {
            ExpressionResourceFacts {
                residual_nodes: 6,
                residual_depth: 3,
                input_declarations: 2,
                potential_value_edges: 2,
                retained_expression_text_bytes: 114,
                ..ExpressionResourceFacts::default()
            }
        },
        expression_attributions,
        expansion_attributions: if template {
            vec![ExpansionAttribution {
                owner: symbol("result"),
                application: location(digest, 124, 154),
                definition: location(digest, 39, 102),
                arguments: vec![location(digest, 152, 153)],
                original_axis_indices: vec![],
            }]
        } else {
            vec![]
        },
    }
}

/// The frozen minimal project retains materialized division and its actual original-byte origin.
fn static_project(literal: bool) -> ExpressionProjectIr {
    let source: &[u8] = if literal {
        include_bytes!("fixtures/expression/identity-literal.neu")
    } else {
        include_bytes!("fixtures/expression/identity.neu")
    };
    let mut ir = project(true);
    let digest = SourceContentDigest::from_bytes(source);
    let original = location(digest, 39, 64);
    ir.declarations[0].body = Some(ExpressionBody::Value(V::Number(number("0.125"))));
    ir.sources[0].digest = digest;
    ir.sources[0].byte_len = source.len() as u64;
    ir.source_maps = vec![ProjectSourceMap {
        declaration: symbol("result"),
        location: original,
    }];
    ir.interface_digest = SemanticDigest::from_raw_bytes(
        hex("62dfa284a3084041d9daa45f4f94c04c70e47eafd620bdfecd89551e2d594881")
            .try_into()
            .unwrap(),
    );
    ir.resources.source_bytes = source.len() as u64;
    ir.origins = vec![CompositionOrigin {
        binding: symbol("result"),
        path: vec![],
        kind: ValueOriginKind::Supplied,
        attribution: Some(CompositionAttribution::Source(original)),
    }];
    ir.public_templates.clear();
    ir.expression_resources = ExpressionResourceFacts::default();
    ir.expression_attributions.clear();
    ir.expansion_attributions.clear();
    ir
}

/// Read a known immutable fixture field, without adding a JSON dependency to the IR crate.
fn field<'a>(section: &'a str, name: &str) -> &'a str {
    section
        .split_once(format!("\"{name}\": \"").as_str())
        .unwrap()
        .1
        .split_once('"')
        .unwrap()
        .0
}

/// Selects a known complete fixture without interpreting its structural data.
fn section(id: &str) -> &str {
    VECTORS
        .split_once(format!("\"id\": \"{id}\"").as_str())
        .unwrap()
        .1
        .split("\"id\":")
        .next()
        .unwrap()
}

/// Decodes immutable literal bytes; expected transcripts are never regenerated.
fn hex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2));
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|digits| u8::from_str_radix(std::str::from_utf8(digits).unwrap(), 16).unwrap())
        .collect()
}

/// Borrowed names, complete transcript bytes and digests for the five distinct layers.
type Partitions = [(&'static str, Vec<u8>, [u8; 32]); 5];

/// Each layer consumes the preceding typed layer, never fake /2 digest conversions.
fn partitions(
    ir: &ExpressionProjectIr,
    producer_version: &str,
    kind: ArtifactKind,
    roots: &[ModuleSymbolIdentity],
) -> Partitions {
    let cancel = CancellationToken::new();
    let sources: Vec<_> = ir
        .sources
        .iter()
        .map(|source| CapturedIdentitySource {
            module: &source.module,
            source_id: &source.source_id,
            digest: source.digest,
            byte_len: source.byte_len,
        })
        .collect();
    let captured = captured_expression_closure(
        &CapturedIdentityInput {
            profile: "1.0",
            sources: &sources,
            vocabularies: &[],
        },
        &ir.semantic_profiles,
        LIMITS,
        &cancel,
    )
    .unwrap();
    let logical = canonical_expression_project(ir, LIMITS, &cancel).unwrap();
    let interface = expression_interface(ir, LIMITS, &cancel).unwrap();
    let derivation = expression_derivation_identity(
        logical.identity(),
        &ExpressionDerivationContext {
            captured: captured.identity(),
            producer: "independent-fixture-oracle",
            producer_version,
            capture_limits: CAPTURE_LIMITS,
            project_limits: ir.limits,
            composition_limits: ir.composition_limits,
            semantic_profiles: &ir.semantic_profiles,
            expression_limits: ir.expression_limits,
            expansion_limits: ir.expansion_limits,
        },
        LIMITS,
        &cancel,
    )
    .unwrap();
    let artifact = expression_artifact_identity(
        derivation.identity(),
        &ArtifactIdentityInput {
            kind,
            format: profile::ENCODING,
            roots,
            options: &[],
        },
        LIMITS,
        &cancel,
    )
    .unwrap();
    [
        (
            "captured",
            captured.bytes().to_vec(),
            captured.identity().as_bytes(),
        ),
        (
            "logical",
            logical.bytes().to_vec(),
            logical.identity().as_bytes(),
        ),
        (
            "interface",
            interface.bytes().to_vec(),
            interface.identity().as_bytes(),
        ),
        (
            "derivation",
            derivation.bytes().to_vec(),
            derivation.identity().as_bytes(),
        ),
        (
            "artifact",
            artifact.bytes().to_vec(),
            artifact.identity().as_bytes(),
        ),
    ]
}

#[test]
fn all_five_partitions_match_complete_frozen_residual_and_template_bytes() {
    for (template, id) in [
        (false, "complete-guarded-residual"),
        (true, "complete-public-template"),
    ] {
        let expected = section(id);
        for (name, bytes, digest) in partitions(&project(template), "1", ArtifactKind::Project, &[])
        {
            let partition = expected
                .split_once(format!("\"{name}\": {{").as_str())
                .unwrap()
                .1;
            assert_eq!(
                bytes,
                hex(field(partition, "transcript_hex")),
                "{id}/{name} bytes"
            );
            assert_eq!(
                digest.as_slice(),
                hex(field(partition, "sha256")),
                "{id}/{name} digest"
            );
        }
    }
}

#[test]
fn all_five_partitions_match_complete_frozen_minimal_static_bytes() {
    let ids = [
        "complete-captured",
        "complete-logical",
        "public-interface",
        "complete-derivation",
        "complete-artifact",
    ];
    for ((name, bytes, digest), id) in
        partitions(&static_project(false), "1", ArtifactKind::Project, &[])
            .into_iter()
            .zip(ids)
    {
        let expected = STATIC_VECTORS
            .split_once(format!("\"id\": \"{id}\"").as_str())
            .unwrap()
            .1
            .split("\"id\":")
            .next()
            .unwrap();
        assert_eq!(
            bytes,
            hex(field(expected, "transcript_hex")),
            "static/{name} bytes"
        );
        assert_eq!(
            digest.as_slice(),
            hex(field(expected, "sha256")),
            "static/{name} digest"
        );
    }
    check_variation(
        "folded-vs-literal-source-evidence",
        &static_project(true),
        "1",
        ArtifactKind::Project,
        &[],
    );
}

/// Compares every partition to the frozen independently calculated variation digest.
fn check_variation(
    id: &str,
    ir: &ExpressionProjectIr,
    revision: &str,
    kind: ArtifactKind,
    roots: &[ModuleSymbolIdentity],
) {
    let expected = section(id).split_once("\"expected_digests\":").unwrap().1;
    for (name, _, digest) in partitions(ir, revision, kind, roots) {
        assert_eq!(digest.as_slice(), hex(field(expected, name)), "{id}/{name}");
    }
}

#[test]
fn frozen_context_and_residual_variations_keep_their_exact_partitions() {
    let mut ir = project(false);
    ir.expression_limits.nodes = 999;
    check_variation(
        "acceptance-limit-not-meaning",
        &ir,
        "1",
        ArtifactKind::Project,
        &[],
    );
    ir.expression_limits.nodes = 1000;
    check_variation(
        "producer-revision-not-meaning",
        &ir,
        "2",
        ArtifactKind::Project,
        &[],
    );
    check_variation(
        "view-roots-artifact-only",
        &ir,
        "1",
        ArtifactKind::View,
        &[symbol("result")],
    );
    // Runtime values have no identity input channel; the frozen excluded request leaves all partitions intact.
    check_variation(
        "external-values-not-project-inputs",
        &ir,
        "1",
        ArtifactKind::Project,
        &[],
    );
    let Some(ExpressionBody::Expression(Expression {
        node: N::If { then_branch, .. },
        ..
    })) = &mut ir.declarations[2].body
    else {
        panic!("if tree")
    };
    let N::Binary { left, right, .. } = &mut then_branch.node else {
        panic!("binary tree")
    };
    std::mem::swap(left, right);
    check_variation(
        "residual-order-semantic",
        &ir,
        "1",
        ArtifactKind::Project,
        &[],
    );
}

#[test]
fn frozen_public_template_variations_retain_name_phase_and_bound_slot() {
    let mut ir = project(true);
    check_variation(
        "presentation-not-meaning",
        &ir,
        "1",
        ArtifactKind::Project,
        &[],
    );
    ir.public_templates[0].parameters[0].name = "argument".into();
    check_variation(
        "public-parameter-name-semantic",
        &ir,
        "1",
        ArtifactKind::Project,
        &[],
    );
    ir.public_templates[0].parameters[0].name = "value".into();
    ir.public_templates[0].parameters[0].phase = ParameterPhase::Deferred;
    check_variation(
        "template-phase-semantic",
        &ir,
        "1",
        ArtifactKind::Project,
        &[],
    );
    ir.public_templates[0].parameters[0].phase = ParameterPhase::Static;
    ir.public_templates[0]
        .parameters
        .insert(0, parameter("other"));
    ir.public_templates[0].body.node = N::Bound { depth: 0, slot: 1 };
    check_variation("bound-slot-semantic", &ir, "1", ArtifactKind::Project, &[]);
}

#[test]
fn private_values_and_source_companions_affect_only_the_governed_partitions() {
    let mut ir = project(false);
    ir.declarations.push(ExpressionDeclaration {
        public: false,
        ..declaration(
            "secret",
            T::Num,
            ExpressionBody::Value(V::Number(number("2"))),
        )
    });
    let before = partitions(&ir, "1", ArtifactKind::Project, &[]);
    ir.declarations[3].body = Some(ExpressionBody::Value(V::Number(number("3"))));
    let changed = partitions(&ir, "1", ArtifactKind::Project, &[]);
    assert_ne!(before[1].2, changed[1].2);
    assert_eq!(before[2].2, changed[2].2);
    ir.sources[0].source_id = "another-logical-source-id".into();
    ir.source_maps.clear();
    ir.expression_attributions.clear();
    ir.interface_digest = SemanticDigest::from_raw_bytes([9; 32]);
    ir.resources.source_bytes = 999;
    ir.expression_resources.residual_nodes = 999;
    let evidence = partitions(&ir, "1", ArtifactKind::Project, &[]);
    assert_ne!(changed[0].2, evidence[0].2);
    assert_eq!(changed[1].2, evidence[1].2);
    assert_eq!(changed[2].2, evidence[2].2);
    assert_ne!(changed[3].2, evidence[3].2);
}

#[test]
fn restrictions_and_unused_closed_defaults_are_complete_logical_meaning() {
    let mut ir = project(true);
    ir.declarations.insert(
        0,
        ExpressionDeclaration {
            identity: symbol("Contract"),
            public: true,
            signature: S::Definition(CompositionBody::Record(vec![CompositionField {
                name: "count".into(),
                ty: T::Num,
                presence: FieldPresence::Defaulted,
                restrictions: FieldRestrictions {
                    minimum: Some(number("1")),
                    ..FieldRestrictions::default()
                },
                default: Some(neutral_ir::composition::ClosedValue::Number(number("2"))),
            }])),
            body: None,
        },
    );
    let original = partitions(&ir, "1", ArtifactKind::Project, &[]);
    assert_contract_frames(&original[1].1, b"1", b"2");
    let S::Definition(CompositionBody::Record(fields)) = &mut ir.declarations[0].signature else {
        panic!("record")
    };
    fields[0].default = Some(neutral_ir::composition::ClosedValue::Number(number("3")));
    let default = partitions(&ir, "1", ArtifactKind::Project, &[]);
    assert_contract_frames(&default[1].1, b"1", b"3");
    assert_ne!(original[1].2, default[1].2);
    assert_ne!(original[2].2, default[2].2);
    let S::Definition(CompositionBody::Record(fields)) = &mut ir.declarations[0].signature else {
        panic!("record")
    };
    fields[0].restrictions.minimum = Some(number("0"));
    let restriction = partitions(&ir, "1", ArtifactKind::Project, &[]);
    assert_contract_frames(&restriction[1].1, b"0", b"3");
    assert_ne!(default[1].2, restriction[1].2);
    assert_ne!(default[2].2, restriction[2].2);
    assert_eq!(original[0].2, restriction[0].2);
}

/// Independently reads literal restriction/default bytes from the complete logical partition.
fn assert_contract_frames(transcript: &[u8], minimum: &[u8], default: &[u8]) {
    let root = framed(transcript, "neutral-nht-v1");
    let logical = framed(root, "neutral/project-logical/v3");
    let declaration = framed(framed(logical, "declarations"), "declaration");
    let field = framed(framed(framed(declaration, "signature"), "record"), "field");
    assert_eq!(framed(field, "name"), b"count");
    assert_eq!(framed(field, "num"), b"");
    assert_eq!(framed(field, "presence"), b"defaulted");
    let restrictions = framed(field, "restrictions");
    assert_eq!(framed(restrictions, "choices"), b"");
    for key in ["maximum", "min-length", "max-length"] {
        assert_eq!(frames(framed(restrictions, key)), [("absent", &b""[..])]);
    }
    for (value, coefficient) in [
        (framed(framed(restrictions, "minimum"), "num"), minimum),
        (framed(framed(field, "default"), "num"), default),
    ] {
        assert_eq!(
            frames(value),
            [
                ("negative", &b"\x00"[..]),
                ("coefficient", coefficient),
                ("scale", &b"\x00\x00\x00\x00\x00\x00\x00\x00"[..]),
            ]
        );
    }
}

/// An independent NHT reader checks exact container boundaries without using production framing.
fn frames(mut bytes: &[u8]) -> Vec<(&str, &[u8])> {
    let mut result = vec![];
    while !bytes.is_empty() {
        let length = u16::from_be_bytes(bytes[..2].try_into().unwrap()) as usize;
        let tag = std::str::from_utf8(&bytes[2..2 + length]).unwrap();
        let size = usize::try_from(u64::from_be_bytes(
            bytes[2 + length..10 + length].try_into().unwrap(),
        ))
        .unwrap();
        let end = 10 + length + size;
        result.push((tag, &bytes[10 + length..end]));
        bytes = &bytes[end..];
    }
    result
}

/// Borrows one named frame from independently decoded container bytes.
fn framed<'a>(bytes: &'a [u8], tag: &str) -> &'a [u8] {
    frames(bytes)
        .into_iter()
        .find(|(name, _)| *name == tag)
        .unwrap()
        .1
}

/// Every tree kind is framed inside a complete project, including the two expression wrappers.
fn retained_tree(bytes: &[u8]) -> &[u8] {
    let outer = framed(bytes, "neutral-nht-v1");
    let logical = framed(outer, "neutral/project-logical/v3");
    let declarations = framed(logical, "declarations");
    let result = frames(declarations)
        .into_iter()
        .find(|(_, declaration)| framed(declaration, "declaration") == b"result")
        .unwrap()
        .1;
    framed(framed(framed(result, "value"), "expression"), "expression")
}

#[test]
#[allow(clippy::too_many_lines)] // One exhaustive table compares every frozen tree grammar.
fn every_frozen_tree_kind_has_the_exact_ordered_fields_and_complete_children() {
    let bool_leaf = || Expression {
        ty: T::Bool,
        node: N::Value(V::Bool(true)),
    };
    let string_leaf = || Expression {
        ty: T::String,
        node: N::Value(V::String("x".into())),
    };
    let cases: Vec<(Expression, &[&str])> = vec![
        (numeric("1.25"), &["kind", "type", "value"]),
        (input("count", T::Num), &["kind", "type", "symbol"]),
        (
            Expression {
                ty: T::String,
                node: N::Output {
                    target: symbol("operation"),
                    result_type: T::Nominal(symbol("Result")),
                    selector: vec![
                        SelectorSegment::Field("items".into()),
                        SelectorSegment::Index(u32::MAX),
                        SelectorSegment::Alternative("ready".into()),
                    ],
                },
            },
            &["kind", "type", "target", "result-type", "selector"],
        ),
        (
            Expression {
                ty: T::Num,
                node: N::Unary {
                    operator: UnaryOperator::Negate,
                    operand: Box::new(numeric("1")),
                },
            },
            &["kind", "type", "operator", "operand"],
        ),
        (
            Expression {
                ty: T::Num,
                node: N::Binary {
                    operator: BinaryOperator::Divide,
                    left: Box::new(numeric("1")),
                    right: Box::new(numeric("8")),
                },
            },
            &["kind", "type", "operator", "left", "right"],
        ),
        (
            Expression {
                ty: T::Num,
                node: N::If {
                    guard: Box::new(bool_leaf()),
                    then_branch: Box::new(numeric("1")),
                    else_branch: Box::new(numeric("0")),
                },
            },
            &["kind", "type", "guard", "then", "else"],
        ),
        (
            Expression {
                ty: T::Num,
                node: N::Match {
                    scrutinee: Box::new(Expression {
                        ty: T::Nominal(symbol("State")),
                        node: N::Variant {
                            tag: "a".into(),
                            payload: Box::new(numeric("1")),
                        },
                    }),
                    arms: vec![
                        MatchArm {
                            tag: "a".into(),
                            payload_type: T::Num,
                            body: Expression {
                                ty: T::Num,
                                node: N::Bound { depth: 0, slot: 0 },
                            },
                        },
                        MatchArm {
                            tag: "b".into(),
                            payload_type: T::String,
                            body: numeric("2"),
                        },
                    ],
                },
            },
            &["kind", "type", "scrutinee", "arms"],
        ),
        (
            Expression {
                ty: T::String,
                node: N::Text(Box::new(numeric("1"))),
            },
            &["kind", "type", "operand"],
        ),
        (
            Expression {
                ty: T::String,
                node: N::Interpolate(vec![string_leaf(), string_leaf()]),
            },
            &["kind", "type", "items"],
        ),
        (
            Expression {
                ty: T::List(Box::new(T::Num)),
                node: N::List(vec![numeric("1"), numeric("2")]),
            },
            &["kind", "type", "items"],
        ),
        (
            Expression {
                ty: T::Nominal(symbol("Record")),
                node: N::Record(vec![("a".into(), None), ("b".into(), Some(numeric("1")))]),
            },
            &["kind", "type", "fields"],
        ),
        (
            Expression {
                ty: T::Nominal(symbol("State")),
                node: N::Variant {
                    tag: "a".into(),
                    payload: Box::new(numeric("1")),
                },
            },
            &["kind", "type", "tag", "payload"],
        ),
    ];
    for (expression, expected) in cases {
        let kind = expression.node.kind();
        let mut ir = project(false);
        ir.declarations[2].body = Some(ExpressionBody::Expression(expression));
        let transcript =
            canonical_expression_project(&ir, LIMITS, &CancellationToken::new()).unwrap();
        let tree = retained_tree(transcript.bytes());
        assert_eq!(framed(tree, "kind"), kind.as_bytes());
        assert_eq!(
            frames(tree).iter().map(|(tag, _)| *tag).collect::<Vec<_>>(),
            expected,
            "{kind}"
        );
        match kind {
            "value" => {
                let number = framed(framed(tree, "value"), "num");
                assert_eq!(framed(number, "coefficient"), b"125");
                assert_eq!(
                    framed(number, "scale"),
                    &[255, 255, 255, 255, 255, 255, 255, 254]
                );
            }
            "output" => {
                let segments = frames(framed(tree, "selector"));
                assert_eq!(
                    segments.iter().map(|(tag, _)| *tag).collect::<Vec<_>>(),
                    ["segment"; 3]
                );
                assert_eq!(framed(segments[0].1, "kind"), b"field");
                assert_eq!(framed(segments[1].1, "kind"), b"index");
                assert_eq!(
                    framed(segments[1].1, "index"),
                    &[0, 0, 0, 0, 255, 255, 255, 255]
                );
                assert_eq!(framed(segments[2].1, "kind"), b"alternative");
            }
            "match" => {
                let arms = frames(framed(tree, "arms"));
                assert_eq!(
                    frames(arms[0].1)
                        .iter()
                        .map(|(tag, _)| *tag)
                        .collect::<Vec<_>>(),
                    ["tag", "payload-type", "body"]
                );
                let bound = framed(framed(arms[0].1, "body"), "expression");
                assert_eq!(
                    frames(bound)
                        .iter()
                        .map(|(tag, _)| *tag)
                        .collect::<Vec<_>>(),
                    ["kind", "type", "depth", "slot"]
                );
                assert_eq!(framed(bound, "depth"), &[0; 8]);
                assert_eq!(framed(bound, "slot"), &[0; 8]);
            }
            "record" => {
                let fields = frames(framed(tree, "fields"));
                assert_eq!(framed(framed(fields[0].1, "value"), "omitted"), &[]);
                assert_eq!(
                    framed(framed(framed(fields[1].1, "value"), "expression"), "kind"),
                    b"value"
                );
            }
            _ => {}
        }
    }
}

#[test]
fn template_defaults_are_complete_trees_and_absence_is_not_typed_null() {
    let mut ir = project(true);
    let absent = canonical_expression_project(&ir, LIMITS, &CancellationToken::new()).unwrap();
    ir.public_templates[0].parameters[0].default = Some(numeric("1"));
    let default = canonical_expression_project(&ir, LIMITS, &CancellationToken::new()).unwrap();
    assert_ne!(absent.identity(), default.identity());
    let body = framed(
        framed(default.bytes(), "neutral-nht-v1"),
        "neutral/project-logical/v3",
    );
    let template = framed(framed(body, "public-templates"), "template");
    let parameter = framed(framed(template, "parameters"), "parameter");
    assert_eq!(
        frames(parameter)
            .iter()
            .map(|(tag, _)| *tag)
            .collect::<Vec<_>>(),
        ["name", "phase", "type", "default"]
    );
    assert_eq!(
        framed(framed(framed(parameter, "default"), "expression"), "kind"),
        b"value"
    );
    ir.public_templates[0].parameters[0].default = Some(Expression {
        ty: T::Nullable(Box::new(T::Num)),
        node: N::Value(V::Null),
    });
    let null = canonical_expression_project(&ir, LIMITS, &CancellationToken::new()).unwrap();
    assert_ne!(default.identity(), null.identity());
    assert_ne!(absent.identity(), null.identity());
}

#[test]
fn malformed_order_bodies_profiles_and_overdeep_unselected_trees_reject() {
    let cancel = CancellationToken::new();
    let mut ir = project(false);
    ir.declarations.swap(0, 1);
    assert_eq!(
        canonical_expression_project(&ir, LIMITS, &cancel),
        Err(IdentityError::InvalidInput)
    );
    ir.declarations.swap(0, 1);
    ir.declarations[0].body = None;
    assert_eq!(
        canonical_expression_project(&ir, LIMITS, &cancel),
        Err(IdentityError::InvalidInput)
    );
    ir.declarations[0].body = Some(ExpressionBody::Input);
    ir.semantic_profiles.required_features.swap(0, 1);
    assert_eq!(
        canonical_expression_project(&ir, LIMITS, &cancel),
        Err(IdentityError::InvalidInput)
    );
    ir.semantic_profiles = SemanticProfiles::try_frozen().unwrap();
    let mut deep = numeric("1");
    for _ in 0..63 {
        deep = Expression {
            ty: T::Num,
            node: N::Unary {
                operator: UnaryOperator::Negate,
                operand: Box::new(deep),
            },
        };
    }
    let Some(ExpressionBody::Expression(Expression {
        node: N::If { else_branch, .. },
        ..
    })) = &mut ir.declarations[2].body
    else {
        panic!("if tree")
    };
    **else_branch = deep;
    assert_eq!(
        canonical_expression_project(&ir, LIMITS, &cancel),
        Err(IdentityError::Limit)
    );
    assert!(canonical_expression_project(&project(false), LIMITS, &cancel).is_ok());
}

/// Counts complete frame trees; scalar payloads are not interpreted as frames.
fn frame_count(mut bytes: &[u8]) -> Option<u64> {
    let mut count = 0;
    while !bytes.is_empty() {
        let length = u16::from_be_bytes(bytes.get(..2)?.try_into().ok()?) as usize;
        if length == 0 || !bytes.get(2..2 + length)?.is_ascii() {
            return None;
        }
        let size = usize::try_from(u64::from_be_bytes(
            bytes.get(2 + length..10 + length)?.try_into().ok()?,
        ))
        .ok()?;
        let end = (10 + length).checked_add(size)?;
        let payload = bytes.get(10 + length..end)?;
        count += 1 + frame_count(payload).unwrap_or(0);
        bytes = bytes.get(end..)?;
    }
    Some(count)
}

/// Every observed failed reservation/cancellation must preserve the next successful request.
fn atomic_partition<I: Copy + Eq + std::fmt::Debug>(
    build: impl Fn(IdentityLimits, &CancellationToken) -> Result<IdentityTranscript<I>, IdentityError>,
) {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let cancel = CancellationToken::new();
    let (success, reservations) = observe(None, || build(LIMITS, &cancel));
    let success = success.unwrap();
    let exact = IdentityLimits {
        bytes: success.bytes().len() as u64,
        ..LIMITS
    };
    assert_eq!(build(exact, &cancel).unwrap(), success);
    assert_eq!(
        build(
            IdentityLimits {
                bytes: exact.bytes - 1,
                ..LIMITS
            },
            &cancel
        ),
        Err(IdentityError::Limit)
    );
    let nodes = frame_count(success.bytes()).unwrap();
    assert_eq!(
        build(IdentityLimits { nodes, ..LIMITS }, &cancel).unwrap(),
        success
    );
    assert_eq!(
        build(
            IdentityLimits {
                nodes: nodes - 1,
                ..LIMITS
            },
            &cancel
        ),
        Err(IdentityError::Limit)
    );
    assert_eq!(
        build(IdentityLimits { nodes: 0, ..LIMITS }, &cancel),
        Err(IdentityError::Limit)
    );
    assert_eq!(
        build(IdentityLimits { bytes: 0, ..LIMITS }, &cancel),
        Err(IdentityError::Limit)
    );
    for index in 0..reservations {
        let (result, _) = observe(Some(index), || build(LIMITS, &cancel));
        assert_eq!(result, Err(IdentityError::Limit), "reservation {index}");
        let signal = CancellationToken::new();
        let (result, _) = observe_cancellation(index, &signal, || build(LIMITS, &signal));
        assert_eq!(
            result,
            Err(IdentityError::Cancelled),
            "cancellation {index}"
        );
        assert_eq!(build(LIMITS, &cancel).unwrap(), success);
    }
}

#[test]
fn all_partitions_enforce_exact_bytes_nodes_and_atomic_fault_cancellation_recovery() {
    let ir = project(false);
    let cancel = CancellationToken::new();
    let sources = [CapturedIdentitySource {
        module: &ir.sources[0].module,
        source_id: &ir.sources[0].source_id,
        digest: ir.sources[0].digest,
        byte_len: ir.sources[0].byte_len,
    }];
    let input = CapturedIdentityInput {
        profile: "1.0",
        sources: &sources,
        vocabularies: &[],
    };
    let captured =
        captured_expression_closure(&input, &ir.semantic_profiles, LIMITS, &cancel).unwrap();
    let logical = canonical_expression_project(&ir, LIMITS, &cancel).unwrap();
    let context = ExpressionDerivationContext {
        captured: captured.identity(),
        producer: "independent-fixture-oracle",
        producer_version: "1",
        capture_limits: CAPTURE_LIMITS,
        project_limits: ir.limits,
        composition_limits: ir.composition_limits,
        semantic_profiles: &ir.semantic_profiles,
        expression_limits: ir.expression_limits,
        expansion_limits: ir.expansion_limits,
    };
    let derivation =
        expression_derivation_identity(logical.identity(), &context, LIMITS, &cancel).unwrap();
    let artifact = ArtifactIdentityInput {
        kind: ArtifactKind::Project,
        format: profile::ENCODING,
        roots: &[],
        options: &[],
    };
    atomic_partition(|limits, token| {
        captured_expression_closure(&input, &ir.semantic_profiles, limits, token)
    });
    atomic_partition(|limits, token| canonical_expression_project(&ir, limits, token));
    atomic_partition(|limits, token| expression_interface(&ir, limits, token));
    atomic_partition(|limits, token| {
        expression_derivation_identity(logical.identity(), &context, limits, token)
    });
    atomic_partition(|limits, token| {
        expression_artifact_identity(derivation.identity(), &artifact, limits, token)
    });
}
