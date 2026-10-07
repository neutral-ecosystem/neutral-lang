// SPDX-License-Identifier: Apache-2.0

//! Independent complete successor validation, before any encoded artifact becomes authority.

use neutral_core::{CancellationToken, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{
        BindingValue as V, CompositionBinding, CompositionBody as B, CompositionDefinition,
        FieldRestrictions, SourceCompositionDefinition, ValueOriginKind as K,
        ValuePathSegment as P, profile,
        project::{
            CompositionAttribution as A, CompositionProjectIr, CompositionSignature as S,
            inspect_composition_ir,
        },
    },
    language::is_snake_name,
    project::{ProjectLimits, ProjectResourceFacts},
    project_identity::{IdentityLimits, composition_interface},
    project_interface::{ProjectPublicEdgeKind as Edge, ProjectPublicType as T},
};
use neutral_vocabulary::composition::{
    CompositionError, CompositionLimits, ValidatedCompositionBindings,
    validate_composition_bindings, validate_composition_model, validate_composition_scope,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Safe whole-project failures; no supplied text, private names or host paths are retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionReadError {
    /// Unknown successor selector.
    Schema,
    /// Independent work, count, byte, allocation or depth bound.
    Limit,
    /// Invalid module, declaration, type, value or semantic graph.
    Semantic,
    /// Mismatched public interpretive digest.
    Interface,
    /// Missing, malformed, dangling or inconsistent companions.
    Companion,
    /// Claimed retained counts differ from independent inspection.
    Resources,
    /// Cancellation prevented atomic publication.
    Cancelled,
}
impl CompositionReadError {
    /// Returns the successor result envelope, not a package release number.
    #[must_use]
    pub const fn schema(self) -> &'static str {
        profile::PROJECT_RESULT_SCHEMA
    }
}
/// Short complete-reader failure name used by private checks.
type E = CompositionReadError;

