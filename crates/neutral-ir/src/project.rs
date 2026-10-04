// SPDX-License-Identifier: Apache-2.0

//! Complete project data; constructors never imply independent validation.

use crate::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    project_interface::{
        ProjectInterface, ProjectPublicEdgeKind, ProjectPublicSignature, ProjectPublicType,
    },
};
use neutral_core::{CoreError, SourceContentDigest, SourceLocation, VocabularyContentDigest};
use std::collections::BTreeSet;

/// Complete project schema, independent of package release numbering.
pub const PROJECT_IR_SCHEMA: &str = "neutral.project-ir/1";
/// Project operation result envelope schema.
pub const PROJECT_RESULT_SCHEMA: &str = "neutral.project-result/1";
/// Post-compilation selection request schema.
pub const PROJECT_VIEW_SCHEMA: &str = "neutral.project-view/1";
/// Hard recursion ceiling shared by producers and independent readers.
pub const PROJECT_MAX_DEPTH: usize = crate::project_interface::MAX_PROJECT_INTERFACE_TYPE_DEPTH;

/// One complete module, including disconnected and private-only modules.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectModule {
    /// Exact logical module identity.
    pub identity: LogicalModuleIdentity,
    /// Distinct imported logical module IDs in canonical order, without aliases.
    pub imports: Vec<String>,
}

/// A fully materialized immutable value, without source or private syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectValue {
    /// Exact base-ten number.
    Number(ExactNumber),
    /// Exact decoded Unicode text.
    String(String),
    /// Boolean scalar.
    Bool(bool),
    /// Explicit nullable value.
    Null,
    /// Inert URL text; never a resolver or capability.
    Url(String),
    /// Inert path text; never a filesystem operation.
    Path(String),
    /// Ordered, fully materialized list.
    List(Vec<Self>),
    /// Contextually typed record fields in canonical name order.
    Record(Vec<(String, Self)>),
    /// Identity-only typed binding reference, never an embedded value.
    Reference(ModuleSymbolIdentity),
}

/// One complete declaration, including private validation content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDeclaration {
    /// Canonical module-symbol identity.
    pub identity: ModuleSymbolIdentity,
    /// Explicit source visibility, not authorization.
    pub public: bool,
    /// Resolved signature; its type representation also represents private types.
    pub signature: ProjectPublicSignature,
    /// Materialized binding value; records have none.
    pub value: Option<ProjectValue>,
    /// Closed record defaults in canonical field order; bindings have none.
    pub defaults: Vec<(String, ProjectValue)>,
}

/// Full locked vocabulary schema required to interpret contextual values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectVocabularyRecord {
    /// Canonical vocabulary identity.
    pub identity: String,
    /// Exact semantic revision.
    pub version: String,
    /// Nominal record name within the vocabulary.
    pub name: String,
    /// Source-authorable visibility.
    pub public: bool,
    /// Closed field signatures in canonical order.
    pub fields: Vec<(String, ProjectPublicType)>,
}

/// Exact source accounting, separate from logical meaning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSource {
    /// Owning logical module ID.
    pub module: String,
    /// Host-supplied opaque logical source ID, never a path.
    pub source_id: String,
    /// Digest of original captured bytes.
    pub digest: SourceContentDigest,
    /// Original byte length used to check spans independently.
    pub byte_len: u64,
}

/// Declaration-level original-byte source-map companion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSourceMap {
    /// Exact declaration identity.
    pub declaration: ModuleSymbolIdentity,
    /// Complete declaration span in its captured source.
    pub location: SourceLocation,
}

/// An occurrence-sensitive semantic edge retained as private provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectProvenance {
    /// Declaration consuming the dependency.
    pub from: ModuleSymbolIdentity,
    /// Declaration providing the dependency.
    pub to: ModuleSymbolIdentity,
    /// Typed dependency category.
    pub kind: ProjectPublicEdgeKind,
    /// Original-byte dependency occurrence, independent of identity.
    pub location: SourceLocation,
}

/// Recomputable resource accounting for complete publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectResourceFacts {
    /// Complete supplied source units.
    pub source_units: u64,
    /// Sum of exact original source byte lengths.
    pub source_bytes: u64,
    /// Complete exact vocabulary lock count.
    pub vocabulary_units: u64,
    /// Sum of exact captured vocabulary bytes.
    pub vocabulary_bytes: u64,
    /// Complete root declaration count, including private roots.
    pub declarations: u64,
    /// Alias-independent logical import edge count.
    pub import_edges: u64,
    /// Materialized value nodes, including unused closed defaults.
    pub value_nodes: u64,
}

/// Explicit immutable project validation bounds, never ambient defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectLimits {
    /// Maximum source/module count.
    pub modules: u64,
    /// Maximum root declarations.
    pub declarations: u64,
    /// Maximum logical import edges.
    pub import_edges: u64,
    /// Maximum materialized nodes and provenance occurrences.
    pub nodes: u64,
    /// Maximum aggregate value/source bytes and individual schema-name bytes.
    pub text_bytes: u64,
}

