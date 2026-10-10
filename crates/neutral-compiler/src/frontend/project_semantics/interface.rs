// SPDX-License-Identifier: Apache-2.0

//! Project checked exports into the public project-interface model.
//!
//! Convert resolved types and exported signatures into consumer-facing contracts
//! without exposing private binding implementations. Dependency edges preserve
//! the information needed to interpret public types and references. This is an
//! interface projection, not complete-project lowering or logical identity.

use super::{
    BTreeMap, BTreeSet, CoreError, LogicalModuleIdentity, ModuleContext, ModuleSymbolIdentity,
    ProjectDependency, ProjectDependencyKind, ProjectInterface, ProjectPublicEdge,
    ProjectPublicEdgeKind, ProjectPublicExport, ProjectPublicField, ProjectPublicSignature,
    ProjectPublicType, ProjectPublicVocabulary, ProjectSymbolKind, ProjectVocabularySet, Root,
    TypeExpr, V1_SOURCE_PROFILE, key,
};

/// Projects only public signatures and public-to-public identity edges.
pub(super) fn build_public_interface(
    roots: &BTreeMap<(String, String), Root>,
    modules: &BTreeMap<String, ModuleContext>,
    vocabularies: &ProjectVocabularySet,
    edges: &[ProjectDependency],
) -> Result<ProjectInterface, CoreError> {
    let mut exports = Vec::new();
    let public_keys = roots
        .iter()
        .filter(|(_, root)| root.symbol.is_public)
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();
    for (owner, root) in roots {
        if !root.symbol.is_public {
            continue;
        }
        let signature = match root.symbol.kind {
            ProjectSymbolKind::Binding => ProjectPublicSignature::Binding(public_type(
                root.declared_type
                    .as_ref()
                    .expect("bindings have a declared type"),
                &owner.0,
                modules,
                vocabularies,
            )),
            ProjectSymbolKind::Record => {
                let mut fields = root
                    .fields
                    .iter()
                    .map(|(name, ty)| {
                        ProjectPublicField::new(
                            name,
                            public_type(ty, &owner.0, modules, vocabularies),
                        )
                    })
                    .collect::<Vec<_>>();
                fields.sort_by(|left, right| left.name().cmp(right.name()));
                ProjectPublicSignature::Record(fields)
            }
        };
        exports.push(ProjectPublicExport::new(
            root.symbol.identity.clone(),
            signature,
        ));
    }
    let mut public_edges = edges
        .iter()
        .filter(|edge| {
            public_keys.contains(&key(edge.from())) && public_keys.contains(&key(edge.to()))
        })
        .map(|edge| {
            ProjectPublicEdge::new(
                edge.from().clone(),
                edge.to().clone(),
                match edge.kind() {
                    ProjectDependencyKind::Type => ProjectPublicEdgeKind::Type,
                    ProjectDependencyKind::ReferenceType => ProjectPublicEdgeKind::ReferenceType,
                    ProjectDependencyKind::Value => ProjectPublicEdgeKind::Value,
                    ProjectDependencyKind::Reference => ProjectPublicEdgeKind::Reference,
                },
            )
        })
        .collect::<Vec<_>>();
    public_edges.sort_by(|left, right| {
        (left.from(), left.kind(), left.to()).cmp(&(right.from(), right.kind(), right.to()))
    });
    public_edges.dedup();
    let public_vocabularies = vocabularies
        .vocabularies()
        .values()
        .map(|vocabulary| {
            ProjectPublicVocabulary::new(
                vocabulary.identity(),
                vocabulary.version(),
                vocabulary
                    .types()
                    .iter()
                    .filter(|ty| ty.is_public())
                    .map(|ty| ty.name().to_owned())
                    .collect(),
            )
        })
        .collect();
    ProjectInterface::with_vocabularies(public_vocabularies, exports, public_edges)
}

/// Converts a resolved source type to an alias-independent public type.
pub(super) fn public_type(
    ty: &TypeExpr,
    module_id: &str,
    modules: &BTreeMap<String, ModuleContext>,
    vocabularies: &ProjectVocabularySet,
) -> ProjectPublicType {
    match ty {
        TypeExpr::Num => ProjectPublicType::Num,
        TypeExpr::String => ProjectPublicType::String,
        TypeExpr::Bool => ProjectPublicType::Bool,
        TypeExpr::Url => ProjectPublicType::Url,
        TypeExpr::Path => ProjectPublicType::Path,
        TypeExpr::Nominal(alias, name, _) => {
            if let Some(vocabulary) = alias
                .as_ref()
                .and_then(|alias| vocabularies.resolve(module_id, alias))
            {
                return ProjectPublicType::VocabularyNominal {
                    identity: vocabulary.identity().to_owned(),
                    version: vocabulary.version().to_owned(),
                    name: name.clone(),
                };
            }
            let owner = alias.as_ref().map_or(module_id, |alias| {
                modules[module_id]
                    .aliases
                    .get(alias)
                    .expect("resolved import alias")
                    .as_str()
            });
            ProjectPublicType::Nominal(ModuleSymbolIdentity::new(
                LogicalModuleIdentity::new(V1_SOURCE_PROFILE, owner),
                name,
            ))
        }
        TypeExpr::List(inner) => ProjectPublicType::List(Box::new(public_type(
            inner,
            module_id,
            modules,
            vocabularies,
        ))),
        TypeExpr::Ref(inner) => ProjectPublicType::Ref(Box::new(public_type(
            inner,
            module_id,
            modules,
            vocabularies,
        ))),
        TypeExpr::Nullable(inner) => ProjectPublicType::Nullable(Box::new(public_type(
            inner,
            module_id,
            modules,
            vocabularies,
        ))),
    }
}