/// Immutable complete authority; construction requires independent revalidation of producer data.
#[derive(Clone)]
pub struct ValidatedCompositionProject {
    /// Complete data; never exposed by public-only catalogue projections.
    ir: Arc<CompositionProjectIr>,
    /// Independently validated interpretation and actual reference paths.
    bindings: Arc<ValidatedCompositionBindings>,
}
impl std::fmt::Debug for ValidatedCompositionProject {
    /// Logs counts only, avoiding private values or identity keys.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatedCompositionProject")
            .field("modules", &self.ir.modules.len())
            .field("declarations", &self.ir.declarations.len())
            .finish_non_exhaustive()
    }
}
impl ValidatedCompositionProject {
    /// Rechecks complete source/vocabulary semantics and companions without compiler linkage.
    ///
    /// # Errors
    /// Rejects unknown profiles, noncanonical data, invalid defaults/references,
    /// stale facts, companion defects, exhausted bounds or cancellation atomically.
    pub fn from_ir(
        ir: Arc<CompositionProjectIr>,
        caller: ProjectLimits,
        composition: CompositionLimits,
        cancellation: &CancellationToken,
    ) -> Result<Self, E> {
        if cancellation.is_cancelled() {
            return Err(E::Cancelled);
        }
        if ir.schema != profile::PROJECT_IR_SCHEMA {
            return Err(E::Schema);
        }
        let limits = caller.intersect(ir.limits);
        let composition = CompositionLimits::from_policy(
            composition.json,
            composition.policy().intersect(ir.composition_limits),
        );
        composition.validate().map_err(|e| error(&e))?;
        if ir.resources.vocabulary_bytes > composition.captured_bytes {
            return Err(E::Limit);
        }
        let facts = inspect_composition_ir(&ir, limits, composition.policy(), cancellation)
            .map_err(|e| match e {
                neutral_ir::composition::project::CompositionShapeError::Cancelled => E::Cancelled,
                neutral_ir::composition::project::CompositionShapeError::Limit => E::Limit,
            })?;
        if facts != ir.composition_resources {
            return Err(E::Resources);
        }
        check_comparison_work(&ir, composition.policy().work)?;
        modules(&ir)?;
        let catalogue = Arc::new(
            validate_composition_model(&ir.vocabularies, composition, cancellation)
                .map_err(|e| error(&e))?,
        );
        let (sources, mut raw) = declarations(&ir)?;
        let expected_sources = sources.clone();
        let scope = Arc::new(
            validate_composition_scope(catalogue, sources, composition, cancellation)
                .map_err(|e| error(&e))?,
        );
        if scope.sources() != expected_sources {
            return Err(E::Semantic);
        }
        let mut origin_index = BTreeMap::new();
        for origin in &ir.origins {
            if origin_index
                .insert((&origin.binding, origin.path.as_slice()), origin)
                .is_some()
            {
                return Err(E::Companion);
            }
        }
        for binding in &mut raw {
            binding.value = supplied(
                &binding.owner,
                &binding.value,
                &mut Vec::new(),
                &origin_index,
            )?;
        }
        let bindings = Arc::new(
            validate_composition_bindings(scope, &raw, composition, cancellation)
                .map_err(|e| error(&e))?,
        );
        let mut origins = ir.origins.iter();
        for binding in bindings.bindings() {
            let d = ir
                .declarations
                .iter()
                .find(|d| d.identity == binding.binding().owner)
                .ok_or(E::Semantic)?;
            if d.value.as_ref() != Some(&binding.binding().value) {
                return Err(E::Semantic);
            }
            for expected in binding.origins() {
                let actual = origins.next().ok_or(E::Companion)?;
                if actual.binding != d.identity
                    || actual.path != expected.path
                    || actual.kind != expected.kind
                {
                    return Err(E::Companion);
                }
                attribution(actual, &ir)?;
            }
        }
        if origins.next().is_some() {
            return Err(E::Companion);
        }
        companions(&ir, &bindings, limits, cancellation)?;
        check_interface(&ir, composition.work, cancellation)?;
        if cancellation.is_cancelled() {
            return Err(E::Cancelled);
        }
        Ok(Self { ir, bindings })
    }
    /// Returns immutable complete data for trusted consumers, not a public redacted view.
    #[must_use]
    pub const fn complete_ir(&self) -> &Arc<CompositionProjectIr> {
        &self.ir
    }
    /// Returns independently checked contracts and binding-reference paths without source reparsing.
    #[must_use]
    pub const fn bindings(&self) -> &Arc<ValidatedCompositionBindings> {
        &self.bindings
    }
}

/// Independently recomputes the interface identity under the intersected reader budget.
fn check_interface(
    ir: &CompositionProjectIr,
    work: u64,
    cancellation: &CancellationToken,
) -> Result<(), E> {
    let digest = composition_interface(
        ir,
        IdentityLimits {
            // Transcript framing is not encoded artifact size; the two independent bounds cannot be conflated.
            bytes: neutral_ir::project_identity::MAX_TRANSCRIPT_BYTES,
            nodes: work,
        },
        cancellation,
    )
    .map_err(|e| match e {
        neutral_ir::project_identity::IdentityError::Cancelled => E::Cancelled,
        neutral_ir::project_identity::IdentityError::Limit => E::Limit,
        neutral_ir::project_identity::IdentityError::InvalidInput => E::Interface,
    })?
    .identity();
    if ir.interface_digest != digest {
        return Err(E::Interface);
    }
    Ok(())
}