impl ProjectLimits {
    /// Intersects producer context with independent consumer bounds; neither can relax the other.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        Self {
            modules: self.modules.min(other.modules),
            declarations: self.declarations.min(other.declarations),
            import_edges: self.import_edges.min(other.import_edges),
            nodes: self.nodes.min(other.nodes),
            text_bytes: self.text_bytes.min(other.text_bytes),
        }
    }
}

/// Complete project IR plus non-semantic companions, before reader validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectIr {
    /// Exact independent project schema.
    pub schema: String,
    /// Every module in canonical order, never root-pruned.
    pub modules: Vec<ProjectModule>,
    /// Every fully typed declaration in module-symbol order.
    pub declarations: Vec<ProjectDeclaration>,
    /// Complete locked vocabulary record schemas, without aliases or metadata.
    pub vocabulary_records: Vec<ProjectVocabularyRecord>,
    /// Public export index and redacted public edges.
    pub public_interface: ProjectInterface,
    /// Exact source accounting in module order.
    pub sources: Vec<ProjectSource>,
    /// Exact complete declaration source maps.
    pub source_maps: Vec<ProjectSourceMap>,
    /// Every occurrence-sensitive declaration dependency.
    pub provenance: Vec<ProjectProvenance>,
    /// Processing context, excluded from logical equality.
    pub limits: ProjectLimits,
    /// Independently recomputable actual resource facts.
    pub resources: ProjectResourceFacts,
    /// Exact captured vocabulary evidence, separate from logical schema meaning.
    pub vocabulary_sources: Vec<ProjectVocabularySource>,
}

/// Exact locked vocabulary derivation companion, never authoring metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectVocabularySource {
    /// Canonical vocabulary identity.
    pub identity: String,
    /// Exact semantic revision.
    pub version: String,
    /// Digest of exact captured bundle bytes.
    pub digest: VocabularyContentDigest,
    /// Captured bundle byte count.
    pub byte_len: u64,
}

impl ProjectIr {
    /// Checks schema work/depth bounds before a consumer clones untrusted types.
    #[must_use]
    pub fn within_schema_limits(&self, limits: ProjectLimits) -> bool {
        if self.modules.len() as u64 > limits.modules
            || self.declarations.len() as u64 > limits.declarations
            || self.public_interface.exports().len() as u64 > limits.declarations
            || self.public_interface.edges().len() as u64 > limits.nodes
            || self.public_interface.vocabularies().len() as u64 > limits.nodes
            || self.vocabulary_records.len() as u64 > limits.nodes
            || !bounded_names(self, limits)
        {
            return false;
        }
        let mut count = 0_u64;
        for signature in self.declarations.iter().map(|decl| &decl.signature).chain(
            self.public_interface
                .exports()
                .iter()
                .map(crate::project_interface::ProjectPublicExport::signature),
        ) {
            match signature {
                ProjectPublicSignature::Binding(ty) => {
                    if !bounded_type(ty, &mut count, limits) {
                        return false;
                    }
                }
                ProjectPublicSignature::Record(fields) => {
                    let Some(next) = count.checked_add(fields.len() as u64) else {
                        return false;
                    };
                    count = next;
                    if count > limits.nodes {
                        return false;
                    }
                    for field in fields {
                        if field.name().len() as u64 > limits.text_bytes
                            || !bounded_type(field.ty(), &mut count, limits)
                        {
                            return false;
                        }
                    }
                }
            }
        }
        for record in &self.vocabulary_records {
            let Some(next) = count.checked_add(record.fields.len() as u64) else {
                return false;
            };
            count = next;
            if count > limits.nodes {
                return false;
            }
            for (name, ty) in &record.fields {
                if name.len() as u64 > limits.text_bytes || !bounded_type(ty, &mut count, limits) {
                    return false;
                }
            }
        }
        true
    }

