// SPDX-License-Identifier: Apache-2.0

//! Bounded complete-project validation and post-compilation public views.

use crate::ValidatedProjectInterface;
use neutral_core::{CancellationToken, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    ModuleSymbolIdentity,
    language::{is_exact_release_version, is_protected_name, is_snake_name, is_upper_name},
    project::{
        PROJECT_IR_SCHEMA, PROJECT_MAX_DEPTH, PROJECT_RESULT_SCHEMA, PROJECT_VIEW_SCHEMA,
        ProjectDeclaration, ProjectIr, ProjectLimits, ProjectResourceFacts, ProjectValue,
        ProjectVocabularyRecord, ViewRequest,
    },
    project_interface::{ProjectPublicExport, ProjectPublicSignature, ProjectPublicType},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Fail-closed complete-project reader and view classifications.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectReadError {
    /// Unknown schema or profile.
    Schema,
    /// An explicit caller bound was exceeded.
    Limit,
    /// Invalid, duplicate, unordered, or dangling module/import.
    Module,
    /// Invalid declaration, type, value, or semantic cycle.
    Declaration,
    /// Missing, mismatched, private-leaking, or malformed export index.
    PublicInterface,
    /// Invalid original-byte source or provenance companion.
    Companion,
    /// Declared resource facts differ from actual content.
    Resources,
    /// Cancellation prevented publication.
    Cancelled,
    /// Invalid, duplicate, absent, or private selected root.
    View,
}

impl ProjectReadError {
    /// Returns the typed outcome schema, independent of package releases.
    #[must_use]
    pub const fn schema(self) -> &'static str {
        PROJECT_RESULT_SCHEMA
    }
}

/// Immutable complete-project authority, published only after independent checks.
#[derive(Clone, Debug)]
pub struct ValidatedProject {
    /// Validated complete data, inaccessible through a public-only view.
    ir: Arc<ProjectIr>,
    /// Independently validated export index.
    interface: ValidatedProjectInterface,
}

/// Public-only selected dependency closure, without source or private provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectView {
    /// Canonical selected roots, separate from the complete project.
    roots: Vec<ModuleSymbolIdentity>,
    /// Full public interpretive declaration closure.
    exports: Vec<ProjectPublicExport>,
    /// Fully materialized public binding values in canonical order.
    values: Vec<(ModuleSymbolIdentity, ProjectValue)>,
    /// Public locked schemas needed to interpret the selected nominal values.
    vocabulary_records: Vec<ProjectVocabularyRecord>,
}

impl ProjectView {
    /// Returns the post-compilation view schema, never a complete IR schema.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        PROJECT_VIEW_SCHEMA
    }
    /// Returns the normalized selection, never a capture input.
    #[must_use]
    pub fn roots(&self) -> &[ModuleSymbolIdentity] {
        &self.roots
    }
    /// Returns required public declaration signatures, without private defaults.
    #[must_use]
    pub fn exports(&self) -> &[ProjectPublicExport] {
        &self.exports
    }
    /// Returns public materialized binding values, with no source accounting.
    #[must_use]
    pub fn values(&self) -> &[(ModuleSymbolIdentity, ProjectValue)] {
        &self.values
    }
    /// Returns exact public vocabulary interpretation dependencies, never metadata.
    #[must_use]
    pub fn vocabulary_records(&self) -> &[ProjectVocabularyRecord] {
        &self.vocabulary_records
    }
}

