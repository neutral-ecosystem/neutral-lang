// SPDX-License-Identifier: Apache-2.0

//! Complete typed /3 logical framing; private bodies are omitted only in interface projection.

use super::{LOGICAL_DOMAIN, features, semantic_profiles, trees};
use crate::{
    ModuleSymbolIdentity,
    composition::{
        CompositionBody as B, CompositionValue as V, project::CompositionSignature as S,
    },
    expression::{ExpressionBody, profile, project::ExpressionProjectIr},
    project_identity::{
        ExpressionLogicalIdentity, IdentityError as E, IdentityLimits, IdentityTranscript,
        composition::{self, type_transcript},
        framing::{Writer, ordered},
        logical::symbol,
        transcript_profile,
    },
    project_interface::ProjectPublicEdgeKind,
};
use neutral_core::{
    CancellationToken, SemanticDigest, allocation::RetainCapacity, profile::V1_SOURCE_PROFILE,
};

/// Frames complete /3 meaning, including disconnected declarations and exported templates.
///
/// # Errors
/// Rejects noncanonical/schema/profile input, independent framing/depth bounds or cancellation.
/// This is structural identity framing; untrusted project semantics require the independent reader.
pub fn canonical_expression_project(
    ir: &ExpressionProjectIr,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<ExpressionLogicalIdentity>, E> {
    build(
        ir,
        limits,
        cancellation,
        false,
        LOGICAL_DOMAIN,
        ExpressionLogicalIdentity,
    )
}

/// Retains the complete structural grammar for both explicit /3 logical projections.
pub(super) fn build<I>(
    ir: &ExpressionProjectIr,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
    public: bool,
    domain: &str,
    wrap: impl FnOnce(SemanticDigest) -> I,
) -> Result<IdentityTranscript<I>, E> {
    transcript_profile(
        domain,
        profile::IDENTITY_PROFILE,
        limits,
        cancellation,
        |w| {
            if ir.schema != profile::PROJECT_IR_SCHEMA
                || ir.modules.is_empty()
                || !ir.semantic_profiles.matches_frozen()
            {
                return Err(E::InvalidInput);
            }
            preflight(w, ir)?;
            w.leaf("schema", ir.schema.as_bytes())?;
            w.leaf("profile", V1_SOURCE_PROFILE.as_bytes())?;
            composition::modules(w, &ir.modules)?;
            w.frame("declarations", |w| {
                for declaration in ir.declarations.iter().filter(|d| !public || d.public) {
                    declaration
                        .validate_body_shape()
                        .map_err(|_| E::InvalidInput)?;
                    w.frame("declaration", |w| {
                        symbol(w, &declaration.identity)?;
                        w.leaf("public", &[u8::from(declaration.public)])?;
                        w.frame("signature", |w| match &declaration.signature {
                            S::Binding(ty) => w.frame("binding", |w| type_transcript(w, ty, 0)),
                            S::Definition(body) => composition::body(w, body),
                        })?;
                        w.frame("value", |w| match &declaration.body {
                            Some(ExpressionBody::Value(value)) => {
                                w.frame("constant", |w| composition::value(w, value, 0))
                            }
                            Some(ExpressionBody::Input) => w.leaf("external-input", &[]),
                            Some(ExpressionBody::Expression(expression)) => {
                                w.frame("expression", |w| trees::tree(w, expression, 1))
                            }
                            None => w.leaf("absent", &[]),
                        })
                    })?;
                }
                Ok(())
            })?;
            composition::vocabulary_types(w, &ir.vocabularies, public)?;
            composition::catalogues(w, &ir.vocabularies)?;
            composition::dependencies(w, &ir.vocabularies)?;
            public_edges(w, ir)?;
            semantic_profiles(w, &ir.semantic_profiles)?;
            features(w, "required-features", &ir.semantic_profiles)?;
            trees::templates(w, &ir.public_templates)
        },
        wrap,
    )
}

/// Bounds all comparison keys before checking canonical order; companions never enter framing.
fn preflight(w: &mut Writer<'_>, ir: &ExpressionProjectIr) -> Result<(), E> {
    for count in [
        ir.modules.len(),
        ir.declarations.len(),
        ir.vocabularies.len(),
        ir.public_templates.len(),
        ir.provenance.len(),
    ] {
        w.items(count)?;
    }
    let mut imports = 0_usize;
    for module in &ir.modules {
        if module.identity.language_behavior_version() != V1_SOURCE_PROFILE {
            return Err(E::InvalidInput);
        }
        w.text(module.identity.language_behavior_version())?;
        w.text(module.identity.module_name())?;
        imports = imports.checked_add(module.imports.len()).ok_or(E::Limit)?;
        w.items(imports)?;
        for import in &module.imports {
            w.text(import)?;
        }
    }
    let mut value_nodes = 0;
    for declaration in &ir.declarations {
        key(w, &declaration.identity)?;
        declaration
            .validate_body_shape()
            .map_err(|_| E::InvalidInput)?;
        if let S::Definition(body) = &declaration.signature {
            body_keys(w, body, &mut value_nodes)?;
        }
        if let Some(ExpressionBody::Value(value)) = &declaration.body {
            value_keys(w, value, 0, &mut value_nodes)?;
        }
    }
    for bundle in &ir.vocabularies {
        w.text(bundle.identity.identity())?;
        w.text(bundle.identity.version())?;
        w.items(bundle.definitions.len())?;
        for definition in &bundle.definitions {
            w.text(&definition.name)?;
            body_keys(w, &definition.body, &mut value_nodes)?;
        }
        ordered(bundle.definitions.iter().map(|d| &d.name))?;
        w.items(bundle.dependencies.len())?;
        for dependency in &bundle.dependencies {
            w.text(&dependency.identity)?;
            w.text(&dependency.version)?;
        }
        ordered(
            bundle
                .dependencies
                .iter()
                .map(|d| (&d.identity, &d.version)),
        )?;
    }
    for template in &ir.public_templates {
        key(w, &template.identity)?;
    }
    ordered(ir.modules.iter().map(|m| &m.identity))?;
    ordered(ir.declarations.iter().map(|d| &d.identity))?;
    ordered(
        ir.vocabularies
            .iter()
            .map(|b| (b.identity.identity(), b.identity.version())),
    )?;
    ordered(ir.public_templates.iter().map(|t| &t.identity))
}

/// Charges a symbol used by canonical ordering before any comparison or binary search.
fn key(w: &mut Writer<'_>, identity: &ModuleSymbolIdentity) -> Result<(), E> {
    if identity.module().language_behavior_version() != V1_SOURCE_PROFILE
        || identity.module().module_name().is_empty()
        || identity.declaration_name().is_empty()
    {
        return Err(E::InvalidInput);
    }
    w.text(identity.module().language_behavior_version())?;
    w.text(identity.module().module_name())?;
    w.text(identity.declaration_name())
}

/// Checks keys used by the unchanged /2 nominal/default/restriction framing helpers.
fn body_keys(w: &mut Writer<'_>, body: &B, nodes: &mut usize) -> Result<(), E> {
    match body {
        B::Record(fields) => {
            w.items(fields.len())?;
            for field in fields {
                w.text(&field.name)?;
                if let Some(choices) = &field.restrictions.choices {
                    w.items(choices.len())?;
                    for choice in choices {
                        value_keys(w, choice, 0, nodes)?;
                    }
                }
                if let Some(default) = &field.default {
                    value_keys(w, default, 0, nodes)?;
                }
            }
        }
        B::Variant(alternatives) => {
            w.items(alternatives.len())?;
            for alternative in alternatives {
                w.text(&alternative.tag)?;
            }
        }
    }
    Ok(())
}

/// Preflights recursive value keys without copying values or imposing producer controls on meaning.
pub(super) fn value_keys<R>(
    w: &mut Writer<'_>,
    value: &V<R>,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), E> {
    w.check()?;
    if depth > crate::project::PROJECT_MAX_DEPTH {
        return Err(E::Limit);
    }
    *nodes = nodes.checked_add(1).ok_or(E::Limit)?;
    w.items(*nodes)?;
    match value {
        V::Number(number) => w.text(number.coefficient())?,
        V::String(text) | V::Url(text) | V::Path(text) => w.text(text)?,
        V::List(items) => {
            w.items(items.len())?;
            for item in items {
                value_keys(w, item, depth + 1, nodes)?;
            }
        }
        V::Record(fields) => {
            w.items(fields.len())?;
            for (name, value) in fields {
                w.text(name)?;
                if let Some(value) = value {
                    value_keys(w, value, depth + 1, nodes)?;
                }
            }
        }
        V::Variant { payload, .. } => value_keys(w, payload, depth + 1, nodes)?,
        V::Null | V::Bool(_) | V::Reference(_) => {}
    }
    Ok(())
}

/// Structural public edges retain /2 set semantics; guarded value edges remain in typed trees.
fn public_edges(w: &mut Writer<'_>, ir: &ExpressionProjectIr) -> Result<(), E> {
    let visible = |owner: &ModuleSymbolIdentity| {
        ir.declarations
            .binary_search_by(|d| d.identity.cmp(owner))
            .is_ok_and(|index| ir.declarations[index].public)
    };
    let mut edges = Vec::new();
    for edge in &ir.provenance {
        w.check()?;
        // Charge supplied lookup keys before the binary searches, including excluded evidence.
        key(w, &edge.from)?;
        key(w, &edge.to)?;
        if visible(&edge.from)
            && visible(&edge.to)
            && matches!(
                edge.kind,
                ProjectPublicEdgeKind::Type
                    | ProjectPublicEdgeKind::ReferenceType
                    | ProjectPublicEdgeKind::Reference
            )
        {
            edges.try_retain(1).map_err(|_| E::Limit)?;
            edges.push((&edge.from, edge.kind, &edge.to));
        }
    }
    edges.sort_unstable();
    edges.dedup();
    w.check()?;
    w.frame("public-edges", |w| {
        for (from, kind, to) in edges {
            w.frame("edge", |w| {
                w.frame("from", |w| symbol(w, from))?;
                w.frame("to", |w| symbol(w, to))?;
                w.leaf("kind", kind.spelling().as_bytes())
            })?;
        }
        Ok(())
    })
}
