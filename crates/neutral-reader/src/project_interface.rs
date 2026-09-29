// SPDX-License-Identifier: Apache-2.0

//! Independent validation and redacted traversal of Stage 4 public interfaces.

use neutral_core::profile::V1_SOURCE_PROFILE;
use neutral_ir::{
    ModuleSymbolIdentity,
    language::{is_snake_name, is_upper_name},
    project_interface::{
        MAX_PROJECT_INTERFACE_TYPE_DEPTH, ProjectInterface, ProjectPublicEdge,
        ProjectPublicEdgeKind, ProjectPublicExport, ProjectPublicSignature, ProjectPublicType,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// A reader rejection of an invalid or non-public project interface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectInterfaceError {
    /// An export identity was malformed, duplicated, or out of order.
    InvalidExport,
    /// A public signature had malformed fields, inaccessible types, or excess depth.
    InvalidSignature,
    /// A public edge was dangling, duplicated, incorrectly typed, or out of order.
    InvalidEdge,
    /// The declared public signature fingerprint did not match its content.
    InvalidFingerprint,
}

/// Immutable, independently validated public-only project interface.
#[derive(Clone, Debug)]
pub struct ValidatedProjectInterface {
    /// Complete public-only snapshot after structural and digest validation.
    interface: Arc<ProjectInterface>,
}

impl ValidatedProjectInterface {
    /// Validates a public interface without compiler linkage or source parsing.
    ///
    /// # Errors
    ///
    /// Rejects invalid export order, types, edges, or fingerprint before a view
    /// is published. No source IDs, private roots, or raw provenance are exposed.
    pub fn from_interface(interface: Arc<ProjectInterface>) -> Result<Self, ProjectInterfaceError> {
        let exports = interface.exports();
        let mut index = BTreeMap::new();
        let mut previous = None;
        for (position, export) in exports.iter().enumerate() {
            if previous.is_some_and(|identity| identity >= export.identity())
                || !valid_identity(export.identity(), export.signature())
            {
                return Err(ProjectInterfaceError::InvalidExport);
            }
            previous = Some(export.identity());
            index.insert(export.identity(), position);
        }
        for export in exports {
            if !valid_signature(export.signature(), &index, exports) {
                return Err(ProjectInterfaceError::InvalidSignature);
            }
        }
        let mut previous_edge = None;
        for edge in interface.edges() {
            let key = (edge.from(), edge.kind(), edge.to());
            if previous_edge.is_some_and(|previous| previous >= key)
                || !valid_edge(edge, &index, exports)
            {
                return Err(ProjectInterfaceError::InvalidEdge);
            }
            previous_edge = Some(key);
        }
        let mut expected_types = BTreeSet::new();
        for export in exports {
            match export.signature() {
                ProjectPublicSignature::Binding(ty) => {
                    collect_type_edges(export.identity(), ty, false, &mut expected_types);
                }
                ProjectPublicSignature::Record(fields) => {
                    for field in fields {
                        collect_type_edges(
                            export.identity(),
                            field.ty(),
                            false,
                            &mut expected_types,
                        );
                    }
                }
            }
        }
        let actual_types = interface
            .edges()
            .iter()
            .filter(|edge| {
                matches!(
                    edge.kind(),
                    ProjectPublicEdgeKind::Type | ProjectPublicEdgeKind::ReferenceType
                )
            })
            .map(|edge| (edge.from().clone(), edge.kind(), edge.to().clone()))
            .collect::<BTreeSet<_>>();
        if actual_types != expected_types {
            return Err(ProjectInterfaceError::InvalidEdge);
        }
        if interface.recompute_fingerprint().ok() != Some(interface.fingerprint()) {
            return Err(ProjectInterfaceError::InvalidFingerprint);
        }
        Ok(Self { interface })
    }

    /// Returns public exports in canonical module-symbol order.
    #[must_use]
    pub fn exports(&self) -> &[ProjectPublicExport] {
        self.interface.exports()
    }

    /// Finds one public export by exact full module-symbol identity.
    #[must_use]
    pub fn export_by_identity(
        &self,
        identity: &ModuleSymbolIdentity,
    ) -> Option<&ProjectPublicExport> {
        self.interface
            .exports()
            .binary_search_by(|export| export.identity().cmp(identity))
            .ok()
            .map(|index| &self.interface.exports()[index])
    }

    /// Returns only public-to-public dependency edges with source details redacted.
    #[must_use]
    pub fn public_edges(&self) -> &[ProjectPublicEdge] {
        self.interface.edges()
    }

    /// Enumerates cross-module immutable reuse without a compiler dependency.
    pub fn cross_module_values(&self) -> impl Iterator<Item = &ProjectPublicEdge> {
        self.public_edges().iter().filter(|edge| {
            edge.kind() == ProjectPublicEdgeKind::Value
                && edge.from().module() != edge.to().module()
        })
    }

    /// Enumerates cross-module identity references without a compiler dependency.
    pub fn cross_module_references(&self) -> impl Iterator<Item = &ProjectPublicEdge> {
        self.public_edges().iter().filter(|edge| {
            edge.kind() == ProjectPublicEdgeKind::Reference
                && edge.from().module() != edge.to().module()
        })
    }

    /// Returns the checked public-signature fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> neutral_core::SemanticDigest {
        self.interface.fingerprint()
    }
}