impl ValidatedProject {
    /// Validates complete data without source parsing or compiler-private linkage.
    ///
    /// # Errors
    /// Rejects malformed, incomplete, over-budget, or cancelled projects atomically.
    pub fn from_ir(
        ir: Arc<ProjectIr>,
        limits: ProjectLimits,
        cancellation: &CancellationToken,
    ) -> Result<Self, ProjectReadError> {
        check_cancel(cancellation)?;
        if ir.schema != PROJECT_IR_SCHEMA {
            return Err(ProjectReadError::Schema);
        }
        let limits = limits.intersect(ir.limits);
        check_schema_bounds(&ir, limits, cancellation)?;
        check_modules(&ir, limits)?;
        let index = declaration_index(&ir)?;
        check_vocabularies(&ir, limits)?;
        check_vocabulary_cycles(&ir, cancellation)?;
        let mut budget = ValueBudget {
            nodes: 0,
            bytes: 0,
            limits,
            cancellation,
        };
        check_declarations(&ir, &index, &mut budget)?;
        check_companions(&ir, &index, limits)?;
        check_semantic_graph(&ir, &index, cancellation)?;
        let interface =
            ValidatedProjectInterface::from_interface(Arc::new(ir.public_interface.clone()))
                .map_err(|_| ProjectReadError::PublicInterface)?;
        let expected = ir
            .declarations
            .iter()
            .filter(|decl| decl.public)
            .map(|decl| ProjectPublicExport::new(decl.identity.clone(), decl.signature.clone()))
            .collect::<Vec<_>>();
        if interface.exports() != expected {
            return Err(ProjectReadError::PublicInterface);
        }
        if ir.recompute_public_interface().ok().as_ref() != Some(&ir.public_interface) {
            return Err(ProjectReadError::PublicInterface);
        }
        let actual = ProjectResourceFacts {
            source_units: ir.sources.len() as u64,
            source_bytes: ir
                .sources
                .iter()
                .try_fold(0_u64, |sum, source| sum.checked_add(source.byte_len))
                .ok_or(ProjectReadError::Limit)?,
            vocabulary_units: ir.vocabulary_sources.len() as u64,
            vocabulary_bytes: ir
                .vocabulary_sources
                .iter()
                .try_fold(0_u64, |sum, source| sum.checked_add(source.byte_len))
                .ok_or(ProjectReadError::Limit)?,
            declarations: ir.declarations.len() as u64,
            import_edges: ir
                .modules
                .iter()
                .map(|module| module.imports.len() as u64)
                .sum(),
            value_nodes: budget.nodes,
        };
        if actual != ir.resources {
            return Err(ProjectReadError::Resources);
        }
        check_cancel(cancellation)?;
        Ok(Self { ir, interface })
    }

    /// Returns complete validated data for trusted inspection, not a redacted view.
    #[must_use]
    pub const fn complete_ir(&self) -> &Arc<ProjectIr> {
        &self.ir
    }

