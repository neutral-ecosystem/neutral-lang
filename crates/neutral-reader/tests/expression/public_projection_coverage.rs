// SPDX-License-Identifier: Apache-2.0

//! Public closure retains exact vocabulary contracts while redacting private source evidence.

use super::*;
use crate::expression::{
    ExpressionReadError, ExpressionViewLimits, ExpressionViewRequest, ValidatedExpressionProject,
};
use neutral_core::VocabularyContentDigest;
use neutral_ir::expression::SelectorSegment;
use neutral_ir::{
    VocabularyIdentity,
    composition::{
        CompositionAlternative, CompositionBundle, CompositionDefinition, CompositionDependency,
    },
};
use neutral_semantics::{AvailabilityRequirement, ExpressionRoot};

/// Publishes only independently accepted authority from a complete producer fixture.
fn authority(ir: ExpressionProjectIr) -> ValidatedExpressionProject {
    ValidatedExpressionProject::from_ir(complete(ir), limits(), &CancellationToken::new()).unwrap()
}

/// Retains one native vocabulary default and its exact captured attribution.
pub(super) fn vocabulary_fixture() -> ExpressionProjectIr {
    let mut ir = default_fixture(loc("m", 20, 21));
    let definition = ir.declarations.remove(0);
    ir.source_maps.remove(0);
    ir.provenance.clear();
    let S::Definition(body) = definition.signature else {
        unreachable!()
    };
    let ty = T::VocabularyNominal {
        identity: "Fixture".into(),
        version: "1.0.0".into(),
        name: "Record".into(),
    };
    ir.declarations[0].signature = S::Binding(ty);
    let digest = VocabularyContentDigest::from_bytes(b"captured-vocabulary");
    ir.vocabularies.push(CompositionBundle {
        identity: VocabularyIdentity::new(
            "Fixture",
            "1.0.0",
            "2.0",
            "1.0",
            digest,
            vec!["vocabulary-composition-v2".into()],
        ),
        dependencies: vec![CompositionDependency {
            identity: "Leaf".into(),
            version: "1.0.0".into(),
        }],
        definitions: vec![
            CompositionDefinition {
                name: "PrivateSecret".into(),
                public: false,
                body: CompositionBody::Record(vec![CompositionField {
                    name: "leaf".into(),
                    ty: T::VocabularyNominal {
                        identity: "Leaf".into(),
                        version: "1.0.0".into(),
                        name: "LeafRecord".into(),
                    },
                    presence: FieldPresence::Required,
                    restrictions: FieldRestrictions::default(),
                    default: None,
                }]),
            },
            CompositionDefinition {
                name: "Record".into(),
                public: true,
                body,
            },
        ],
    });
    ir.vocabulary_sources
        .push(neutral_ir::project::ProjectVocabularySource {
            identity: "Fixture".into(),
            version: "1.0.0".into(),
            digest,
            byte_len: 200,
        });
    ir.vocabularies.push(CompositionBundle {
        identity: VocabularyIdentity::new(
            "Leaf",
            "1.0.0",
            "2.0",
            "1.0",
            digest,
            vec!["vocabulary-composition-v2".into()],
        ),
        dependencies: vec![],
        definitions: vec![CompositionDefinition {
            name: "LeafRecord".into(),
            public: true,
            body: CompositionBody::Record(vec![]),
        }],
    });
    ir.vocabulary_sources
        .push(neutral_ir::project::ProjectVocabularySource {
            identity: "Leaf".into(),
            version: "1.0.0".into(),
            digest,
            byte_len: 200,
        });
    ir.origins[1].attribution = Some(CompositionAttribution::Vocabulary {
        identity: "Fixture".into(),
        version: "1.0.0".into(),
        type_name: "Record".into(),
        field_name: "x".into(),
        span: Some(ByteSpan::new(10, 20).unwrap()),
    });
    ir
}