/// Checks canonical complete declaration categories before retaining any semantic clones.
fn declarations(
    ir: &CompositionProjectIr,
) -> Result<(Vec<SourceCompositionDefinition>, Vec<CompositionBinding>), E> {
    let mut sources = Vec::new();
    let mut raw = Vec::new();
    for (i, d) in ir.declarations.iter().enumerate() {
        if i > 0 && ir.declarations[i - 1].identity >= d.identity {
            return Err(E::Semantic);
        }
        if !ir
            .modules
            .iter()
            .any(|m| m.identity == *d.identity.module())
        {
            return Err(E::Semantic);
        }
        match (&d.signature, &d.value) {
            (S::Definition(body), None) => {
                if let B::Record(fields) = body
                    && fields
                        .iter()
                        .any(|f| f.restrictions != FieldRestrictions::default())
                {
                    return Err(E::Semantic);
                }
                sources.push(SourceCompositionDefinition {
                    owner: d.identity.clone(),
                    definition: CompositionDefinition {
                        name: d.identity.declaration_name().to_owned(),
                        public: d.public,
                        body: body.clone(),
                    },
                });
            }
            (S::Binding(ty), Some(value)) => raw.push(CompositionBinding {
                owner: d.identity.clone(),
                public: d.public,
                ty: ty.clone(),
                value: value.clone(),
            }),
            _ => return Err(E::Semantic),
        }
    }
    Ok((sources, raw))
}

/// Converts shared contract failures into a source-independent, non-disclosing envelope.
fn error(e: &CompositionError) -> E {
    match e {
        CompositionError::Cancelled => E::Cancelled,
        CompositionError::Limit
        | CompositionError::Allocation
        | CompositionError::InvalidLimits => E::Limit,
        _ => E::Semantic,
    }
}

/// Checks canonical complete module topology without requiring source text or acquisition.
fn modules(ir: &CompositionProjectIr) -> Result<(), E> {
    if ir.modules.is_empty() || ir.sources.len() != ir.modules.len() {
        return Err(E::Semantic);
    }
    for (i, m) in ir.modules.iter().enumerate() {
        if m.identity.language_behavior_version() != V1_SOURCE_PROFILE
            || !m.identity.module_name().split("::").all(is_snake_name)
            || i > 0 && ir.modules[i - 1].identity >= m.identity
        {
            return Err(E::Semantic);
        }
        for (j, target) in m.imports.iter().enumerate() {
            if target == m.identity.module_name()
                || j > 0 && m.imports[j - 1] >= *target
                || !ir
                    .modules
                    .iter()
                    .any(|m| m.identity.module_name() == target)
            {
                return Err(E::Semantic);
            }
        }
    }
    Ok(())
}

/// Reconstructs explicit input by pruning only claimed default/optional occurrences.
/// Re-materialization below must reproduce both meaning and every classification.
fn supplied(
    owner: &ModuleSymbolIdentity,
    value: &V,
    path: &mut Vec<P>,
    origins: &BTreeMap<
        (&ModuleSymbolIdentity, &[P]),
        &neutral_ir::composition::project::CompositionOrigin,
    >,
) -> Result<V, E> {
    if !origins.contains_key(&(owner, path.as_slice())) {
        return Err(E::Companion);
    }
    Ok(match value {
        V::Record(fields) => {
            if fields.windows(2).any(|f| f[0].0 >= f[1].0) {
                return Err(E::Semantic);
            }
            let mut result = Vec::new();
            for (name, value) in fields {
                path.push(P::Field(name.clone()));
                let origin = origins.get(&(owner, path.as_slice())).ok_or(E::Companion)?;
                if !matches!(origin.kind, K::Defaulted | K::OmittedOptional) {
                    result.push((
                        name.clone(),
                        Some(supplied(
                            owner,
                            value.as_ref().ok_or(E::Companion)?,
                            path,
                            origins,
                        )?),
                    ));
                }
                path.pop();
            }
            V::Record(result)
        }
        V::List(values) => {
            let mut result = Vec::new();
            for (i, v) in values.iter().enumerate() {
                path.push(P::Element(i as u64));
                result.push(supplied(owner, v, path, origins)?);
                path.pop();
            }
            V::List(result)
        }
        V::Variant { tag, payload } => {
            path.push(P::Payload);
            let payload = supplied(owner, payload, path, origins)?;
            path.pop();
            V::Variant {
                tag: tag.clone(),
                payload: Box::new(payload),
            }
        }
        _ => value.clone(),
    })
}