    /// Derives the exact redacted export index from complete validated content.
    ///
    /// # Errors
    /// Returns an error when public fingerprint framing exceeds representable lengths.
    pub fn recompute_public_interface(&self) -> Result<ProjectInterface, CoreError> {
        use crate::project_interface::{ProjectPublicEdge, ProjectPublicExport};
        let public = self
            .declarations
            .iter()
            .filter(|decl| decl.public)
            .map(|decl| &decl.identity)
            .collect::<BTreeSet<_>>();
        let exports = self
            .declarations
            .iter()
            .filter(|decl| decl.public)
            .map(|decl| ProjectPublicExport::new(decl.identity.clone(), decl.signature.clone()))
            .collect();
        let mut edges = self
            .provenance
            .iter()
            .filter(|edge| public.contains(&edge.from) && public.contains(&edge.to))
            .map(|edge| (edge.from.clone(), edge.kind, edge.to.clone()))
            .collect::<BTreeSet<_>>();
        for decl in self.declarations.iter().filter(|decl| decl.public) {
            if let Some(value) = &decl.value {
                for target in value.references() {
                    edges.insert((
                        decl.identity.clone(),
                        ProjectPublicEdgeKind::Reference,
                        target.clone(),
                    ));
                }
            }
        }
        let edges = edges
            .into_iter()
            .map(|(from, kind, to)| ProjectPublicEdge::new(from, to, kind))
            .collect();
        ProjectInterface::with_vocabularies(
            self.public_interface.vocabularies().to_vec(),
            exports,
            edges,
        )
    }
    /// Compares complete logical content, excluding source and processing evidence.
    #[must_use]
    pub fn logical_eq(&self, other: &Self) -> bool {
        self.schema == other.schema
            && self.modules == other.modules
            && self.declarations == other.declarations
            && self.vocabulary_records == other.vocabulary_records
            && self.public_interface == other.public_interface
    }
}

/// Bounds one linear nested type without recursion or proportional allocation.
fn bounded_type(mut ty: &ProjectPublicType, count: &mut u64, limits: ProjectLimits) -> bool {
    let mut depth = 0;
    loop {
        let Some(next) = count.checked_add(1) else {
            return false;
        };
        *count = next;
        if *count > limits.nodes || depth > PROJECT_MAX_DEPTH {
            return false;
        }
        match ty {
            ProjectPublicType::List(inner)
            | ProjectPublicType::Nullable(inner)
            | ProjectPublicType::Ref(inner) => {
                ty = inner;
                depth += 1;
            }
            ProjectPublicType::Nominal(identity) => {
                return bounded_symbol(identity, limits.text_bytes);
            }
            ProjectPublicType::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                return [identity, version, name]
                    .iter()
                    .all(|value| value.len() as u64 <= limits.text_bytes);
            }
            _ => return true,
        }
    }
}

/// Checks each logical identity before name parsing or public-interface copying.
fn bounded_symbol(identity: &ModuleSymbolIdentity, text_bytes: u64) -> bool {
    identity.declaration_name().len() as u64 <= text_bytes
        && identity.module().module_name().len() as u64 <= text_bytes
        && identity.module().language_behavior_version().len() as u64 <= text_bytes
}

/// Bounds schema names and catalogues before any proportional validation work.
fn bounded_names(ir: &ProjectIr, limits: ProjectLimits) -> bool {
    for module in &ir.modules {
        if module.identity.module_name().len() as u64 > limits.text_bytes
            || module.imports.len() as u64 > limits.import_edges
            || module
                .imports
                .iter()
                .any(|target| target.len() as u64 > limits.text_bytes)
        {
            return false;
        }
    }
    let identities = ir
        .declarations
        .iter()
        .map(|decl| &decl.identity)
        .chain(
            ir.public_interface
                .exports()
                .iter()
                .map(crate::project_interface::ProjectPublicExport::identity),
        )
        .chain(
            ir.public_interface
                .edges()
                .iter()
                .flat_map(|edge| [edge.from(), edge.to()]),
        );
    if identities
        .into_iter()
        .any(|identity| !bounded_symbol(identity, limits.text_bytes))
    {
        return false;
    }
    let mut names = 0_u64;
    for vocabulary in ir.public_interface.vocabularies() {
        let Some(next) = names.checked_add(vocabulary.public_types().len() as u64) else {
            return false;
        };
        names = next;
        if names > limits.nodes
            || vocabulary.identity().len() as u64 > limits.text_bytes
            || vocabulary.version().len() as u64 > limits.text_bytes
            || vocabulary
                .public_types()
                .iter()
                .any(|name| name.len() as u64 > limits.text_bytes)
        {
            return false;
        }
    }
    ir.vocabulary_records.iter().all(|record| {
        [&record.identity, &record.version, &record.name]
            .iter()
            .all(|name| name.len() as u64 <= limits.text_bytes)
    })
}

impl ProjectValue {
    /// Collects typed identity targets iteratively without following identity edges.
    #[must_use]
    pub fn references(&self) -> Vec<&ModuleSymbolIdentity> {
        let mut targets = Vec::new();
        let mut pending = vec![self];
        while let Some(value) = pending.pop() {
            match value {
                Self::Reference(target) => targets.push(target),
                Self::List(values) => pending.extend(values),
                Self::Record(fields) => pending.extend(fields.iter().map(|(_, value)| value)),
                _ => {}
            }
        }
        targets
    }
}

/// A selection applied only to a previously validated complete project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewRequest {
    /// Exact request schema.
    pub schema: String,
    /// Distinct selected public roots; input order is non-semantic.
    pub roots: Vec<ModuleSymbolIdentity>,
}