/// Collects the exact nominal dependencies implied by one validated public type.
fn collect_type_edges(
    from: &ModuleSymbolIdentity,
    ty: &ProjectPublicType,
    under_ref: bool,
    edges: &mut BTreeSet<(
        ModuleSymbolIdentity,
        ProjectPublicEdgeKind,
        ModuleSymbolIdentity,
    )>,
) {
    match ty {
        ProjectPublicType::Nominal(to) => {
            let kind = if under_ref {
                ProjectPublicEdgeKind::ReferenceType
            } else {
                ProjectPublicEdgeKind::Type
            };
            edges.insert((from.clone(), kind, to.clone()));
        }
        ProjectPublicType::List(inner) | ProjectPublicType::Nullable(inner) => {
            collect_type_edges(from, inner, under_ref, edges);
        }
        ProjectPublicType::Ref(inner) => collect_type_edges(from, inner, true, edges),
        ProjectPublicType::Num | ProjectPublicType::String | ProjectPublicType::Bool => {}
    }
}

/// Checks exact profile, qualified module grammar, and root-name category.
fn valid_identity(identity: &ModuleSymbolIdentity, signature: &ProjectPublicSignature) -> bool {
    if identity.module().language_behavior_version() != V1_SOURCE_PROFILE
        || !identity
            .module()
            .module_name()
            .split("::")
            .all(is_snake_name)
    {
        return false;
    }
    match signature {
        ProjectPublicSignature::Binding(_) => is_snake_name(identity.declaration_name()),
        ProjectPublicSignature::Record(_) => is_upper_name(identity.declaration_name()),
    }
}

/// Checks a complete public signature and canonical field ordering.
fn valid_signature(
    signature: &ProjectPublicSignature,
    index: &BTreeMap<&ModuleSymbolIdentity, usize>,
    exports: &[ProjectPublicExport],
) -> bool {
    match signature {
        ProjectPublicSignature::Binding(ty) => valid_type(ty, 0, index, exports),
        ProjectPublicSignature::Record(fields) => {
            let mut previous = None;
            for field in fields {
                if !is_snake_name(field.name())
                    || previous.is_some_and(|name| name >= field.name())
                    || !valid_type(field.ty(), 0, index, exports)
                {
                    return false;
                }
                previous = Some(field.name());
            }
            true
        }
    }
}

/// Checks nested public nominal closure before recursive fingerprinting.
fn valid_type(
    ty: &ProjectPublicType,
    depth: usize,
    index: &BTreeMap<&ModuleSymbolIdentity, usize>,
    exports: &[ProjectPublicExport],
) -> bool {
    if depth > MAX_PROJECT_INTERFACE_TYPE_DEPTH {
        return false;
    }
    match ty {
        ProjectPublicType::Num | ProjectPublicType::String | ProjectPublicType::Bool => true,
        ProjectPublicType::Nominal(identity) => index.get(identity).is_some_and(|position| {
            matches!(
                exports[*position].signature(),
                ProjectPublicSignature::Record(_)
            )
        }),
        ProjectPublicType::List(inner) | ProjectPublicType::Ref(inner) => {
            valid_type(inner, depth + 1, index, exports)
        }
        ProjectPublicType::Nullable(inner) => {
            !matches!(inner.as_ref(), ProjectPublicType::Nullable(_))
                && valid_type(inner, depth + 1, index, exports)
        }
    }
}

/// Checks public endpoints and edge category against their export kinds.
fn valid_edge(
    edge: &ProjectPublicEdge,
    index: &BTreeMap<&ModuleSymbolIdentity, usize>,
    exports: &[ProjectPublicExport],
) -> bool {
    let Some(&from) = index.get(edge.from()) else {
        return false;
    };
    let Some(&to) = index.get(edge.to()) else {
        return false;
    };
    match edge.kind() {
        ProjectPublicEdgeKind::Type | ProjectPublicEdgeKind::ReferenceType => {
            matches!(exports[to].signature(), ProjectPublicSignature::Record(_))
        }
        ProjectPublicEdgeKind::Value | ProjectPublicEdgeKind::Reference => {
            matches!(
                exports[from].signature(),
                ProjectPublicSignature::Binding(_)
            ) && matches!(exports[to].signature(), ProjectPublicSignature::Binding(_))
        }
    }
}