    /// Derives a canonical public dependency closure without changing complete IR.
    ///
    /// # Errors
    /// Rejects unknown/private/duplicate roots, schema mismatch, limits, or cancellation.
    pub fn derive_view(
        &self,
        request: &ViewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ProjectView, ProjectReadError> {
        check_cancel(cancellation)?;
        if request.schema != PROJECT_VIEW_SCHEMA {
            return Err(ProjectReadError::Schema);
        }
        if request.roots.len() > self.ir.declarations.len() {
            return Err(ProjectReadError::Limit);
        }
        let roots = request.roots.iter().cloned().collect::<BTreeSet<_>>();
        if roots.len() != request.roots.len()
            || roots
                .iter()
                .any(|root| self.interface.export_by_identity(root).is_none())
        {
            return Err(ProjectReadError::View);
        }
        let mut closure = roots.clone();
        let mut pending = roots.iter().cloned().collect::<Vec<_>>();
        let mut adjacency = BTreeMap::<_, Vec<_>>::new();
        for edge in self.interface.public_edges() {
            adjacency.entry(edge.from()).or_default().push(edge.to());
        }
        while let Some(from) = pending.pop() {
            check_cancel(cancellation)?;
            for to in adjacency.get(&from).into_iter().flatten() {
                if closure.insert((*to).clone()) {
                    pending.push((*to).clone());
                }
            }
        }
        let exports: Vec<_> = self
            .interface
            .exports()
            .iter()
            .filter(|export| closure.contains(export.identity()))
            .cloned()
            .collect();
        let values = self
            .ir
            .declarations
            .iter()
            .filter(|decl| closure.contains(&decl.identity))
            .filter_map(|decl| {
                decl.value
                    .as_ref()
                    .map(|value| (decl.identity.clone(), value.clone()))
            })
            .collect();
        let vocabulary_records = view_vocabularies(&exports, &self.ir, cancellation)?;
        Ok(ProjectView {
            roots: roots.into_iter().collect(),
            exports,
            values,
            vocabulary_records,
        })
    }
}

/// Derives the transitive public vocabulary schema closure of exported types.
fn view_vocabularies(
    exports: &[ProjectPublicExport],
    ir: &ProjectIr,
    cancellation: &CancellationToken,
) -> Result<Vec<ProjectVocabularyRecord>, ProjectReadError> {
    let mut types = Vec::new();
    for export in exports {
        match export.signature() {
            ProjectPublicSignature::Binding(ty) => types.push(ty),
            ProjectPublicSignature::Record(fields) => types.extend(
                fields
                    .iter()
                    .map(neutral_ir::project_interface::ProjectPublicField::ty),
            ),
        }
    }
    let mut selected = BTreeSet::new();
    while let Some(ty) = types.pop() {
        check_cancel(cancellation)?;
        match ty {
            ProjectPublicType::List(inner)
            | ProjectPublicType::Ref(inner)
            | ProjectPublicType::Nullable(inner) => types.push(inner),
            ProjectPublicType::VocabularyNominal {
                identity,
                version,
                name,
            } if selected.insert((identity, version, name)) => {
                let record = ir
                    .vocabulary_records
                    .iter()
                    .find(|record| {
                        record.identity == *identity
                            && record.version == *version
                            && record.name == *name
                            && record.public
                    })
                    .ok_or(ProjectReadError::PublicInterface)?;
                types.extend(record.fields.iter().map(|(_, ty)| ty));
            }
            _ => {}
        }
    }
    Ok(ir
        .vocabulary_records
        .iter()
        .filter(|record| selected.contains(&(&record.identity, &record.version, &record.name)))
        .cloned()
        .collect())
}

/// Checks cooperative cancellation at bounded work boundaries.
fn check_cancel(cancellation: &CancellationToken) -> Result<(), ProjectReadError> {
    if cancellation.is_cancelled() {
        Err(ProjectReadError::Cancelled)
    } else {
        Ok(())
    }
}

/// Rejects malformed module identities, imports, and caller limit violations.
fn check_modules(ir: &ProjectIr, limits: ProjectLimits) -> Result<(), ProjectReadError> {
    if ir.modules.is_empty()
        || ir.modules.len() as u64 > limits.modules
        || ir.declarations.len() as u64 > limits.declarations
        || ir.provenance.len() as u64 > limits.nodes
        || ir.sources.len() != ir.modules.len()
    {
        return Err(ProjectReadError::Limit);
    }
    let mut previous = None;
    let modules = ir
        .modules
        .iter()
        .map(|module| module.identity.module_name())
        .collect::<BTreeSet<_>>();
    let mut edges = 0_u64;
    for module in &ir.modules {
        let name = module.identity.module_name();
        if module.identity.language_behavior_version() != V1_SOURCE_PROFILE
            || !name.split("::").all(is_snake_name)
            || previous.is_some_and(|prior| prior >= name)
        {
            return Err(ProjectReadError::Module);
        }
        previous = Some(name);
        let mut prior_import = None;
        for target in &module.imports {
            if target == name
                || !modules.contains(target.as_str())
                || prior_import.is_some_and(|prior| prior >= target)
            {
                return Err(ProjectReadError::Module);
            }
            prior_import = Some(target);
            edges = edges.checked_add(1).ok_or(ProjectReadError::Limit)?;
        }
    }
    if edges > limits.import_edges {
        return Err(ProjectReadError::Limit);
    }
    Ok(())
}

/// Builds a checked canonical declaration index with valid name categories.
fn declaration_index(
    ir: &ProjectIr,
) -> Result<BTreeMap<&ModuleSymbolIdentity, &ProjectDeclaration>, ProjectReadError> {
    let mut index = BTreeMap::new();
    let mut previous = None;
    for decl in &ir.declarations {
        let valid_name = match decl.signature {
            ProjectPublicSignature::Binding(_) => is_snake_name(decl.identity.declaration_name()),
            ProjectPublicSignature::Record(_) => is_upper_name(decl.identity.declaration_name()),
        };
        if !valid_name
            || is_protected_name(decl.identity.declaration_name())
            || previous.is_some_and(|prior| prior >= &decl.identity)
            || !ir
                .modules
                .iter()
                .any(|module| module.identity == *decl.identity.module())
        {
            return Err(ProjectReadError::Declaration);
        }
        previous = Some(&decl.identity);
        index.insert(&decl.identity, decl);
    }
    Ok(index)
}

/// Bounds complete and exported schemas before cloning or graph construction.
fn check_schema_bounds(
    ir: &ProjectIr,
    limits: ProjectLimits,
    cancellation: &CancellationToken,
) -> Result<(), ProjectReadError> {
    check_cancel(cancellation)?;
    if !ir.within_schema_limits(limits) {
        return Err(ProjectReadError::Limit);
    }
    check_cancel(cancellation)
}

/// Checks complete canonical vocabulary schemas against the public catalogue.
fn check_vocabularies(ir: &ProjectIr, limits: ProjectLimits) -> Result<(), ProjectReadError> {
    if ir.vocabulary_records.len() as u64 > limits.nodes {
        return Err(ProjectReadError::Limit);
    }
    let mut previous = None;
    for record in &ir.vocabulary_records {
        let key = (&record.identity, &record.version, &record.name);
        if !is_upper_name(&record.identity)
            || !is_exact_release_version(&record.version)
            || !is_upper_name(&record.name)
            || previous.is_some_and(|prior| prior >= key)
        {
            return Err(ProjectReadError::Declaration);
        }
        previous = Some(key);
        let vocabulary = ir
            .public_interface
            .vocabularies()
            .iter()
            .find(|vocabulary| {
                vocabulary.identity() == record.identity && vocabulary.version() == record.version
            })
            .ok_or(ProjectReadError::PublicInterface)?;
        if vocabulary
            .public_types()
            .binary_search(&record.name)
            .is_ok()
            != record.public
        {
            return Err(ProjectReadError::PublicInterface);
        }
        let mut prior = None;
        if record.fields.len() as u64 > limits.nodes {
            return Err(ProjectReadError::Limit);
        }
        for (name, ty) in &record.fields {
            if !is_snake_name(name)
                || is_protected_name(name)
                || prior.is_some_and(|old| old >= name)
            {
                return Err(ProjectReadError::Declaration);
            }
            prior = Some(name);
            match ty {
                ProjectPublicType::Num
                | ProjectPublicType::String
                | ProjectPublicType::Bool
                | ProjectPublicType::Url
                | ProjectPublicType::Path => {}
                ProjectPublicType::VocabularyNominal {
                    identity,
                    version,
                    name,
                } if identity == &record.identity
                    && version == &record.version
                    && ir.vocabulary_records.iter().any(|target| {
                        target.identity == *identity
                            && target.version == *version
                            && target.name == *name
                            && (!record.public || target.public)
                    }) => {}
                _ => return Err(ProjectReadError::Declaration),
            }
        }
    }
    for vocabulary in ir.public_interface.vocabularies() {
        for name in vocabulary.public_types() {
            if !ir.vocabulary_records.iter().any(|record| {
                record.identity == vocabulary.identity()
                    && record.version == vocabulary.version()
                    && record.name == *name
                    && record.public
            }) {
                return Err(ProjectReadError::PublicInterface);
            }
        }
    }
    Ok(())
}

/// Rejects embedded vocabulary type cycles without recursive graph traversal.
fn check_vocabulary_cycles(
    ir: &ProjectIr,
    cancellation: &CancellationToken,
) -> Result<(), ProjectReadError> {
    let index = ir
        .vocabulary_records
        .iter()
        .enumerate()
        .map(|(position, record)| {
            (
                (
                    record.identity.as_str(),
                    record.version.as_str(),
                    record.name.as_str(),
                ),
                position,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = vec![BTreeSet::new(); index.len()];
    let mut incoming = vec![0_usize; index.len()];
    for (position, record) in ir.vocabulary_records.iter().enumerate() {
        check_cancel(cancellation)?;
        for (_, ty) in &record.fields {
            if let ProjectPublicType::VocabularyNominal {
                identity,
                version,
                name,
            } = ty
            {
                let target = *index
                    .get(&(identity.as_str(), version.as_str(), name.as_str()))
                    .ok_or(ProjectReadError::Declaration)?;
                if outgoing[position].insert(target) {
                    incoming[target] += 1;
                }
            }
        }
    }
    let mut pending = incoming
        .iter()
        .enumerate()
        .filter_map(|(position, count)| (*count == 0).then_some(position))
        .collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(position) = pending.pop() {
        check_cancel(cancellation)?;
        visited += 1;
        for target in &outgoing[position] {
            incoming[*target] -= 1;
            if incoming[*target] == 0 {
                pending.push(*target);
            }
        }
    }
    if visited != index.len() {
        return Err(ProjectReadError::Declaration);
    }
    Ok(())
}

/// Verifies complete type/provenance edges, imported visibility, and non-reference cycles.
fn check_semantic_graph(
    ir: &ProjectIr,
    index: &BTreeMap<&ModuleSymbolIdentity, &ProjectDeclaration>,
    cancellation: &CancellationToken,
) -> Result<(), ProjectReadError> {
    use neutral_ir::project_interface::ProjectPublicEdgeKind;
    let expected = expected_type_edges(ir, cancellation)?;
    let actual = ir
        .provenance
        .iter()
        .filter(|edge| {
            matches!(
                edge.kind,
                ProjectPublicEdgeKind::Type | ProjectPublicEdgeKind::ReferenceType
            )
        })
        .map(|edge| (&edge.from, edge.kind, &edge.to))
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(ProjectReadError::Companion);
    }
    let mut adjacency = BTreeMap::<_, BTreeSet<_>>::new();
    let mut indegrees = index
        .keys()
        .map(|identity| (*identity, 0_usize))
        .collect::<BTreeMap<_, _>>();
    for edge in &ir.provenance {
        let from = index[&edge.from];
        let to = index[&edge.to];
        if edge.from.module() != edge.to.module()
            && (!to.public
                || !ir.modules.iter().any(|module| {
                    module.identity == *edge.from.module()
                        && module
                            .imports
                            .iter()
                            .any(|target| target == edge.to.module().module_name())
                }))
        {
            return Err(ProjectReadError::Declaration);
        }
        if matches!(
            edge.kind,
            ProjectPublicEdgeKind::Reference | ProjectPublicEdgeKind::Value
        ) && (!matches!(from.signature, ProjectPublicSignature::Binding(_))
            || !matches!(to.signature, ProjectPublicSignature::Binding(_)))
        {
            return Err(ProjectReadError::Declaration);
        }
        if matches!(
            edge.kind,
            ProjectPublicEdgeKind::Type | ProjectPublicEdgeKind::Value
        ) && adjacency.entry(&edge.from).or_default().insert(&edge.to)
        {
            *indegrees
                .get_mut(&edge.to)
                .ok_or(ProjectReadError::Declaration)? += 1;
        }
    }
    let mut ready = indegrees
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(identity, _)| *identity)
        .collect::<Vec<_>>();
    let mut seen = 0;
    while let Some(from) = ready.pop() {
        check_cancel(cancellation)?;
        seen += 1;
        for to in adjacency.get(from).into_iter().flatten() {
            let degree = indegrees.get_mut(to).ok_or(ProjectReadError::Declaration)?;
            *degree -= 1;
            if *degree == 0 {
                ready.push(to);
            }
        }
    }
    if seen != index.len() {
        return Err(ProjectReadError::Declaration);
    }
    Ok(())
}

/// Canonical nominal type-edge keys, independent of source occurrence multiplicity.
type TypeEdges<'a> = BTreeSet<(
    &'a ModuleSymbolIdentity,
    neutral_ir::project_interface::ProjectPublicEdgeKind,
    &'a ModuleSymbolIdentity,
)>;

/// Derives the required nominal type edges from all complete signatures.
fn expected_type_edges<'a>(
    ir: &'a ProjectIr,
    cancellation: &CancellationToken,
) -> Result<TypeEdges<'a>, ProjectReadError> {
    use neutral_ir::project_interface::ProjectPublicEdgeKind;
    let mut expected = BTreeSet::new();
    for decl in &ir.declarations {
        check_cancel(cancellation)?;
        let types = match &decl.signature {
            ProjectPublicSignature::Binding(ty) => vec![ty],
            ProjectPublicSignature::Record(fields) => fields
                .iter()
                .map(neutral_ir::project_interface::ProjectPublicField::ty)
                .collect(),
        };
        let mut pending = types.into_iter().map(|ty| (ty, false)).collect::<Vec<_>>();
        while let Some((ty, under_ref)) = pending.pop() {
            match ty {
                ProjectPublicType::Nominal(target) => {
                    expected.insert((
                        &decl.identity,
                        if under_ref {
                            ProjectPublicEdgeKind::ReferenceType
                        } else {
                            ProjectPublicEdgeKind::Type
                        },
                        target,
                    ));
                }
                ProjectPublicType::Ref(inner) => pending.push((inner, true)),
                ProjectPublicType::List(inner) | ProjectPublicType::Nullable(inner) => {
                    pending.push((inner, under_ref));
                }
                _ => {}
            }
        }
    }
    Ok(expected)
}

