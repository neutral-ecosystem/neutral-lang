// SPDX-License-Identifier: Apache-2.0

//! Assemble complete project meaning and its source/provenance companions.
//!
//! Lower resolved declarations, retain captured-source facts and attach checked
//! occurrences and origin evidence. Construction remains request-local and
//! fallible so an error cannot publish a partial artifact. The parent pipeline
//! controls phase order and any cache-generation publication.

use super::{
    Arc, B, CompositionDeclaration, CompositionOrigin, CompositionResourceFacts, E, Edge,
    IdentityLimits, Ir, LogicalModuleIdentity, ProjectLimits, ProjectModule, ProjectProvenance,
    ProjectResourceFacts, ProjectSource, ProjectSourceMap, ProjectVocabularySource, Resolver, S,
    SemanticDigest, TryClone, V1_SOURCE_PROFILE, ValidatedCompositionScope, attribution, codes,
    composition_interface, copy_slice, fail, inspect_composition_ir, profile, reserve, retain,
    text, type_edges,
};

/// Builds complete canonical logical data and separately retained source/resource companions.
pub(super) fn assemble(
    resolver: &mut Resolver<'_>,
    scope: &ValidatedCompositionScope,
    bindings: &neutral_vocabulary::composition::ValidatedCompositionBindings,
) -> Result<Arc<Ir>, E> {
    let captured = resolver.captured;
    let cap = captured.limits().values();
    let LoweredDeclarations {
        declarations,
        maps,
        origins,
    } = declarations(resolver, scope, bindings)?;
    let CapturedCompanions {
        modules,
        sources,
        vocabulary_sources,
    } = captured_companions(resolver)?;
    complete_provenance(resolver, &declarations, bindings)?;
    let limits = ProjectLimits {
        modules: cap.source_units,
        declarations: cap.declarations,
        import_edges: cap.import_edges,
        nodes: captured.composition_limits().value_nodes,
        text_bytes: cap
            .total_source_bytes
            .checked_add(cap.total_vocabulary_bytes)
            .ok_or_else(|| fail(codes::LIMIT, None))?,
        artifact_bytes: cap.output_bytes,
    };
    let facts = captured.resource_facts();
    let mut ir = Ir {
        schema: retain(text(profile::PROJECT_IR_SCHEMA))?,
        modules,
        declarations,
        vocabularies: retain(copy_slice(scope.catalogue().bundles()))?,
        interface_digest: SemanticDigest::from_raw_bytes([0; 32]),
        sources,
        source_maps: maps,
        provenance: std::mem::take(&mut resolver.provenance),
        limits,
        resources: ProjectResourceFacts {
            source_units: facts.source_units(),
            source_bytes: facts.total_source_bytes(),
            vocabulary_units: facts.vocabulary_units(),
            vocabulary_bytes: facts.total_vocabulary_bytes(),
            declarations: resolver.roots.len() as u64,
            import_edges: resolver.graph.edges().len() as u64,
            value_nodes: 0,
        },
        vocabulary_sources,
        origins,
        composition_limits: captured.composition_limits().policy(),
        composition_resources: CompositionResourceFacts::from_values([0; 7]),
    };
    ir.composition_resources =
        inspect_composition_ir(&ir, limits, ir.composition_limits, resolver.cancel).map_err(
            |e| {
                fail(
                    match e {
                        neutral_ir::composition::project::CompositionShapeError::Cancelled => {
                            codes::CANCELLED
                        }
                        neutral_ir::composition::project::CompositionShapeError::Limit => {
                            codes::LIMIT
                        }
                    },
                    None,
                )
            },
        )?;
    ir.resources.value_nodes = ir.composition_resources.retained_value_nodes;
    ir.interface_digest = composition_interface(
        &ir,
        IdentityLimits {
            bytes: neutral_ir::project_identity::MAX_TRANSCRIPT_BYTES,
            nodes: captured.composition_limits().work,
        },
        resolver.cancel,
    )
    .map_err(|e| {
        fail(
            match e {
                neutral_ir::project_identity::IdentityError::Cancelled => codes::CANCELLED,
                _ => codes::LIMIT,
            },
            None,
        )
    })?
    .identity();
    resolver.step(1, None)?;
    Arc::try_new(ir).map_err(|_| fail(codes::LIMIT, None))
}

/// Completes canonical source type and reference edges after all values have been validated.
fn complete_provenance(
    resolver: &mut Resolver<'_>,
    declarations: &[CompositionDeclaration],
    bindings: &neutral_vocabulary::composition::ValidatedCompositionBindings,
) -> Result<(), E> {
    for d in declarations {
        let location = resolver.roots[&d.identity].location;
        match &d.signature {
            S::Binding(t) => type_edges(&d.identity, t, false, location, resolver)?,
            S::Definition(B::Record(fs)) => {
                for f in fs {
                    type_edges(&d.identity, &f.ty, false, location, resolver)?;
                }
            }
            S::Definition(B::Variant(alternatives)) => {
                for a in alternatives {
                    type_edges(&d.identity, &a.ty, false, location, resolver)?;
                }
            }
        }
    }
    for b in bindings.bindings() {
        for r in b.references() {
            resolver.step(1, None)?;
            reserve(&mut resolver.provenance, 1)?;
            resolver.provenance.push(ProjectProvenance {
                from: retain(b.binding().owner.try_clone())?,
                to: retain(r.target.try_clone())?,
                kind: Edge::Reference,
                location: resolver.roots[&b.binding().owner].location,
            });
        }
    }
    resolver.provenance.sort_unstable_by(|a, b| {
        (&a.from, a.kind, &a.to, a.location).cmp(&(&b.from, b.kind, &b.to, b.location))
    });
    resolver.provenance.dedup();
    resolver.step(1, None)
}