/// Public vocabulary metadata remains exact while private definitions and captured evidence stay hidden.
#[test]
fn vocabulary_default_projection_retains_exact_public_contract_and_redacts_evidence() {
    let project = authority(vocabulary_fixture());
    let roots = [symbol("m", "a")];
    let cancel = CancellationToken::new();
    let view = project
        .derive_view(
            &ExpressionViewRequest {
                schema: "neutral.project-view/3",
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(view.schema(), "neutral.project-view/3");
    let vocabularies = view.vocabularies().collect::<Vec<_>>();
    assert_eq!(vocabularies.len(), 2);
    let vocabulary = &vocabularies[0];
    assert_eq!(vocabulary.identity(), "Fixture");
    assert_eq!(vocabulary.version(), "1.0.0");
    assert_eq!(vocabulary.schema_version(), "2.0");
    assert_eq!(
        vocabulary.required_features(),
        ["vocabulary-composition-v2"]
    );
    assert_eq!(
        vocabulary.dependencies(),
        [CompositionDependency {
            identity: "Leaf".into(),
            version: "1.0.0".into()
        }]
    );
    assert_eq!(
        vocabulary
            .definitions()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>(),
        ["Record"]
    );
    assert_eq!(vocabularies[1].identity(), "Leaf");
    assert_eq!(vocabularies[1].definitions().count(), 0);
    assert_eq!(
        view.origins()
            .map(|o| (o.path.to_vec(), o.kind))
            .collect::<Vec<_>>(),
        [
            (vec![], ValueOriginKind::Supplied),
            (
                vec![ValuePathSegment::Field("x".into())],
                ValueOriginKind::Defaulted
            ),
        ]
    );
    assert_eq!(view.availability_obligations().count(), 0);
    assert_eq!(view.restriction_obligations().count(), 0);
    for debug in [format!("{view:?}"), format!("{project:?}")] {
        for forbidden in ["PrivateSecret", "source:m", "captured-vocabulary"] {
            assert!(!debug.contains(forbidden));
        }
    }
    assert!(std::ptr::eq(project.ir(), project.complete_ir().as_ref()));
    let contract = T::VocabularyNominal {
        identity: "Fixture".into(),
        version: "1.0.0".into(),
        name: "Record".into(),
    };
    let entries = project
        .entries(
            &crate::consumer::EntryRequest {
                schema: "neutral.entry-request/1",
                contract: &contract,
                selection: crate::consumer::EntrySelection::Unique,
            },
            &project.ir().semantic_profiles,
            ExpressionViewLimits::default(),
            &cancel,
        )
        .unwrap();
    assert_eq!(entries.bindings()[0].identity, symbol("m", "a"));
    assert!(!format!("{entries:?}").contains("PrivateSecret"));
}

/// Constructs independently typed nested materialized paths, including explicit null occurrences.
fn nested_origin_fixture() -> ExpressionProjectIr {
    let mut ir = empty();
    for (name, start, body) in [
        (
            "Inner",
            10,
            CompositionBody::Record(vec![CompositionField {
                name: "x".into(),
                ty: T::Num,
                presence: FieldPresence::Defaulted,
                restrictions: FieldRestrictions::default(),
                default: Some(neutral_ir::composition::ClosedValue::Number(
                    ExactNumber::from_normalized_parts(false, "1", 0, 4096, 4096).unwrap(),
                )),
            }]),
        ),
        (
            "Choice",
            50,
            CompositionBody::Variant(vec![CompositionAlternative {
                tag: "inner".into(),
                ty: T::Nominal(symbol("m", "Inner")),
            }]),
        ),
        (
            "Outer",
            90,
            CompositionBody::Record(vec![
                CompositionField {
                    name: "items".into(),
                    ty: T::List(Box::new(T::Nominal(symbol("m", "Choice")))),
                    presence: FieldPresence::Required,
                    restrictions: FieldRestrictions::default(),
                    default: None,
                },
                CompositionField {
                    name: "nothing".into(),
                    ty: T::Nullable(Box::new(T::Num)),
                    presence: FieldPresence::Required,
                    restrictions: FieldRestrictions::default(),
                    default: None,
                },
            ]),
        ),
    ] {
        ir.declarations.push(ExpressionDeclaration {
            identity: symbol("m", name),
            public: true,
            signature: S::Definition(body),
            body: None,
        });
        ir.source_maps.push(ProjectSourceMap {
            declaration: symbol("m", name),
            location: loc("m", start, start + 30),
        });
    }
    let binding = symbol("m", "a");
    ir.declarations.push(ExpressionDeclaration {
        identity: binding.clone(),
        public: true,
        signature: S::Binding(T::Nominal(symbol("m", "Outer"))),
        body: Some(ExpressionBody::Value(V::Record(vec![
            (
                "items".into(),
                Some(V::List(vec![V::Variant {
                    tag: "inner".into(),
                    payload: Box::new(V::Record(vec![("x".into(), Some(one()))])),
                }])),
            ),
            ("nothing".into(), Some(V::Null)),
        ]))),
    });
    ir.source_maps.push(ProjectSourceMap {
        declaration: binding.clone(),
        location: loc("m", 130, 180),
    });
    for (from, to, start) in [
        ("Choice", "Inner", 51),
        ("Outer", "Choice", 91),
        ("a", "Outer", 131),
    ] {
        ir.provenance.push(ProjectProvenance {
            from: symbol("m", from),
            to: symbol("m", to),
            kind: K::Type,
            location: loc("m", start, start + 1),
        });
    }
    retain_nested_origins(&mut ir, &binding);
    ir
}

/// Retains the hand-checked list, variant payload, default-field and null occurrence paths.
fn retain_nested_origins(ir: &mut ExpressionProjectIr, binding: &ModuleSymbolIdentity) {
    let items = ValuePathSegment::Field("items".into());
    for (path, kind, attribution) in [
        (vec![], ValueOriginKind::Supplied, None),
        (vec![items.clone()], ValueOriginKind::Supplied, None),
        (
            vec![items.clone(), ValuePathSegment::Element(0)],
            ValueOriginKind::Supplied,
            None,
        ),
        (
            vec![
                items.clone(),
                ValuePathSegment::Element(0),
                ValuePathSegment::Payload,
            ],
            ValueOriginKind::Supplied,
            None,
        ),
        (
            vec![
                items,
                ValuePathSegment::Element(0),
                ValuePathSegment::Payload,
                ValuePathSegment::Field("x".into()),
            ],
            ValueOriginKind::Defaulted,
            Some(CompositionAttribution::Source(loc("m", 20, 21))),
        ),
        (
            vec![ValuePathSegment::Field("nothing".into())],
            ValueOriginKind::ExplicitNull,
            None,
        ),
    ] {
        ir.origins.push(CompositionOrigin {
            binding: binding.clone(),
            path,
            kind,
            attribution,
        });
    }
}

/// Element/payload paths and deeply introduced defaults must name actual typed occurrences.
#[test]
fn nested_origins_validate_lists_variants_and_nulls() {
    let project = authority(nested_origin_fixture());
    let roots = [symbol("m", "a")];
    let cancel = CancellationToken::new();
    let view = project
        .derive_view(
            &ExpressionViewRequest {
                schema: "neutral.project-view/3",
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(view.origins().count(), 6);
    assert_eq!(
        view.declarations()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["Choice", "Inner", "Outer", "a"]
    );
    for fault in 0..3 {
        let mut ir = nested_origin_fixture();
        match fault {
            0 => ir.origins[2].path[1] = ValuePathSegment::Element(1),
            1 => {
                ir.origins[4].attribution =
                    Some(CompositionAttribution::Source(loc("m", 140, 141)));
            }
            _ => ir.origins[5].kind = ValueOriginKind::OmittedOptional,
        }
        assert_eq!(check(ir), Err(E::Companion));
    }
}

/// A vocabulary origin must name its introducing default and remain inside captured bytes.
#[test]
fn vocabulary_default_attribution_rejects_forged_contracts_and_ranges() {
    for fault in 0..4 {
        let mut ir = vocabulary_fixture();
        let Some(CompositionAttribution::Vocabulary {
            identity,
            field_name,
            span,
            ..
        }) = &mut ir.origins[1].attribution
        else {
            unreachable!()
        };
        match fault {
            0 => *identity = "Other".into(),
            1 => *field_name = "missing".into(),
            2 => *span = Some(ByteSpan::new(199, 201).unwrap()),
            _ => ir.origins[1].kind = ValueOriginKind::Supplied,
        }
        assert_eq!(check(ir), Err(E::Companion));
    }
}

/// Expansion projection exposes checked original ranges with no source identity or private owner.
#[test]
fn expansion_view_retains_original_spans_and_template_contract() {
    let mut ir = expanded();
    scalar(&mut ir, "m", "private_secret", 100);
    ir.declarations.last_mut().unwrap().public = false;
    let project = authority(ir);
    let roots = [symbol("m", "result"), symbol("m", "Identity")];
    let view = project
        .derive_view(
            &ExpressionViewRequest {
                schema: "neutral.project-view/3",
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(
        view.roots()
            .map(ModuleSymbolIdentity::declaration_name)
            .collect::<Vec<_>>(),
        ["Identity", "result"]
    );
    assert_eq!(
        view.templates()
            .map(|t| t.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["Identity"]
    );
    let attributions = view.expression_attributions().collect::<Vec<_>>();
    assert_eq!(attributions.len(), 1);
    assert_eq!(attributions[0].owner, &symbol("m", "result"));
    assert_eq!(attributions[0].node_path, []);
    assert_eq!(attributions[0].span, Some(ByteSpan::new(39, 40).unwrap()));
    let expansions = view.expansion_attributions().collect::<Vec<_>>();
    assert_eq!(expansions.len(), 1);
    assert_eq!(expansions[0].owner, &symbol("m", "result"));
    assert_eq!(expansions[0].application, ByteSpan::new(60, 90).unwrap());
    assert_eq!(expansions[0].definition, ByteSpan::new(10, 50).unwrap());
    assert_eq!(
        expansions[0].arguments().collect::<Vec<_>>(),
        [ByteSpan::new(80, 81).unwrap()]
    );
    assert_eq!(expansions[0].original_axis_indices, []);
    assert!(!format!("{view:?}").contains("private_secret"));
}

/// Complete authority rejects schema, resource, semantic, companion and digest corruption distinctly.
#[test]
fn complete_reader_errors_keep_safe_classes_and_frozen_result_schema() {
    for (fault, expected) in [
        (0, ExpressionReadError::Schema),
        (1, ExpressionReadError::Resources),
        (2, ExpressionReadError::Semantic),
        (3, ExpressionReadError::Companion),
        (4, ExpressionReadError::Interface),
    ] {
        let mut ir = prepare(default_fixture(loc("m", 20, 21)));
        ir.interface_digest = SemanticDigest::from_raw_bytes(
            neutral_ir::project_identity::expression_interface(
                &ir,
                neutral_ir::project_identity::IdentityLimits {
                    bytes: ir.limits.artifact_bytes,
                    nodes: ir.limits.nodes,
                },
                &CancellationToken::new(),
            )
            .unwrap()
            .identity()
            .as_bytes(),
        );
        match fault {
            0 => ir.schema = "unknown".into(),
            1 => ir.resources.source_bytes += 1,
            2 => ir.declarations[1].body = Some(ExpressionBody::Value(V::Bool(true))),
            3 => ir.source_maps.pop().map(|_| ()).unwrap(),
            _ => ir.interface_digest = SemanticDigest::from_raw_bytes([0; 32]),
        }
        let error = ValidatedExpressionProject::from_ir(
            Shared::try_new(ir).unwrap(),
            limits(),
            &CancellationToken::new(),
        )
        .unwrap_err();
        assert_eq!(error, expected);
        assert_eq!(error.schema(), "neutral.project-result/3");
        assert!(!format!("{error:?}").contains("source:m"));
    }
}

/// Unsupported feature tuples and independent projection controls fail before retaining public closure.
#[test]
fn public_projection_and_entries_reject_inexact_profiles_and_limits() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let roots = [symbol("m", "a")];
    let mut profiles = SemanticProfiles::try_frozen().unwrap();
    profiles.required_features.pop();
    let cancel = CancellationToken::new();
    assert_eq!(
        project
            .derive_view(
                &ExpressionViewRequest {
                    schema: "neutral.project-view/3",
                    semantic_profiles: &profiles,
                    roots: &roots
                },
                &cancel
            )
            .unwrap_err(),
        ExpressionReadError::Schema
    );
    let contract = T::Nominal(symbol("m", "Record"));
    let request = crate::consumer::EntryRequest {
        schema: "neutral.entry-request/1",
        contract: &contract,
        selection: crate::consumer::EntrySelection::Unique,
    };
    assert_eq!(
        project
            .entries(
                &request,
                &profiles,
                ExpressionViewLimits::default(),
                &cancel
            )
            .unwrap_err(),
        crate::expression::ExpressionEntryError::Unsupported
    );
    for limits in [
        ExpressionViewLimits {
            roots: 0,
            ..Default::default()
        },
        ExpressionViewLimits {
            closure_edges: 0,
            ..Default::default()
        },
        ExpressionViewLimits {
            work: 1,
            ..Default::default()
        },
        ExpressionViewLimits {
            output_bytes: 1,
            ..Default::default()
        },
    ] {
        assert_eq!(
            project
                .derive_view_with_limits(
                    &ExpressionViewRequest {
                        schema: "neutral.project-view/3",
                        semantic_profiles: &project.ir().semantic_profiles,
                        roots: &roots
                    },
                    limits,
                    &cancel
                )
                .unwrap_err(),
            ExpressionReadError::Limit
        );
        assert_eq!(
            project
                .entries(&request, &project.ir().semantic_profiles, limits, &cancel)
                .unwrap_err(),
            crate::expression::ExpressionEntryError::Limit
        );
    }
}

/// Retains real future selector and field-restriction obligations on distinct public owners.
fn obligation_fixture() -> ExpressionProjectIr {
    let mut ir = empty();
    let restricted = T::Nominal(symbol("m", "Restricted"));
    let bucket = T::VocabularyNominal {
        identity: "Domain".into(),
        version: "1.0.0".into(),
        name: "Bucket".into(),
    };
    for (name, start, signature, body) in [
        (
            "Restricted",
            10,
            S::Definition(CompositionBody::Record(vec![CompositionField {
                name: "flag".into(),
                ty: T::Bool,
                presence: FieldPresence::Required,
                restrictions: FieldRestrictions {
                    choices: Some(vec![neutral_ir::composition::ClosedValue::Bool(true)]),
                    ..Default::default()
                },
                default: None,
            }])),
            None,
        ),
        (
            "external",
            40,
            S::Binding(T::Bool),
            Some(ExpressionBody::Input),
        ),
        (
            "operation",
            60,
            S::Binding(restricted.clone()),
            Some(ExpressionBody::Input),
        ),
        (
            "restricted",
            80,
            S::Binding(restricted.clone()),
            Some(ExpressionBody::Expression(Expression {
                ty: restricted,
                node: ExpressionNode::Record(vec![(
                    "flag".into(),
                    Some(Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Input(symbol("m", "external")),
                    }),
                )]),
            })),
        ),
        (
            "output",
            110,
            S::Binding(T::Bool),
            Some(ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Output {
                    target: symbol("m", "operation"),
                    result_type: bucket,
                    selector: vec![
                        SelectorSegment::Field("items".into()),
                        SelectorSegment::Index(3),
                        SelectorSegment::Alternative("ready".into()),
                    ],
                },
            })),
        ),
    ] {
        ir.declarations.push(ExpressionDeclaration {
            identity: symbol("m", name),
            public: true,
            signature,
            body,
        });
        ir.source_maps.push(ProjectSourceMap {
            declaration: symbol("m", name),
            location: loc("m", start, start + 20),
        });
    }
    for (from, start) in [("operation", 61), ("restricted", 81)] {
        ir.provenance.push(ProjectProvenance {
            from: symbol("m", from),
            to: symbol("m", "Restricted"),
            kind: K::Type,
            location: loc("m", start, start + 1),
        });
    }
    scalar(&mut ir, "m", "unrelated", 150);
    add_optional_output_contract(&mut ir);
    ir
}

/// Captures the optional field, list and alternative contracts used by retained output selectors.
fn add_optional_output_contract(ir: &mut ExpressionProjectIr) {
    let digest = VocabularyContentDigest::from_bytes(b"optional-output-contract");
    ir.vocabularies.push(CompositionBundle {
        identity: VocabularyIdentity::new(
            "Domain",
            "1.0.0",
            "2.0",
            "1.0",
            digest,
            vec!["vocabulary-composition-v2".into()],
        ),
        dependencies: vec![],
        definitions: vec![
            CompositionDefinition {
                name: "Bucket".into(),
                public: true,
                body: CompositionBody::Record(vec![CompositionField {
                    name: "items".into(),
                    ty: T::List(Box::new(T::VocabularyNominal {
                        identity: "Domain".into(),
                        version: "1.0.0".into(),
                        name: "State".into(),
                    })),
                    presence: FieldPresence::Optional,
                    restrictions: FieldRestrictions::default(),
                    default: None,
                }]),
            },
            CompositionDefinition {
                name: "State".into(),
                public: true,
                body: CompositionBody::Variant(vec![CompositionAlternative {
                    tag: "ready".into(),
                    ty: T::Bool,
                }]),
            },
        ],
    });
    ir.vocabulary_sources
        .push(neutral_ir::project::ProjectVocabularySource {
            identity: "Domain".into(),
            version: "1.0.0".into(),
            digest,
            byte_len: 200,
        });
}

/// Selected public views keep real unresolved obligations and exclude unrelated owners' checks.
#[test]
fn public_views_filter_actual_availability_and_restriction_obligations_by_owner() {
    let project = authority(obligation_fixture());
    let cancel = CancellationToken::new();
    let roots = [symbol("m", "output"), symbol("m", "restricted")];
    let view = project
        .derive_view(
            &ExpressionViewRequest {
                schema: "neutral.project-view/3",
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            &cancel,
        )
        .unwrap();
    let availability = view.availability_obligations().collect::<Vec<_>>();
    assert_eq!(availability.len(), 3);
    for (obligation, (segment, requirement)) in availability.iter().zip([
        (0, AvailabilityRequirement::Presence),
        (1, AvailabilityRequirement::Index),
        (2, AvailabilityRequirement::Alternative),
    ]) {
        assert_eq!(obligation.site.owner, symbol("m", "output"));
        assert_eq!(obligation.site.root, ExpressionRoot::Binding);
        assert_eq!(obligation.site.node_path, []);
        assert_eq!(obligation.segment, segment);
        assert_eq!(obligation.requirement, requirement);
    }
    let restrictions = view.restriction_obligations().collect::<Vec<_>>();
    assert_eq!(restrictions.len(), 1);
    assert_eq!(restrictions[0].site.owner, symbol("m", "restricted"));
    assert_eq!(restrictions[0].site.root, ExpressionRoot::Binding);
    assert_eq!(restrictions[0].site.node_path, [0]);
    assert_eq!(
        restrictions[0].contract,
        T::Nominal(symbol("m", "Restricted"))
    );
    assert_eq!(restrictions[0].field, "flag");
    let unrelated = [symbol("m", "unrelated")];
    let view = project
        .derive_view(
            &ExpressionViewRequest {
                schema: "neutral.project-view/3",
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &unrelated,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(view.availability_obligations().count(), 0);
    assert_eq!(view.restriction_obligations().count(), 0);
}