/// Checks supplied source evidence or exact closed-default vocabulary attribution.
fn attribution(
    origin: &neutral_ir::composition::project::CompositionOrigin,
    ir: &CompositionProjectIr,
) -> Result<(), E> {
    match &origin.attribution {
        None => Ok(()),
        Some(A::Source(location)) => {
            let source = ir
                .sources
                .iter()
                .find(|s| s.module == origin.binding.module().module_name())
                .ok_or(E::Companion)?;
            let map = ir
                .source_maps
                .iter()
                .find(|m| m.declaration == origin.binding)
                .ok_or(E::Companion)?;
            if source.digest != location.source()
                || location.span().is_empty()
                || location.span().end() > source.byte_len
                || location.span().start() < map.location.span().start()
                || location.span().end() > map.location.span().end()
            {
                return Err(E::Companion);
            }
            Ok(())
        }
        Some(A::Vocabulary {
            identity,
            version,
            type_name,
            field_name,
            span,
        }) => {
            if origin.kind != K::Defaulted {
                return Err(E::Companion);
            }
            let bundle = ir
                .vocabularies
                .iter()
                .find(|b| b.identity.identity() == identity && b.identity.version() == version)
                .ok_or(E::Companion)?;
            let definition = bundle
                .definitions
                .iter()
                .find(|d| d.name == *type_name)
                .ok_or(E::Companion)?;
            let B::Record(fields) = &definition.body else {
                return Err(E::Companion);
            };
            if !fields
                .iter()
                .any(|f| f.name == *field_name && f.default.is_some())
            {
                return Err(E::Companion);
            }
            let source = ir
                .vocabulary_sources
                .iter()
                .find(|s| s.identity == *identity && s.version == *version)
                .ok_or(E::Companion)?;
            if span.is_some_and(|s| s.is_empty() || s.end() > source.byte_len) {
                return Err(E::Companion);
            }
            // The owner must be on this binding's actual typed path, not merely exist elsewhere.
            if !default_path_matches(ir, origin, (identity, version, type_name, field_name))? {
                return Err(E::Companion);
            }
            Ok(())
        }
    }
}

/// Resolves a nominal body from complete interpretation facts, without aliases.
fn body<'a>(ir: &'a CompositionProjectIr, ty: &T) -> Result<&'a B, E> {
    match ty {
        T::Nominal(owner) => match &ir
            .declarations
            .iter()
            .find(|d| d.identity == *owner)
            .ok_or(E::Semantic)?
            .signature
        {
            S::Definition(body) => Ok(body),
            S::Binding(_) => Err(E::Semantic),
        },
        T::VocabularyNominal {
            identity,
            version,
            name,
        } => ir
            .vocabularies
            .iter()
            .find(|b| b.identity.identity() == identity && b.identity.version() == version)
            .and_then(|b| b.definitions.iter().find(|d| d.name == *name))
            .map(|d| &d.body)
            .ok_or(E::Semantic),
        _ => Err(E::Semantic),
    }
}

/// Binds a vocabulary default annotation to an actual traversed field prefix.
fn default_path_matches(
    ir: &CompositionProjectIr,
    origin: &neutral_ir::composition::project::CompositionOrigin,
    owner: (&str, &str, &str, &str),
) -> Result<bool, E> {
    let declaration = ir
        .declarations
        .iter()
        .find(|d| d.identity == origin.binding)
        .ok_or(E::Semantic)?;
    let S::Binding(initial) = &declaration.signature else {
        return Err(E::Semantic);
    };
    let mut ty = initial;
    let mut value = declaration.value.as_ref();
    for segment in &origin.path {
        while let T::Nullable(inner) = ty {
            ty = inner;
        }
        match segment {
            P::Field(name) => {
                let B::Record(fields) = body(ir, ty)? else {
                    return Err(E::Companion);
                };
                let field = fields
                    .iter()
                    .find(|f| f.name == *name)
                    .ok_or(E::Companion)?;
                if matches!(ty,T::VocabularyNominal {identity,version,name} if (identity.as_str(),version.as_str(),name.as_str(),field.name.as_str())==owner)
                    && field.default.is_some()
                {
                    return Ok(true);
                }
                ty = &field.ty;
                value = match value {
                    Some(V::Record(fields)) => fields
                        .iter()
                        .find(|(n, _)| n == name)
                        .and_then(|(_, v)| v.as_ref()),
                    _ => None,
                };
            }
            P::Element(index) => {
                let T::List(inner) = ty else {
                    return Err(E::Companion);
                };
                ty = inner;
                value = match value {
                    Some(V::List(values)) => {
                        usize::try_from(*index).ok().and_then(|i| values.get(i))
                    }
                    _ => None,
                };
            }
            P::Payload => {
                let B::Variant(alternatives) = body(ir, ty)? else {
                    return Err(E::Companion);
                };
                let Some(V::Variant { tag, payload }) = value else {
                    return Err(E::Companion);
                };
                ty = &alternatives
                    .iter()
                    .find(|a| a.tag == *tag)
                    .ok_or(E::Companion)?
                    .ty;
                value = Some(payload);
            }
        }
    }
    Ok(false)
}