/// Canonical source declarations with separate occurrence and coverage companions.
struct LoweredDeclarations {
    /// Complete private/public signatures and values.
    declarations: Vec<CompositionDeclaration>,
    /// Complete original-byte declaration coverage.
    maps: Vec<ProjectSourceMap>,
    /// Non-semantic supplied/default/null/omitted occurrences.
    origins: Vec<CompositionOrigin>,
}
/// Projects already validated complete source contracts and materialized bindings.
fn declarations(
    resolver: &mut Resolver<'_>,
    scope: &ValidatedCompositionScope,
    bindings: &neutral_vocabulary::composition::ValidatedCompositionBindings,
) -> Result<LoweredDeclarations, E> {
    let mut declarations = Vec::new();
    let mut maps = Vec::new();
    let mut origins = Vec::new();
    let roots = resolver.retained_roots()?;
    for root in &roots {
        let (signature, value) = if root.ty.is_some() {
            let b = bindings
                .bindings()
                .iter()
                .find(|b| b.binding().owner == root.owner)
                .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, Some(root.location)))?;
            for o in b.origins() {
                resolver.step(1, Some(root.location))?;
                reserve(&mut origins, 1)?;
                origins.push(CompositionOrigin {
                    binding: retain(root.owner.try_clone())?,
                    path: retain(o.path.try_clone())?,
                    kind: o.kind,
                    attribution: attribution::origin(
                        resolver,
                        root,
                        &b.binding().ty,
                        &o.path,
                        o.kind,
                    )?,
                });
            }
            (
                S::Binding(retain(b.binding().ty.try_clone())?),
                Some(retain(b.binding().value.try_clone())?),
            )
        } else {
            let d = scope
                .sources()
                .iter()
                .find(|d| d.owner == root.owner)
                .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, Some(root.location)))?;
            (S::Definition(retain(d.definition.body.try_clone())?), None)
        };
        resolver.step(1, Some(root.location))?;
        reserve(&mut declarations, 1)?;
        reserve(&mut maps, 1)?;
        declarations.push(CompositionDeclaration {
            identity: retain(root.owner.try_clone())?,
            public: root.public,
            signature,
            value,
        });
        maps.push(ProjectSourceMap {
            declaration: retain(root.owner.try_clone())?,
            location: root.location,
        });
    }
    Ok(LoweredDeclarations {
        declarations,
        maps,
        origins,
    })
}
/// Exact captured accounting and complete logical module topology.
pub(super) struct CapturedCompanions {
    /// All modules, including disconnected and private-only units.
    pub(super) modules: Vec<ProjectModule>,
    /// Logical source identities and byte lengths.
    pub(super) sources: Vec<ProjectSource>,
    /// Exact locked vocabulary capture evidence.
    pub(super) vocabulary_sources: Vec<ProjectVocabularySource>,
}
/// Derives checked captured companions without acquiring any input.
pub(super) fn captured_companions(resolver: &Resolver<'_>) -> Result<CapturedCompanions, E> {
    let captured = resolver.captured;
    let mut modules = Vec::new();
    let mut sources = Vec::new();
    reserve(&mut modules, captured.sources().len())?;
    reserve(&mut sources, captured.sources().len())?;
    for s in captured.sources() {
        if resolver.cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, None));
        }
        let mut imports = Vec::new();
        for e in resolver
            .graph
            .edges()
            .iter()
            .filter(|e| e.from() == s.module_id())
        {
            if resolver.cancel.is_cancelled() {
                return Err(fail(codes::CANCELLED, None));
            }
            reserve(&mut imports, 1)?;
            imports.push(retain(text(e.target()))?);
        }
        imports.sort_unstable();
        modules.push(ProjectModule {
            identity: LogicalModuleIdentity::new(
                retain(text(V1_SOURCE_PROFILE))?,
                retain(text(s.module_id()))?,
            ),
            imports,
        });
        sources.push(ProjectSource {
            module: retain(text(s.module_id()))?,
            source_id: retain(text(s.source_id()))?,
            digest: s.digest(),
            byte_len: s.bytes().len() as u64,
        });
    }
    let mut vocabulary_sources = Vec::new();
    reserve(
        &mut vocabulary_sources,
        captured.catalogue().bundles().len(),
    )?;
    for b in captured.catalogue().bundles() {
        if resolver.cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, None));
        }
        let Some(source) = captured
            .vocabularies()
            .iter()
            .find(|v| v.lock().identity() == b.identity.identity())
        else {
            return Err(fail(codes::INCOMPATIBLE_VALUE, None));
        };
        vocabulary_sources.push(ProjectVocabularySource {
            identity: retain(text(b.identity.identity()))?,
            version: retain(text(b.identity.version()))?,
            digest: b.identity.content_digest(),
            byte_len: source.bytes().len() as u64,
        });
    }
    Ok(CapturedCompanions {
        modules,
        sources,
        vocabulary_sources,
    })
}

#[cfg(test)]
#[path = "../../../../tests/project_cache/assembly.rs"]
mod tests;