/// Shared aggregate value/text work accounting across all declarations and defaults.
struct ValueBudget<'a> {
    /// Nodes processed so far.
    nodes: u64,
    /// Retained text bytes processed so far.
    bytes: u64,
    /// Caller-provided maximums.
    limits: ProjectLimits,
    /// Cooperative cancellation input.
    cancellation: &'a CancellationToken,
}

impl ValueBudget<'_> {
    /// Charges a node and its retained textual payload before further work.
    fn charge(&mut self, bytes: usize) -> Result<(), ProjectReadError> {
        check_cancel(self.cancellation)?;
        self.nodes = self.nodes.checked_add(1).ok_or(ProjectReadError::Limit)?;
        self.bytes = self
            .bytes
            .checked_add(bytes as u64)
            .ok_or(ProjectReadError::Limit)?;
        if self.nodes > self.limits.nodes || self.bytes > self.limits.text_bytes {
            return Err(ProjectReadError::Limit);
        }
        Ok(())
    }
}

/// Validates all typed roots and closed defaults before publication.
fn check_declarations(
    ir: &ProjectIr,
    index: &BTreeMap<&ModuleSymbolIdentity, &ProjectDeclaration>,
    budget: &mut ValueBudget<'_>,
) -> Result<(), ProjectReadError> {
    for decl in &ir.declarations {
        match (&decl.signature, &decl.value) {
            (ProjectPublicSignature::Binding(ty), Some(value)) if decl.defaults.is_empty() => {
                check_type(ty, ir, index, 0)?;
                check_value(value, ty, decl.public, false, ir, index, budget, 0)?;
            }
            (ProjectPublicSignature::Record(fields), None) => {
                let mut previous = None;
                for field in fields {
                    if !is_snake_name(field.name())
                        || is_protected_name(field.name())
                        || previous.is_some_and(|old| old >= field.name())
                    {
                        return Err(ProjectReadError::Declaration);
                    }
                    previous = Some(field.name());
                    check_type(field.ty(), ir, index, 0)?;
                }
                let mut prior = None;
                for (name, value) in &decl.defaults {
                    if prior.is_some_and(|old| old >= name) {
                        return Err(ProjectReadError::Declaration);
                    }
                    prior = Some(name);
                    let field = fields
                        .iter()
                        .find(|field| field.name() == name)
                        .ok_or(ProjectReadError::Declaration)?;
                    check_value(value, field.ty(), decl.public, true, ir, index, budget, 0)?;
                }
            }
            _ => return Err(ProjectReadError::Declaration),
        }
    }
    Ok(())
}

