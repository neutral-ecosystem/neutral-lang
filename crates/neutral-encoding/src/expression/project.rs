// SPDX-License-Identifier: Apache-2.0

//! Closed 22-position reconstruction; `/3` bodies never become fabricated `/2` values.

use super::{
    check_cancelled, companions, read_list,
    trees::{read_tree, write_tree},
};
use crate::{
    DecodeError as D, DecodeErrorClass as C, EncodingError as E,
    cbor::CborWriter,
    contracts::{
        read_body, read_companions, read_numbers, read_option, read_origin, read_value,
        read_vocabularies, write_body, write_companions, write_option, write_origin, write_value,
    },
    decoder::LocatedValue,
    primitives::{
        array, boolean, digest, failure, owned_text, read_module, read_symbol, read_type, schema,
        text, tuple, write_list, write_module, write_numbers, write_symbol, write_type,
    },
};
use neutral_core::{CancellationToken, SemanticDigest};
use neutral_ir::{
    composition::project::{
        CompositionPolicy, CompositionResourceFacts, CompositionSignature as S,
    },
    expression::{
        ExpressionBody as B,
        profile::{self, ExpressionLimits, SemanticProfiles},
        project::{ExpressionDeclaration, ExpressionProjectIr, ExpressionResourceFacts},
        templates::{ExpansionLimits, ParameterPhase, PublicTemplate, TemplateParameter},
    },
    project::ProjectModule,
};

/// Writes all frozen sections without changing their distinct meanings.
pub(super) fn write_ir(w: &mut CborWriter, ir: &ExpressionProjectIr) -> Result<(), E> {
    w.array(22)?;
    w.text(&ir.schema)?;
    write_list(w, &ir.modules, |w, module| {
        w.array(2)?;
        write_module(w, &module.identity)?;
        write_list(w, &module.imports, |w, import| w.text(import))
    })?;
    write_list(w, &ir.declarations, write_declaration)?;
    let definitions = ir.vocabularies.iter().try_fold(0_usize, |count, bundle| {
        count
            .checked_add(bundle.definitions.len())
            .ok_or(E::EncodedSizeLimit)
    })?;
    w.array(definitions)?;
    for bundle in &ir.vocabularies {
        for definition in &bundle.definitions {
            w.array(5)?;
            w.text(bundle.identity.identity())?;
            w.text(bundle.identity.version())?;
            w.text(&definition.name)?;
            w.boolean(definition.public)?;
            write_body(w, &definition.body)?;
        }
    }
    write_list(w, &ir.vocabularies, |w, bundle| {
        w.array(5)?;
        w.text(bundle.identity.identity())?;
        w.text(bundle.identity.version())?;
        w.text(bundle.identity.schema_version())?;
        write_list(w, bundle.identity.required_features(), |w, feature| {
            w.text(feature)
        })?;
        w.array(
            bundle
                .definitions
                .iter()
                .filter(|definition| definition.public)
                .count(),
        )?;
        for definition in bundle
            .definitions
            .iter()
            .filter(|definition| definition.public)
        {
            w.text(&definition.name)?;
        }
        Ok(())
    })?;
    w.bytes(&ir.interface_digest.as_bytes())?;
    write_companions(
        w,
        &ir.sources,
        &ir.source_maps,
        &ir.provenance,
        ir.limits,
        ir.resources,
        &ir.vocabulary_sources,
    )?;
    write_list(w, &ir.vocabularies, |w, bundle| {
        w.array(3)?;
        w.text(bundle.identity.identity())?;
        w.text(bundle.identity.version())?;
        write_list(w, &bundle.dependencies, |w, dependency| {
            w.array(2)?;
            w.text(&dependency.identity)?;
            w.text(&dependency.version)
        })
    })?;
    write_list(w, &ir.origins, write_origin)?;
    write_numbers(w, &ir.composition_limits.values())?;
    write_numbers(w, &ir.composition_resources.values())?;
    w.array(5)?;
    for profile in [
        &ir.semantic_profiles.expression,
        &ir.semantic_profiles.evaluator,
        &ir.semantic_profiles.expansion,
        &ir.semantic_profiles.decimal,
    ] {
        w.text(profile)?;
    }
    write_list(w, &ir.semantic_profiles.required_features, |w, feature| {
        w.text(feature)
    })?;
    write_list(w, &ir.public_templates, write_template)?;
    write_numbers(w, &ir.expression_limits.values())?;
    write_numbers(w, &ir.expansion_limits.values())?;
    write_numbers(w, &ir.expression_resources.values())?;
    companions::write_attributions(w, ir)
}