/// Validates source coverage, dependency provenance and independently recomputed base facts.
fn companions(
    ir: &CompositionProjectIr,
    bindings: &ValidatedCompositionBindings,
    limits: ProjectLimits,
    cancel: &CancellationToken,
) -> Result<(), E> {
    source_companions(ir)?;
    let index = ir
        .declarations
        .iter()
        .map(|d| (&d.identity, d))
        .collect::<BTreeMap<_, _>>();
    let mut actual = BTreeSet::new();
    let mut previous = None;
    for edge in &ir.provenance {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        let key = (&edge.from, edge.kind, &edge.to, edge.location);
        let map = ir
            .source_maps
            .iter()
            .find(|m| m.declaration == edge.from)
            .ok_or(E::Companion)?;
        if previous.is_some_and(|p| p >= key)
            || !index.contains_key(&edge.to)
            || edge.location.source() != map.location.source()
            || edge.location.span().is_empty()
            || edge.location.span().start() < map.location.span().start()
            || edge.location.span().end() > map.location.span().end()
        {
            return Err(E::Companion);
        }
        previous = Some(key);
        actual.insert((&edge.from, edge.kind, &edge.to));
    }
    let mut expected = BTreeSet::new();
    for d in &ir.declarations {
        let types = match &d.signature {
            S::Binding(ty) => vec![ty],
            S::Definition(B::Record(fields)) => fields.iter().map(|f| &f.ty).collect(),
            S::Definition(B::Variant(alts)) => alts.iter().map(|a| &a.ty).collect(),
        };
        for ty in types {
            type_edges(&d.identity, ty, false, &mut expected);
        }
    }
    for b in bindings.bindings() {
        for r in b.references() {
            expected.insert((&b.binding().owner, Edge::Reference, &r.target));
        }
    }
    if expected
        .iter()
        .any(|(from, kind, to)| !actual.contains(&(from, *kind, to)))
    {
        return Err(E::Companion);
    }
    for (from, kind, to) in &actual {
        if *kind != Edge::Value && !expected.contains(&(from, *kind, to)) {
            return Err(E::Companion);
        }
        if from.module() != to.module()
            && (!index.get(to).ok_or(E::Companion)?.public
                || !reachable(
                    ir,
                    from.module().module_name(),
                    to.module().module_name(),
                    cancel,
                )?)
        {
            return Err(E::Semantic);
        }
        if *kind == Edge::Value {
            let left = index.get(from).ok_or(E::Companion)?;
            let right = index.get(to).ok_or(E::Companion)?;
            let (S::Binding(a), S::Binding(b), Some(value), Some(target)) =
                (&left.signature, &right.signature, &left.value, &right.value)
            else {
                return Err(E::Semantic);
            };
            if !contains_reuse(ir, value, a, target, b)? {
                return Err(E::Semantic);
            }
        }
    }
    // Ordinary reuse evaluates; identity references deliberately do not enter this cycle check.
    check_reuse_cycles(&actual, cancel)?;
    resource_facts(ir, limits)
}