/// Validates bounded canonical type closure against complete nominal schemas.
fn check_type(
    ty: &ProjectPublicType,
    ir: &ProjectIr,
    index: &BTreeMap<&ModuleSymbolIdentity, &ProjectDeclaration>,
    depth: usize,
) -> Result<(), ProjectReadError> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(ProjectReadError::Limit);
    }
    match ty {
        ProjectPublicType::Nominal(identity) => {
            if !index
                .get(identity)
                .is_some_and(|decl| matches!(decl.signature, ProjectPublicSignature::Record(_)))
            {
                return Err(ProjectReadError::Declaration);
            }
        }
        ProjectPublicType::VocabularyNominal {
            identity,
            version,
            name,
        } => {
            if !ir.vocabulary_records.iter().any(|record| {
                record.identity == *identity
                    && record.version == *version
                    && record.name == *name
                    && record.public
            }) {
                return Err(ProjectReadError::Declaration);
            }
        }
        ProjectPublicType::List(inner) | ProjectPublicType::Ref(inner) => {
            check_type(inner, ir, index, depth + 1)?;
        }
        ProjectPublicType::Nullable(inner) => {
            if matches!(inner.as_ref(), ProjectPublicType::Nullable(_)) {
                return Err(ProjectReadError::Declaration);
            }
            check_type(inner, ir, index, depth + 1)?;
        }
        _ => {}
    }
    Ok(())
}

