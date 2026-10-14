// SPDX-License-Identifier: Apache-2.0

//! Companion authority tests use complete real raw projects and the independent semantic model.

use super::*;
use neutral_core::{
    ByteSpan, SemanticDigest, SourceContentDigest, StructuralLimits, allocation::Shared,
};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity,
    composition::{
        CompositionBody, CompositionField, FieldPresence, FieldRestrictions, ValueOriginKind,
        ValuePathSegment,
        project::{CompositionAttribution, CompositionOrigin, CompositionResourceFacts},
    },
    expression::{
        ExpressionNode,
        attribution::{ExpansionAttribution, ExpressionAttribution},
        profile::{ExpressionLimits, SemanticProfiles},
        project::{ExpressionDeclaration, ExpressionResourceFacts},
        templates::{ExpansionLimits, ParameterPhase, PublicTemplate, TemplateParameter},
    },
    project::{ProjectLimits, ProjectModule, ProjectProvenance, ProjectSource, ProjectSourceMap},
    project_interface::{ProjectPublicEdgeKind as K, ProjectPublicType as T},
};
use neutral_semantics::{SemanticLimits, derive_expression_resources, validate_expression_model};

#[path = "consumer_coverage.rs"]
mod consumer_coverage;

/// Gives each source a deterministic original digest without using a host path.
fn source(module: &str) -> ProjectSource {
    ProjectSource {
        module: module.into(),
        source_id: format!("source:{module}"),
        digest: SourceContentDigest::from_bytes(module.as_bytes()),
        byte_len: 200,
    }
}
/// Creates stable module and declaration ownership.
fn symbol(module: &str, name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", module), name)
}
/// Creates checked original coordinates in the fixture's source.
fn loc(module: &str, start: u64, end: u64) -> SourceLocation {
    SourceLocation::new(source(module).digest, ByteSpan::new(start, end).unwrap())
}
/// Uses independent positive controls for all retained partitions.
fn limits() -> SemanticLimits {
    SemanticLimits {
        project: ProjectLimits {
            modules: 1000,
            declarations: 1000,
            import_edges: 1000,
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
/// Supplies a normalized exact numeric value.
fn one() -> V {
    V::Number(ExactNumber::from_normalized_parts(false, "1", 0, 4096, 4096).unwrap())
}
/// Starts a complete source-only successor project with recomputable facts initially unset.
fn empty() -> ExpressionProjectIr {
    let limits = limits();
    ExpressionProjectIr {
        schema: neutral_ir::expression::profile::PROJECT_IR_SCHEMA.into(),
        modules: vec![ProjectModule {
            identity: LogicalModuleIdentity::new("1.0", "m"),
            imports: vec![],
        }],
        declarations: vec![],
        vocabularies: vec![],
        interface_digest: SemanticDigest::from_raw_bytes([0; 32]),
        sources: vec![source("m")],
        source_maps: vec![],
        provenance: vec![],
        limits: limits.project,
        resources: ProjectResourceFacts {
            source_units: 0,
            source_bytes: 0,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: 0,
            import_edges: 0,
            value_nodes: 0,
        },
        vocabulary_sources: vec![],
        origins: vec![],
        composition_limits: limits.composition.policy(),
        composition_resources: CompositionResourceFacts::from_values([0; 7]),
        semantic_profiles: SemanticProfiles::try_frozen().unwrap(),
        public_templates: vec![],
        expression_limits: limits.expression,
        expansion_limits: limits.expansion,
        expression_resources: ExpressionResourceFacts::default(),
        expression_attributions: vec![],
        expansion_attributions: vec![],
    }
}
/// Retains the actual root occurrence for a scalar/materialized binding.
fn origin(owner: &ModuleSymbolIdentity) -> CompositionOrigin {
    CompositionOrigin {
        binding: owner.clone(),
        path: vec![],
        kind: ValueOriginKind::Supplied,
        attribution: None,
    }
}
/// Adds one scalar immutable declaration, its original map and exact root occurrence.
fn scalar(ir: &mut ExpressionProjectIr, module: &str, name: &str, start: u64) {
    let owner = symbol(module, name);
    ir.declarations.push(ExpressionDeclaration {
        identity: owner.clone(),
        public: true,
        signature: S::Binding(T::Num),
        body: Some(ExpressionBody::Value(one())),
    });
    ir.source_maps.push(ProjectSourceMap {
        declaration: owner.clone(),
        location: loc(module, start, start + 10),
    });
    ir.origins.push(origin(&owner));
}
/// Adds source-owned immutable reuse evidence at an actual consuming declaration occurrence.
fn reuse(ir: &mut ExpressionProjectIr, from: &str, to: &str, start: u64) {
    ir.provenance.push(ProjectProvenance {
        from: symbol("m", from),
        to: symbol("m", to),
        kind: K::Value,
        location: loc("m", start, start + 1),
    });
}
/// Canonicalizes producer collections and derives their facts before the real companion check.
fn prepare(mut ir: ExpressionProjectIr) -> ExpressionProjectIr {
    let cancel = CancellationToken::new();
    ir.modules.sort_by(|a, b| a.identity.cmp(&b.identity));
    ir.sources.sort_by(|a, b| a.module.cmp(&b.module));
    ir.declarations.sort_by(|a, b| a.identity.cmp(&b.identity));
    ir.source_maps
        .sort_by(|a, b| a.declaration.cmp(&b.declaration));
    ir.origins
        .sort_by(|a, b| (&a.binding, &a.path).cmp(&(&b.binding, &b.path)));
    ir.provenance.sort_by(|a, b| {
        (&a.from, a.kind, &a.to, a.location).cmp(&(&b.from, b.kind, &b.to, b.location))
    });
    ir.composition_resources =
        inspect_expression_project_resources(&ir, ir.limits, ir.composition_limits, &cancel)
            .unwrap();
    ir.resources = ProjectResourceFacts {
        source_units: ir.sources.len() as u64,
        source_bytes: ir.sources.iter().map(|s| s.byte_len).sum(),
        vocabulary_units: 0,
        vocabulary_bytes: 0,
        declarations: ir.declarations.len() as u64,
        import_edges: ir.modules.iter().map(|m| m.imports.len() as u64).sum(),
        value_nodes: ir.composition_resources.retained_value_nodes,
    };
    ir.expression_resources = derive_expression_resources(&ir, limits(), &cancel).unwrap();
    ir
}

/// Exercises companion checks with independently recomputed complete producer facts.
fn check(ir: ExpressionProjectIr) -> Result<(), E> {
    let ir = prepare(ir);
    let cancel = CancellationToken::new();
    let model = validate_expression_model(Shared::try_new(ir).unwrap(), limits(), &cancel).unwrap();
    validate(&model, &cancel)
}

/// Interface transcripts are artifact metadata, not evaluated UTF-8 result output.
#[test]
fn complete_authority_keeps_payload_and_artifact_limits_independent() {
    let mut ir = empty();
    scalar(&mut ir, "m", "a", 10);
    ir.declarations[0].signature = S::Binding(T::String);
    ir.declarations[0].body = Some(ExpressionBody::Value(V::String("éa".into())));
    ir.expression_limits.output_bytes = "éa".len() as u64;
    let mut ir = prepare(ir);
    let cancel = CancellationToken::new();
    ir.interface_digest = SemanticDigest::from_raw_bytes(
        neutral_ir::project_identity::expression_interface(
            &ir,
            neutral_ir::project_identity::IdentityLimits {
                bytes: ir.limits.artifact_bytes,
                nodes: ir.limits.nodes,
            },
            &cancel,
        )
        .unwrap()
        .identity()
        .as_bytes(),
    );
    assert!(
        super::super::ValidatedExpressionProject::from_ir(
            Shared::try_new(ir).unwrap(),
            limits(),
            &cancel,
        )
        .is_ok()
    );
}

/// Constructs complete independent reader authority without invoking compiler or decoder code.
fn complete(ir: ExpressionProjectIr) -> Shared<ExpressionProjectIr> {
    let mut ir = prepare(ir);
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
    Shared::try_new(ir).unwrap()
}

/// Every reader reservation fails atomically and leaves the shared successful generation reusable.
#[test]
fn independent_complete_reader_faults_cancel_and_recover() {
    use super::super::ValidatedExpressionProject;
    use neutral_core::allocation::testing;
    for ir in [default_fixture(loc("m", 20, 21)), expanded()] {
        let ir = complete(ir);
        let read = |cancel: &CancellationToken| {
            ValidatedExpressionProject::from_ir(ir.clone(), limits(), cancel)
        };
        let (accepted, count) = testing::observe(None, || read(&CancellationToken::new()));
        let accepted = accepted.unwrap();
        for index in 0..count {
            let (failed, _) = testing::observe(Some(index), || read(&CancellationToken::new()));
            assert_eq!(failed.unwrap_err(), E::Limit, "allocation {index}");
            let signal = CancellationToken::new();
            let (failed, _) = testing::observe_cancellation(index, &signal, || read(&signal));
            assert_eq!(failed.unwrap_err(), E::Cancelled, "cancellation {index}");
            assert_eq!(accepted.ir().interface_digest, ir.interface_digest);
            assert!(read(&CancellationToken::new()).is_ok());
        }
    }
}

/// Catches accepting only one direct reuse hop or trusting an unconnected source span.
#[test]
fn source_origin_requires_the_real_transitive_reuse_closure() {
    let mut ir = empty();
    scalar(&mut ir, "m", "a", 10);
    scalar(&mut ir, "m", "b", 30);
    scalar(&mut ir, "m", "c", 50);
    reuse(&mut ir, "a", "b", 11);
    reuse(&mut ir, "b", "c", 31);
    ir.origins[0].attribution = Some(CompositionAttribution::Source(loc("m", 51, 52)));
    assert_eq!(check(ir), Ok(()));
    let mut ir = empty();
    scalar(&mut ir, "m", "a", 10);
    scalar(&mut ir, "m", "b", 30);
    ir.origins[0].attribution = Some(CompositionAttribution::Source(loc("m", 31, 32)));
    assert_eq!(check(ir), Err(E::Companion));
}

/// Creates a source-owned record field whose closed default introduces the occurrence.
fn default_fixture(attribution: SourceLocation) -> ExpressionProjectIr {
    let mut ir = empty();
    let owner = symbol("m", "Record");
    ir.declarations.push(ExpressionDeclaration {
        identity: owner.clone(),
        public: true,
        signature: S::Definition(CompositionBody::Record(vec![CompositionField {
            name: "x".into(),
            ty: T::Num,
            presence: FieldPresence::Defaulted,
            restrictions: FieldRestrictions::default(),
            default: Some(neutral_ir::composition::ClosedValue::Number(
                ExactNumber::from_normalized_parts(false, "1", 0, 4096, 4096).unwrap(),
            )),
        }])),
        body: None,
    });
    ir.source_maps.push(ProjectSourceMap {
        declaration: owner.clone(),
        location: loc("m", 10, 40),
    });
    let binding = symbol("m", "a");
    ir.declarations.push(ExpressionDeclaration {
        identity: binding.clone(),
        public: true,
        signature: S::Binding(T::Nominal(owner.clone())),
        body: Some(ExpressionBody::Value(V::Record(vec![(
            "x".into(),
            Some(one()),
        )]))),
    });
    ir.source_maps.push(ProjectSourceMap {
        declaration: binding.clone(),
        location: loc("m", 50, 70),
    });
    ir.provenance.push(ProjectProvenance {
        from: binding.clone(),
        to: owner,
        kind: K::Type,
        location: loc("m", 51, 52),
    });
    ir.origins.extend([
        origin(&binding),
        CompositionOrigin {
            binding,
            path: vec![ValuePathSegment::Field("x".into())],
            kind: ValueOriginKind::Defaulted,
            attribution: Some(CompositionAttribution::Source(attribution)),
        },
    ]);
    ir
}
/// Catches laundering a source default through a consuming binding's unrelated original bytes.
#[test]
fn default_source_attribution_belongs_to_the_introducing_nominal_contract() {
    assert_eq!(check(default_fixture(loc("m", 20, 21))), Ok(()));
    assert_eq!(check(default_fixture(loc("m", 60, 61))), Err(E::Companion));
}

/// Equal record payloads do not prove reuse across distinct nominal contracts.
#[test]
fn reuse_requires_the_same_typed_occurrence() {
    for distinct in [false, true] {
        let mut ir = default_fixture(loc("m", 20, 21));
        let definition = symbol("m", if distinct { "Other" } else { "Record" });
        if distinct {
            ir.declarations.push(ExpressionDeclaration {
                identity: definition.clone(),
                public: true,
                signature: ir.declarations[0].signature.clone(),
                body: None,
            });
            ir.source_maps.push(ProjectSourceMap {
                declaration: definition.clone(),
                location: loc("m", 80, 100),
            });
        }
        let owner = symbol("m", "b");
        ir.declarations.push(ExpressionDeclaration {
            identity: owner.clone(),
            public: true,
            signature: S::Binding(T::Nominal(definition.clone())),
            body: Some(ExpressionBody::Value(V::Record(vec![(
                "x".into(),
                Some(one()),
            )]))),
        });
        ir.source_maps.push(ProjectSourceMap {
            declaration: owner.clone(),
            location: loc("m", 110, 130),
        });
        ir.origins.extend([
            origin(&owner),
            CompositionOrigin {
                binding: owner.clone(),
                path: vec![ValuePathSegment::Field("x".into())],
                kind: ValueOriginKind::Supplied,
                attribution: None,
            },
        ]);
        ir.provenance.push(ProjectProvenance {
            from: owner,
            to: definition,
            kind: K::Type,
            location: loc("m", 111, 112),
        });
        reuse(&mut ir, "a", "b", 51);
        assert_eq!(check(ir), if distinct { Err(E::Semantic) } else { Ok(()) });
    }
}

/// Creates a public retained template and an expanded residual with original definition lineage.
fn expanded() -> ExpressionProjectIr {
    let mut ir = empty();
    let template = symbol("m", "Identity");
    ir.public_templates.push(PublicTemplate {
        identity: template.clone(),
        parameters: vec![TemplateParameter {
            name: "value".into(),
            phase: ParameterPhase::Static,
            ty: T::Num,
            default: None,
        }],
        result_type: T::Num,
        body: Expression {
            ty: T::Num,
            node: ExpressionNode::Bound { depth: 0, slot: 0 },
        },
    });
    ir.source_maps.push(ProjectSourceMap {
        declaration: template,
        location: loc("m", 10, 50),
    });
    let owner = symbol("m", "result");
    ir.declarations.push(ExpressionDeclaration {
        identity: owner.clone(),
        public: true,
        signature: S::Binding(T::Num),
        body: Some(ExpressionBody::Expression(Expression {
            ty: T::Num,
            node: ExpressionNode::Value(one()),
        })),
    });
    ir.source_maps.push(ProjectSourceMap {
        declaration: owner.clone(),
        location: loc("m", 60, 90),
    });
    ir.expansion_attributions.push(ExpansionAttribution {
        owner: owner.clone(),
        application: loc("m", 60, 90),
        definition: loc("m", 10, 50),
        arguments: vec![loc("m", 80, 81)],
        original_axis_indices: vec![],
    });
    ir.expression_attributions.push(ExpressionAttribution {
        owner,
        node_path: vec![],
        occurrence: Some(loc("m", 39, 40)),
        related_public_locations: vec![],
    });
    ir
}
/// Catches rejecting real definition-scoped bytes or accepting an unrelated private mapped origin.
#[test]
fn expanded_tree_locations_use_checked_lineage_and_public_related_owners() {
    assert_eq!(check(expanded()), Ok(()));
    let mut ir = expanded();
    scalar(&mut ir, "m", "private", 100);
    ir.declarations.last_mut().unwrap().public = false;
    ir.expression_attributions[0]
        .related_public_locations
        .push(loc("m", 101, 102));
    assert_eq!(check(ir), Err(E::Companion));
}
/// Catches forged argument count, unrelated argument bytes and out-of-policy original axis indices.
#[test]
fn expansion_lineage_checks_retained_template_parameters_and_axis_controls() {
    for fault in 0..3 {
        let mut ir = expanded();
        match fault {
            0 => ir.expansion_attributions[0].arguments.clear(),
            1 => ir.expansion_attributions[0].arguments[0] = loc("m", 1, 2),
            _ => ir.expansion_attributions[0]
                .original_axis_indices
                .push(ir.expansion_limits.axis_items),
        }
        assert!(check(ir).is_err());
    }
}

/// Catches interpreting a guarded external leaf as a structural Value edge or ignoring imports.
#[test]
fn cross_unit_input_leaves_need_imports_without_becoming_structural_reuse() {
    for connected in [true, false] {
        let mut ir = empty();
        ir.modules[0].imports = if connected { vec!["n".into()] } else { vec![] };
        ir.modules.push(ProjectModule {
            identity: LogicalModuleIdentity::new("1.0", "n"),
            imports: vec![],
        });
        ir.sources.push(source("n"));
        let input = symbol("n", "input_value");
        ir.declarations.push(ExpressionDeclaration {
            identity: input.clone(),
            public: true,
            signature: S::Binding(T::Num),
            body: Some(ExpressionBody::Input),
        });
        ir.source_maps.push(ProjectSourceMap {
            declaration: input.clone(),
            location: loc("n", 10, 20),
        });
        let owner = symbol("m", "result");
        ir.declarations.push(ExpressionDeclaration {
            identity: owner.clone(),
            public: true,
            signature: S::Binding(T::Num),
            body: Some(ExpressionBody::Expression(Expression {
                ty: T::Num,
                node: ExpressionNode::Input(input),
            })),
        });
        ir.source_maps.push(ProjectSourceMap {
            declaration: owner,
            location: loc("m", 10, 20),
        });
        assert_eq!(check(ir), if connected { Ok(()) } else { Err(E::Semantic) });
    }
}