/// Recomputes aggregate captured and retained counts instead of trusting advertised facts.
fn resource_facts(ir: &CompositionProjectIr, limits: ProjectLimits) -> Result<(), E> {
    let sum = |values: Vec<u64>| {
        values
            .into_iter()
            .try_fold(0_u64, u64::checked_add)
            .ok_or(E::Limit)
    };
    let facts = ProjectResourceFacts {
        source_units: ir.sources.len() as u64,
        source_bytes: sum(ir.sources.iter().map(|s| s.byte_len).collect())?,
        vocabulary_units: ir.vocabulary_sources.len() as u64,
        vocabulary_bytes: sum(ir.vocabulary_sources.iter().map(|s| s.byte_len).collect())?,
        declarations: ir.declarations.len() as u64,
        import_edges: sum(ir.modules.iter().map(|m| m.imports.len() as u64).collect())?,
        value_nodes: ir.composition_resources.retained_value_nodes,
    };
    if facts != ir.resources {
        return Err(E::Resources);
    }
    if facts.source_bytes > limits.text_bytes
        || facts.vocabulary_bytes > ir.composition_limits.captured_bytes
    {
        return Err(E::Limit);
    }
    Ok(())
}

/// Checks source accounting, original-byte coverage and exact vocabulary evidence ownership.
fn source_companions(ir: &CompositionProjectIr) -> Result<(), E> {
    if ir.source_maps.len() != ir.declarations.len()
        || ir.vocabulary_sources.len() != ir.vocabularies.len()
    {
        return Err(E::Companion);
    }
    let mut ids = BTreeSet::new();
    for (s, m) in ir.sources.iter().zip(&ir.modules) {
        if s.module != m.identity.module_name()
            || s.source_id.is_empty()
            || !ids.insert(&s.source_id)
            || s.byte_len == 0
        {
            return Err(E::Companion);
        }
    }
    for (s, b) in ir.vocabulary_sources.iter().zip(&ir.vocabularies) {
        if s.identity != b.identity.identity()
            || s.version != b.identity.version()
            || s.digest != b.identity.content_digest()
            || s.byte_len == 0
        {
            return Err(E::Companion);
        }
    }
    for (map, d) in ir.source_maps.iter().zip(&ir.declarations) {
        let s = ir
            .sources
            .iter()
            .find(|s| s.module == d.identity.module().module_name())
            .ok_or(E::Companion)?;
        if map.declaration != d.identity
            || map.location.source() != s.digest
            || map.location.span().is_empty()
            || map.location.span().end() > s.byte_len
        {
            return Err(E::Companion);
        }
    }
    Ok(())
}

/// Bounds conservative comparison/lookup work before repeated scans of producer-owned data.
fn check_comparison_work(ir: &CompositionProjectIr, limit: u64) -> Result<(), E> {
    let declarations = ir.declarations.len() as u64;
    let modules = ir.modules.len() as u64;
    let facts = ir.composition_resources;
    let size = declarations
        .checked_add(modules)
        .and_then(|n| n.checked_add(facts.types))
        .and_then(|n| n.checked_add(facts.fields))
        .and_then(|n| n.checked_add(facts.alternatives))
        .and_then(|n| n.checked_add(facts.retained_value_nodes))
        .and_then(|n| n.checked_add(1))
        .ok_or(E::Limit)?;
    let paths = ir
        .origins
        .iter()
        .try_fold(0_u64, |n, o| n.checked_add(o.path.len() as u64 + 1))
        .ok_or(E::Limit)?;
    let scans = paths
        .checked_add(ir.provenance.len() as u64)
        .and_then(|n| n.checked_add(declarations))
        .and_then(|n| n.checked_mul(size))
        .ok_or(E::Limit)?;
    if scans > limit {
        return Err(E::Limit);
    }
    Ok(())
}