/// Validates materialized contextual values and public reference visibility.
#[expect(
    clippy::too_many_arguments,
    reason = "explicit validation context and budget prevent ambient or unbounded work"
)]
fn check_value(
    value: &ProjectValue,
    ty: &ProjectPublicType,
    public: bool,
    closed: bool,
    ir: &ProjectIr,
    index: &BTreeMap<&ModuleSymbolIdentity, &ProjectDeclaration>,
    budget: &mut ValueBudget<'_>,
    depth: usize,
) -> Result<(), ProjectReadError> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(ProjectReadError::Limit);
    }
    let ty = if let ProjectPublicType::Nullable(inner) = ty {
        if matches!(value, ProjectValue::Null) {
            budget.charge(0)?;
            return Ok(());
        }
        inner.as_ref()
    } else {
        ty
    };
    budget.charge(match value {
        ProjectValue::Number(number) => number.coefficient().len(),
        ProjectValue::String(text) | ProjectValue::Url(text) | ProjectValue::Path(text) => {
            text.len()
        }
        _ => 0,
    })?;
    match (value, ty) {
        (ProjectValue::Number(number), ProjectPublicType::Num) => {
            if number.scale().unsigned_abs() > budget.limits.text_bytes {
                return Err(ProjectReadError::Limit);
            }
        }
        (ProjectValue::String(_), ProjectPublicType::String)
        | (ProjectValue::Bool(_), ProjectPublicType::Bool)
        | (ProjectValue::Url(_), ProjectPublicType::Url)
        | (ProjectValue::Path(_), ProjectPublicType::Path) => {}
        (ProjectValue::List(values), ProjectPublicType::List(inner)) => {
            for value in values {
                check_value(value, inner, public, closed, ir, index, budget, depth + 1)?;
            }
        }
        (ProjectValue::Reference(target), ProjectPublicType::Ref(inner)) if !closed => {
            let decl = index.get(target).ok_or(ProjectReadError::Declaration)?;
            if (public && !decl.public)
                || decl.signature != ProjectPublicSignature::Binding(inner.as_ref().clone())
            {
                return Err(ProjectReadError::Declaration);
            }
        }
        (ProjectValue::Record(values), ProjectPublicType::Nominal(identity)) => {
            let ProjectPublicSignature::Record(fields) = &index[identity].signature else {
                return Err(ProjectReadError::Declaration);
            };
            let fields = fields
                .iter()
                .map(|field| (field.name(), field.ty()))
                .collect::<Vec<_>>();
            check_record(
                values,
                &fields,
                public,
                closed,
                ir,
                index,
                budget,
                depth + 1,
            )?;
        }
        (
            ProjectValue::Record(values),
            ProjectPublicType::VocabularyNominal {
                identity,
                version,
                name,
            },
        ) => {
            let record = ir
                .vocabulary_records
                .iter()
                .find(|record| {
                    record.identity == *identity
                        && record.version == *version
                        && record.name == *name
                })
                .ok_or(ProjectReadError::Declaration)?;
            let fields = record
                .fields
                .iter()
                .map(|(name, ty)| (name.as_str(), ty))
                .collect::<Vec<_>>();
            check_record(
                values,
                &fields,
                public,
                closed,
                ir,
                index,
                budget,
                depth + 1,
            )?;
        }
        _ => return Err(ProjectReadError::Declaration),
    }
    Ok(())
}