/// Retains named parameters in declaration order and the definition-scoped trees.
fn write_template(w: &mut CborWriter, template: &PublicTemplate) -> Result<(), E> {
    w.array(4)?;
    write_symbol(w, &template.identity)?;
    write_list(w, &template.parameters, |w, parameter| {
        w.array(4)?;
        w.text(&parameter.name)?;
        w.text(match parameter.phase {
            ParameterPhase::Static => "static",
            ParameterPhase::Deferred => "deferred",
        })?;
        write_type(w, &parameter.ty, 0)?;
        write_option(w, parameter.default.as_ref(), |w, tree| {
            write_tree(w, tree, 1)
        })
    })?;
    write_type(w, &template.result_type, 0)?;
    write_tree(w, &template.body, 1)
}

/// Restores complete raw data after exact schema and semantic-profile selection.
pub(super) fn read_ir(
    value: &LocatedValue,
    cancel: &CancellationToken,
) -> Result<ExpressionProjectIr, D> {
    check_cancelled(cancel)?;
    let p = tuple::<22>(value)?;
    if text(&p[0])? != profile::PROJECT_IR_SCHEMA {
        return Err(failure(C::UnsupportedVersion));
    }
    let profiles = tuple::<5>(&p[16])?;
    let semantic_profiles = SemanticProfiles {
        expression: owned_text(&profiles[0])?,
        evaluator: owned_text(&profiles[1])?,
        expansion: owned_text(&profiles[2])?,
        decimal: owned_text(&profiles[3])?,
        required_features: read_list(&profiles[4], cancel, owned_text)?,
    };
    if !semantic_profiles.matches_frozen() {
        return Err(failure(C::UnsupportedVersion));
    }
    let expression = read_numbers::<11>(&p[18])?;
    let expansion = read_numbers::<13>(&p[19])?;
    if expression.contains(&0)
        || expansion.contains(&0)
        || read_numbers::<6>(&p[9])?.contains(&0)
        || read_numbers::<15>(&p[14])?.contains(&0)
    {
        return Err(failure(C::EncodedSizeLimit));
    }
    let companions = read_companions(p, cancel)?;
    let resources = read_numbers::<7>(&p[20])?;
    let attributions = tuple::<2>(&p[21])?;
    let ir = ExpressionProjectIr {
        schema: owned_text(&p[0])?,
        modules: read_list(&p[1], cancel, |v| {
            let a = tuple::<2>(v)?;
            Ok(ProjectModule {
                identity: read_module(&a[0])?,
                imports: read_list(&a[1], cancel, owned_text)?,
            })
        })?,
        declarations: read_list(&p[2], cancel, |v| read_declaration(v, cancel))?,
        vocabularies: read_vocabularies(p, &companions.vocabularies)?,
        interface_digest: SemanticDigest::from_raw_bytes(digest(&p[5])?),
        sources: companions.sources,
        source_maps: companions.maps,
        provenance: companions.provenance,
        limits: companions.limits,
        resources: companions.resources,
        vocabulary_sources: companions.vocabularies,
        origins: read_list(&p[13], cancel, read_origin)?,
        composition_limits: CompositionPolicy::from_values(read_numbers(&p[14])?),
        composition_resources: CompositionResourceFacts::from_values(read_numbers(&p[15])?),
        semantic_profiles,
        public_templates: read_list(&p[17], cancel, |v| read_template(v, cancel))?,
        expression_limits: ExpressionLimits {
            nodes: expression[0],
            syntax_depth: expression[1],
            evaluation_depth: expression[2],
            bindings: expression[3],
            dependency_edges: expression[4],
            work: expression[5],
            retained_bytes: expression[6],
            output_bytes: expression[7],
            coefficient_digits: expression[8],
            absolute_scale: expression[9],
            diagnostics: expression[10],
        },
        expansion_limits: ExpansionLimits {
            definitions: expansion[0],
            applications: expansion[1],
            parameters: expansion[2],
            axes: expansion[3],
            axis_items: expansion[4],
            product_tuples: expansion[5],
            aggregate_tuples: expansion[6],
            expanded_bindings: expansion[7],
            expanded_nodes: expansion[8],
            depth: expansion[9],
            work: expansion[10],
            retained_bytes: expansion[11],
            output_bytes: expansion[12],
        },
        expression_resources: ExpressionResourceFacts {
            residual_nodes: resources[0],
            residual_depth: resources[1],
            input_declarations: resources[2],
            output_leaves: resources[3],
            potential_value_edges: resources[4],
            public_templates: resources[5],
            retained_expression_text_bytes: resources[6],
        },
        expression_attributions: companions::read_expressions(&attributions[0], cancel)?,
        expansion_attributions: companions::read_expansions(&attributions[1], cancel)?,
    };
    check_cancelled(cancel)?;
    Ok(ir)
}