/// Proves a reuse edge points to an invariant typed materialized occurrence, including nested fields/lists.
fn contains_reuse(
    ir: &CompositionProjectIr,
    value: &V,
    ty: &T,
    target: &V,
    target_ty: &T,
) -> Result<bool, E> {
    if ty == target_ty && value == target {
        return Ok(true);
    }
    let ty = if let T::Nullable(inner) = ty {
        inner.as_ref()
    } else {
        ty
    };
    match (ty, value) {
        (T::List(inner), V::List(values)) => {
            for v in values {
                if contains_reuse(ir, v, inner, target, target_ty)? {
                    return Ok(true);
                }
            }
        }
        (T::Nominal(_) | T::VocabularyNominal { .. }, V::Record(values)) => {
            let B::Record(fields) = body(ir, ty)? else {
                return Err(E::Semantic);
            };
            for ((name, value), field) in values.iter().zip(fields) {
                if name != &field.name {
                    return Err(E::Semantic);
                }
                if let Some(v) = value
                    && contains_reuse(ir, v, &field.ty, target, target_ty)?
                {
                    return Ok(true);
                }
            }
        }
        (T::Nominal(_) | T::VocabularyNominal { .. }, V::Variant { tag, payload }) => {
            let B::Variant(alternatives) = body(ir, ty)? else {
                return Err(E::Semantic);
            };
            let alternative = alternatives
                .iter()
                .find(|a| a.tag == *tag)
                .ok_or(E::Semantic)?;
            return contains_reuse(ir, payload, &alternative.ty, target, target_ty);
        }
        _ => {}
    }
    Ok(false)
}

/// Checks ordinary value dependencies in linear graph work; identity-only references may cycle.
fn check_reuse_cycles(
    edges: &BTreeSet<(&ModuleSymbolIdentity, Edge, &ModuleSymbolIdentity)>,
    cancel: &CancellationToken,
) -> Result<(), E> {
    let mut incoming = BTreeMap::new();
    let mut outgoing: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (from, kind, to) in edges {
        if *kind == Edge::Value {
            incoming.entry(*from).or_insert(0_usize);
            *incoming.entry(*to).or_insert(0) += 1;
            outgoing.entry(*from).or_default().push(*to);
        }
    }
    let mut ready = incoming
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(key, _)| *key)
        .collect::<Vec<_>>();
    let mut count = 0;
    while let Some(node) = ready.pop() {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        count += 1;
        if let Some(targets) = outgoing.get(node) {
            for target in targets {
                let n = incoming.get_mut(target).ok_or(E::Semantic)?;
                *n -= 1;
                if *n == 0 {
                    ready.push(*target);
                }
            }
        }
    }
    if count != incoming.len() {
        return Err(E::Semantic);
    }
    Ok(())
}

/// Enumerates every source nominal dependency, including unselected alternatives.
fn type_edges<'a>(
    from: &'a ModuleSymbolIdentity,
    ty: &'a T,
    reference: bool,
    edges: &mut BTreeSet<(&'a ModuleSymbolIdentity, Edge, &'a ModuleSymbolIdentity)>,
) {
    match ty {
        T::Nominal(to) => {
            edges.insert((
                from,
                if reference {
                    Edge::ReferenceType
                } else {
                    Edge::Type
                },
                to,
            ));
        }
        T::Ref(inner) => type_edges(from, inner, true, edges),
        T::List(inner) | T::Nullable(inner) => type_edges(from, inner, reference, edges),
        _ => {}
    }
}

/// Checks logical reachability without acquiring imports or interpreting host locations.
fn reachable(
    ir: &CompositionProjectIr,
    from: &str,
    to: &str,
    cancel: &CancellationToken,
) -> Result<bool, E> {
    let mut stack = vec![from];
    let mut seen = BTreeSet::new();
    while let Some(name) = stack.pop() {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        if name == to {
            return Ok(true);
        }
        if !seen.insert(name) {
            continue;
        }
        let module = ir
            .modules
            .iter()
            .find(|m| m.identity.module_name() == name)
            .ok_or(E::Semantic)?;
        stack.extend(module.imports.iter().map(String::as_str));
    }
    Ok(false)
}