/// Checks exact contextual field coverage, order, and compatibility.
#[expect(
    clippy::too_many_arguments,
    reason = "shares the explicit bounded value-validation context"
)]
fn check_record(
    values: &[(String, ProjectValue)],
    fields: &[(&str, &ProjectPublicType)],
    public: bool,
    closed: bool,
    ir: &ProjectIr,
    index: &BTreeMap<&ModuleSymbolIdentity, &ProjectDeclaration>,
    budget: &mut ValueBudget<'_>,
    depth: usize,
) -> Result<(), ProjectReadError> {
    if values.len() != fields.len() {
        return Err(ProjectReadError::Declaration);
    }
    for ((name, value), (field, ty)) in values.iter().zip(fields) {
        if name != field {
            return Err(ProjectReadError::Declaration);
        }
        check_value(value, ty, public, closed, ir, index, budget, depth)?;
    }
    Ok(())
}

/// Checks complete source-map coverage and bounded, non-dangling provenance.
fn check_companions(
    ir: &ProjectIr,
    index: &BTreeMap<&ModuleSymbolIdentity, &ProjectDeclaration>,
    limits: ProjectLimits,
) -> Result<(), ProjectReadError> {
    if ir.source_maps.len() != ir.declarations.len() {
        return Err(ProjectReadError::Companion);
    }
    if ir.vocabulary_sources.len() != ir.public_interface.vocabularies().len() {
        return Err(ProjectReadError::Companion);
    }
    for (source, vocabulary) in ir
        .vocabulary_sources
        .iter()
        .zip(ir.public_interface.vocabularies())
    {
        if source.identity != vocabulary.identity()
            || source.version != vocabulary.version()
            || source.byte_len == 0
            || source.byte_len > limits.text_bytes
        {
            return Err(ProjectReadError::Companion);
        }
    }
    let mut ids = BTreeSet::new();
    let mut lengths = BTreeMap::new();
    for (source, module) in ir.sources.iter().zip(&ir.modules) {
        if source.module != module.identity.module_name()
            || source.source_id.is_empty()
            || !ids.insert(&source.source_id)
        {
            return Err(ProjectReadError::Companion);
        }
        lengths.insert(source.digest, source.byte_len);
    }
    for (map, decl) in ir.source_maps.iter().zip(&ir.declarations) {
        let source = ir
            .sources
            .iter()
            .find(|source| source.module == decl.identity.module().module_name())
            .ok_or(ProjectReadError::Companion)?;
        if map.declaration != decl.identity
            || map.location.source() != source.digest
            || map.location.span().end() > source.byte_len
            || map.location.span().is_empty()
        {
            return Err(ProjectReadError::Companion);
        }
    }
    let maps = ir
        .source_maps
        .iter()
        .map(|map| (&map.declaration, map.location))
        .collect::<BTreeMap<_, _>>();
    let mut previous = None;
    for edge in &ir.provenance {
        let key = (&edge.from, edge.kind, &edge.to, edge.location);
        let owner = maps.get(&edge.from).ok_or(ProjectReadError::Companion)?;
        if !index.contains_key(&edge.from)
            || !index.contains_key(&edge.to)
            || previous.is_some_and(|prior| prior >= key)
            || lengths
                .get(&edge.location.source())
                .is_none_or(|length| edge.location.span().end() > *length)
            || edge.location.span().is_empty()
            || owner.source() != edge.location.source()
            || edge.location.span().start() < owner.span().start()
            || edge.location.span().end() > owner.span().end()
        {
            return Err(ProjectReadError::Companion);
        }
        previous = Some(key);
    }
    if ir.resources.source_bytes > limits.text_bytes {
        return Err(ProjectReadError::Limit);
    }
    Ok(())
}