/// Restores a non-callable public contract including nullable parameter defaults.
fn read_template(value: &LocatedValue, cancel: &CancellationToken) -> Result<PublicTemplate, D> {
    let a = tuple::<4>(value)?;
    Ok(PublicTemplate {
        identity: read_symbol(&a[0])?,
        parameters: read_list(&a[1], cancel, |v| {
            let a = tuple::<4>(v)?;
            Ok(TemplateParameter {
                name: owned_text(&a[0])?,
                phase: match text(&a[1])? {
                    "static" => ParameterPhase::Static,
                    "deferred" => ParameterPhase::Deferred,
                    _ => return Err(schema(v)),
                },
                ty: read_type(&a[2], 0)?,
                default: read_option(&a[3], |v| read_tree(v, 1, cancel))?,
            })
        })?,
        result_type: read_type(&a[2], 0)?,
        body: read_tree(&a[3], 1, cancel)?,
    })
}

/// Writes definition absence or the exact constant/input/expression body sum.
fn write_declaration(w: &mut CborWriter, declaration: &ExpressionDeclaration) -> Result<(), E> {
    w.array(4)?;
    write_symbol(w, &declaration.identity)?;
    w.boolean(declaration.public)?;
    match &declaration.signature {
        S::Binding(ty) => {
            w.array(2)?;
            w.text("binding")?;
            write_type(w, ty, 0)?;
        }
        S::Definition(body) => write_body(w, body)?,
    }
    write_option(w, declaration.body.as_ref(), |w, body| match body {
        B::Value(value) => {
            w.array(2)?;
            w.text("value")?;
            write_value(w, value, 0)
        }
        B::Input => {
            w.array(1)?;
            w.text("input")
        }
        B::Expression(expression) => {
            w.array(2)?;
            w.text("expression")?;
            write_tree(w, expression, 1)
        }
    })
}

/// Reads one exact declaration shape without fabricating binding values.
fn read_declaration(
    v: &LocatedValue,
    cancel: &CancellationToken,
) -> Result<ExpressionDeclaration, D> {
    let a = tuple::<4>(v)?;
    let signature = tuple::<2>(&a[2])?;
    Ok(ExpressionDeclaration {
        identity: read_symbol(&a[0])?,
        public: boolean(&a[1])?,
        signature: if text(&signature[0])? == "binding" {
            S::Binding(read_type(&signature[1], 0)?)
        } else {
            S::Definition(read_body(&a[2])?)
        },
        body: read_option(&a[3], |v| {
            let a = array(v)?;
            match a {
                [tag] if text(tag)? == "input" => Ok(B::Input),
                [tag, value] if text(tag)? == "value" => Ok(B::Value(read_value(value, 0)?)),
                [tag, tree] if text(tag)? == "expression" => {
                    Ok(B::Expression(read_tree(tree, 1, cancel)?))
                }
                _ => Err(schema(v)),
            }
        })?,
    })
}
