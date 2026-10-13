// SPDX-License-Identifier: Apache-2.0

//! Borrowed typed traversal and privacy-filtered origins; never evaluates or follows references.

use super::validation::{Budget, binding, public_location, public_type};
use super::{ConsumerBoundary, ConsumerError as E, LocationUnavailable};
use neutral_core::{CancellationToken, SourceLocation, allocation::RetainCapacity};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::project::CompositionAttribution,
    composition::{
        BindingValue as V, CompositionBody as Body, FieldPresence, ValueOriginKind,
        ValuePathSegment,
    },
    project_interface::ProjectPublicType as T,
};

/// Frozen typed member request schema, independent of historical wire formats.
pub const MEMBER_SELECTOR_SCHEMA: &str = "neutral.member-selector/1";
/// Frozen occurrence lookup schema.
pub const VALUE_ORIGIN_SCHEMA: &str = "neutral.value-origin/1";

/// Closed selector grammar; alternatives name a tag rather than executing a match.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemberSegment<'a> {
    /// Exact record field name.
    Field(&'a str),
    /// Zero-based list position; the Rust type excludes invalid wire index shapes.
    Index(u32),
    /// Declared closed-variant payload tag.
    Alternative(&'a str),
}

/// Exact public authority; type mode makes no claim about future value availability.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum SelectorRoot<'a> {
    /// Existing public binding.
    Value(&'a ModuleSymbolIdentity),
    /// Exact public source/vocabulary nominal contract.
    Type(&'a T),
}

/// Data-only request; expected type is a check, never lookup authority.
#[derive(Clone, Copy)]
pub struct MemberSelector<'a> {
    /// Must equal [`MEMBER_SELECTOR_SCHEMA`].
    pub schema: &'a str,
    /// Exact public root and lookup mode.
    pub root: SelectorRoot<'a>,
    /// Ordered, bounded typed segments.
    pub path: &'a [MemberSegment<'a>],
    /// Exact expected type, including wrappers and nominal revisions.
    pub expected_type: &'a T,
}

/// Statically valid traversal that still needs consumer availability validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AvailabilityKind {
    /// Future list must contain this position.
    Index,
    /// Future variant must select this tag.
    Alternative,
    /// Optional field must be present.
    Presence,
}

/// Position in the original selector, not a copied string path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AvailabilityObligation {
    /// Zero-based segment index.
    pub segment: usize,
    /// Unresolved availability condition; never an instruction to evaluate.
    pub kind: AvailabilityKind,
}

/// Canonical structural identity tuple; excludes evidence, aliases and graph-local IDs.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SelectorIdentity<'a> {
    /// Frozen schema domain.
    pub schema: &'static str,
    /// Canonical root with explicit value/type mode.
    pub root: SelectorRoot<'a>,
    /// Canonical ordered typed segments.
    pub path: &'a [MemberSegment<'a>],
    /// Independently resolved exact type, not the caller's asserted type.
    pub resolved_type: &'a T,
}

/// Completely validated selection; values and contracts borrow reader authority.
pub struct SelectedMember<'p, 'r> {
    /// Canonical request root and segments.
    request: &'r MemberSelector<'r>,
    /// Independently resolved contract.
    ty: &'p T,
    /// Existing materialized value, absent for type-mode and omitted terminals.
    value: Option<&'p V>,
    /// Fallibly retained future availability conditions.
    obligations: Vec<AvailabilityObligation>,
}

impl SelectedMember<'_, '_> {
    /// Returns the exact resolved type without unwrapping nullable terminals.
    #[must_use]
    pub const fn resolved_type(&self) -> &T {
        self.ty
    }
    /// Returns a retained value only; no deferred evaluation occurs.
    #[must_use]
    pub const fn value(&self) -> Option<&V> {
        self.value
    }
    /// Returns all future availability checks in traversal order.
    #[must_use]
    pub fn obligations(&self) -> &[AvailabilityObligation] {
        &self.obligations
    }
    /// Returns the frozen canonical typed tuple for consumer plan keys.
    #[must_use]
    pub fn identity(&self) -> SelectorIdentity<'_> {
        SelectorIdentity {
            schema: MEMBER_SELECTOR_SCHEMA,
            root: self.request.root,
            path: self.request.path,
            resolved_type: self.ty,
        }
    }
}

impl std::fmt::Debug for SelectedMember<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SelectedMember { .. }")
    }
}

/// Public occurrence request; invalid paths cannot be substituted with declaration spans.
#[derive(Clone, Copy)]
pub struct ValueOriginRequest<'a> {
    /// Must equal [`VALUE_ORIGIN_SCHEMA`].
    pub schema: &'a str,
    /// Exact public binding.
    pub symbol: &'a ModuleSymbolIdentity,
    /// Same typed grammar as value-mode selectors.
    pub path: &'a [MemberSegment<'a>],
}

/// Checked public evidence or explicit absence; hidden evidence is never retained here.
#[derive(Clone, Copy, Debug)]
pub enum PublicOriginLocation {
    /// Checked original-byte source location, not a host path or authentication claim.
    Available(SourceLocation),
    /// Explicit reason with no hidden identifiers or ranges.
    Unavailable(LocationUnavailable),
}

/// Safe origin facts, separate from materialized meaning and source occurrence.
#[derive(Clone, Copy)]
pub enum PublicValueOrigin<'a> {
    /// Producer evidence cannot establish whether the materialized value was reused.
    Unavailable(LocationUnavailable),
    /// Supplied at the consuming occurrence.
    Supplied,
    /// Reuse retains only public evidence; private targets have an explicit redaction.
    Reuse(PublicOriginLocation),
    /// Closed default owner and evidence, never an evaluated expression.
    Default {
        /// Exact public nominal contract whose field supplied the default.
        owner: &'a T,
        /// Canonical default field name, including inherited default children.
        field: &'a str,
        /// Checked source evidence where retained.
        location: PublicOriginLocation,
        /// Checked canonical vocabulary evidence; absent for source defaults.
        vocabulary: Option<&'a CompositionAttribution>,
    },
}

/// Atomic public origin projection; debug output cannot reveal private companion data.
pub struct PublicValueAttribution<'p> {
    /// Independent presence classification.
    presence: ValueOriginKind,
    /// Consuming occurrence only, never a copied value's definition span.
    occurrence: PublicOriginLocation,
    /// Privacy-filtered source/default/reuse facts.
    origin: PublicValueOrigin<'p>,
}
impl<'p> PublicValueAttribution<'p> {
    /// Returns supplied/null/omitted/defaulted independently of logical equality.
    #[must_use]
    pub const fn presence(&self) -> ValueOriginKind {
        self.presence
    }
    /// Returns precise occurrence evidence or an explicit unavailable state.
    #[must_use]
    pub const fn occurrence(&self) -> PublicOriginLocation {
        self.occurrence
    }
    /// Returns the safe origin, never private owner hints.
    #[must_use]
    pub const fn origin(&self) -> PublicValueOrigin<'p> {
        self.origin
    }
}
impl std::fmt::Debug for PublicValueAttribution<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PublicValueAttribution { .. }")
    }
}

/// Private traversal result, borrowing default ownership instead of copying contracts.
struct Traversal<'p> {
    /// Terminal contract.
    ty: &'p T,
    /// Terminal materialized value or absence.
    value: Option<&'p V>,
    /// Ordered future availability conditions.
    obligations: Vec<AvailabilityObligation>,
    /// Actual default owner, distinguishing inherited classification from a newly applied default.
    default: Option<(&'p T, &'p str)>,
}

impl<'p> ConsumerBoundary<'p, '_> {
    /// Resolves public contract members without implicit unwrap, dereference or evaluation.
    ///
    /// # Errors
    /// Rejects unknown/private roots, paths, type mismatch, limits and cancellation atomically.
    pub fn select<'r>(
        &'r self,
        request: &'r MemberSelector<'r>,
        cancellation: &CancellationToken,
    ) -> Result<SelectedMember<'r, 'r>, E> {
        let mut budget = Budget::new(self.limits, cancellation);
        self.output(
            std::mem::size_of::<SelectedMember<'_, '_>>() as u64,
            &mut budget,
        )?;
        budget.text(request.schema)?;
        if request.schema != MEMBER_SELECTOR_SCHEMA {
            return Err(E::Unsupported);
        }
        self.check_type(request.expected_type, &mut budget)?;
        let result = self.traverse(request.root, request.path, &mut budget)?;
        // Both types are bounded before structural equality can recurse.
        if result.ty != request.expected_type {
            return Err(E::TypeMismatch);
        }
        budget.work(1)?;
        Ok(SelectedMember {
            request,
            ty: result.ty,
            value: result.value,
            obligations: result.obligations,
        })
    }

    /// Projects a retained origin after checked public traversal and companion ownership.
    ///
    /// # Errors
    /// Rejects invalid paths/schema/ownership or exhausted bounds; never invents a span.
    pub fn value_origin<'r>(
        &'r self,
        request: &ValueOriginRequest<'r>,
        cancellation: &CancellationToken,
    ) -> Result<PublicValueAttribution<'r>, E> {
        let mut budget = Budget::new(self.limits, cancellation);
        self.output(
            std::mem::size_of::<PublicValueAttribution<'_>>() as u64,
            &mut budget,
        )?;
        budget.text(request.schema)?;
        if request.schema != VALUE_ORIGIN_SCHEMA {
            return Err(E::Unsupported);
        }
        let result = self.traverse(
            SelectorRoot::Value(request.symbol),
            request.path,
            &mut budget,
        )?;
        let origin = self.origin_at(request.symbol, request.path, &mut budget)?;
        let missing = PublicOriginLocation::Unavailable(LocationUnavailable::NotRetained);
        let mut occurrence = missing;
        let mut source_origin = missing;
        let mut reused = false;
        let mut vocabulary = None;
        if let Some(evidence) = &origin.attribution {
            match evidence {
                CompositionAttribution::Source(location) => {
                    let safe = match public_location(self.project, *location, &mut budget) {
                        Ok(()) => PublicOriginLocation::Available(*location),
                        Err(E::InvalidAttribution) => {
                            PublicOriginLocation::Unavailable(LocationUnavailable::Redacted)
                        }
                        Err(error) => return Err(error),
                    };
                    source_origin = safe;
                    // A companion may point at a reused definition; it is not the consuming use site.
                    for map in &self.project.complete_ir().source_maps {
                        budget.authority_symbol(&map.declaration)?;
                        if &map.declaration == request.symbol {
                            reused = map.location.source() != location.source()
                                || map.location.span().start() > location.span().start()
                                || map.location.span().end() < location.span().end();
                            break;
                        }
                    }
                    if !reused && origin.kind != ValueOriginKind::Defaulted {
                        occurrence = safe;
                    }
                }
                CompositionAttribution::Vocabulary { .. } => {
                    vocabulary = Some(evidence);
                }
            }
        }
        if reused && request.path.is_empty() && origin.kind != ValueOriginKind::Defaulted {
            occurrence = self.root_reuse_occurrence(request.symbol, &mut budget)?;
        }
        let public_origin = if origin.kind == ValueOriginKind::Defaulted {
            match result.default {
                Some((owner, field)) => PublicValueOrigin::Default {
                    owner,
                    field,
                    location: source_origin,
                    vocabulary,
                },
                None => PublicValueOrigin::Unavailable(LocationUnavailable::NotRetained),
            }
        } else if reused {
            PublicValueOrigin::Reuse(source_origin)
        } else if origin.attribution.is_none() {
            PublicValueOrigin::Unavailable(LocationUnavailable::NotRetained)
        } else {
            PublicValueOrigin::Supplied
        };
        let locations = if matches!(
            public_origin,
            PublicValueOrigin::Default { .. } | PublicValueOrigin::Reuse(_)
        ) {
            2
        } else {
            1
        };
        if locations > self.limits.locations {
            return Err(E::Limit);
        }
        budget.bytes(locations * 64)?;
        budget.work(1)?;
        Ok(PublicValueAttribution {
            presence: origin.kind,
            occurrence,
            origin: public_origin,
        })
    }

    /// Bounds wrapper recursion and independently checks nominal visibility before comparisons.
    fn check_type(&self, mut ty: &T, budget: &mut Budget<'_>) -> Result<(), E> {
        let mut depth = 0;
        loop {
            budget.work(1)?;
            if depth > self.project.effective_composition_limits().type_depth {
                return Err(E::Limit);
            }
            match ty {
                T::List(inner) | T::Nullable(inner) | T::Ref(inner) => {
                    ty = inner;
                    depth += 1;
                }
                T::Nominal(_) | T::VocabularyNominal { .. } => {
                    return public_type(self.project, ty, budget);
                }
                _ => return Ok(()),
            }
        }
    }

    /// Finds exact public bodies from reader-validated scope without allocating an index.
    fn body(&self, ty: &T, budget: &mut Budget<'_>) -> Result<&'p Body, E> {
        if !matches!(ty, T::Nominal(_) | T::VocabularyNominal { .. }) {
            return Err(E::MissingMember);
        }
        public_type(self.project, ty, budget)?;
        let scope = self.project.bindings().scope();
        match ty {
            T::Nominal(owner) => {
                for source in scope.sources() {
                    budget.authority_symbol(&source.owner)?;
                    if &source.owner == owner {
                        return Ok(&source.definition.body);
                    }
                }
            }
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                for bundle in scope.catalogue().bundles() {
                    budget.work(
                        bundle.identity.identity().len() as u64
                            + bundle.identity.version().len() as u64
                            + 1,
                    )?;
                    if bundle.identity.identity() == identity
                        && bundle.identity.version() == version
                    {
                        for definition in &bundle.definitions {
                            budget.work(definition.name.len() as u64 + 1)?;
                            if &definition.name == name {
                                return Ok(&definition.body);
                            }
                        }
                    }
                }
            }
            _ => return Err(E::MissingMember),
        }
        Err(E::IncompleteContract)
    }

    /// Traverses in lockstep against type and optional value, retaining obligations only for type mode.
    fn traverse<'r>(
        &'r self,
        root: SelectorRoot<'r>,
        path: &[MemberSegment<'_>],
        budget: &mut Budget<'_>,
    ) -> Result<Traversal<'r>, E> {
        budget.work(1)?;
        if path.len() as u64 > self.limits.path_segments {
            return Err(E::Limit);
        }
        budget.bytes((path.len() as u64).checked_mul(16).ok_or(E::Limit)?)?;
        for segment in path {
            match segment {
                MemberSegment::Field(name) | MemberSegment::Alternative(name) => {
                    budget.text(name)?;
                }
                MemberSegment::Index(_) => budget.work(1)?,
            }
        }
        let type_mode = matches!(root, SelectorRoot::Type(_));
        let (ty, value) = match root {
            SelectorRoot::Value(owner) => {
                let b = binding(self.project, owner, budget)?;
                (&b.ty, Some(&b.value))
            }
            SelectorRoot::Type(ty) => {
                public_type(self.project, ty, budget)?;
                (ty, None)
            }
        };
        self.check_type(ty, budget)?;
        let mut result = Traversal {
            ty,
            value,
            obligations: Vec::new(),
            default: None,
        };
        for index in 0..path.len() {
            budget.work(1)?;
            let obligation = self.advance(&mut result, root, &path[..=index], budget)?;
            self.check_type(result.ty, budget)?;
            if type_mode && let Some(kind) = obligation {
                let bytes = (result.obligations.len() as u64 + 1)
                    .checked_mul(std::mem::size_of::<AvailabilityObligation>() as u64)
                    .and_then(|bytes| {
                        bytes.checked_add(std::mem::size_of::<SelectedMember<'_, '_>>() as u64)
                    })
                    .ok_or(E::Limit)?;
                if bytes > self.limits.output_bytes {
                    return Err(E::Limit);
                }
                budget.bytes(std::mem::size_of::<AvailabilityObligation>() as u64)?;
                result
                    .obligations
                    .try_retain_exact(1)
                    .map_err(|_| E::Limit)?;
                result.obligations.push(AvailabilityObligation {
                    segment: index,
                    kind,
                });
            }
            if !type_mode && result.value.is_none() && index + 1 < path.len() {
                return Err(E::MissingMember);
            }
        }
        budget.work(1)?;
        Ok(result)
    }

    /// Advances one segment; nullable/reference wrappers deliberately have no implicit traversal.
    fn advance<'r>(
        &'r self,
        result: &mut Traversal<'r>,
        root: SelectorRoot<'r>,
        prefix: &[MemberSegment<'_>],
        budget: &mut Budget<'_>,
    ) -> Result<Option<AvailabilityKind>, E> {
        let mut ty = result.ty;
        let mut value = result.value;
        let mut obligation = None;
        let segment = prefix.last().ok_or(E::InvalidInput)?;
        match segment {
            MemberSegment::Field(name) => {
                let field = self.field(ty, name, budget)?;
                if let Some(v) = value {
                    let V::Record(values) = v else {
                        return Err(E::MissingMember);
                    };
                    let mut found = None;
                    for (key, v) in values {
                        budget.work(key.len() as u64 + 1)?;
                        if key == name {
                            found = Some(v.as_ref());
                            break;
                        }
                    }
                    value = found.ok_or(E::MissingMember)?;
                }
                if field.presence == FieldPresence::Defaulted
                    && let SelectorRoot::Value(owner) = root
                {
                    let origin = self.origin_at(owner, prefix, budget)?;
                    if origin.kind == ValueOriginKind::Defaulted
                        && self.default_owner(
                            ty,
                            &field.name,
                            origin.attribution.as_ref(),
                            budget,
                        )?
                    {
                        result.default = Some((ty, field.name.as_str()));
                    }
                }
                if field.presence == FieldPresence::Optional {
                    obligation = Some(AvailabilityKind::Presence);
                }
                ty = &field.ty;
            }
            MemberSegment::Index(position) => {
                let T::List(inner) = ty else {
                    return Err(E::MissingMember);
                };
                if let Some(v) = value {
                    let V::List(values) = v else {
                        return Err(E::MissingMember);
                    };
                    value = Some(values.get(*position as usize).ok_or(E::MissingMember)?);
                }
                ty = inner;
                obligation = Some(AvailabilityKind::Index);
            }
            MemberSegment::Alternative(tag) => {
                let Body::Variant(alternatives) = self.body(ty, budget)? else {
                    return Err(E::MissingMember);
                };
                let mut found = None;
                for alternative in alternatives {
                    budget.work(alternative.tag.len() as u64 + 1)?;
                    if alternative.tag == *tag {
                        found = Some(&alternative.ty);
                        break;
                    }
                }
                let selected = found.ok_or(E::MissingMember)?;
                if let Some(v) = value {
                    let V::Variant {
                        tag: actual,
                        payload,
                    } = v
                    else {
                        return Err(E::MissingMember);
                    };
                    budget.work(actual.len() as u64 + 1)?;
                    if actual != tag {
                        return Err(E::MissingMember);
                    }
                    value = Some(payload);
                }
                ty = selected;
                obligation = Some(AvailabilityKind::Alternative);
            }
        }
        result.ty = ty;
        result.value = value;
        Ok(obligation)
    }

    /// Finds a record field only after exact nominal visibility validation.
    fn field(
        &self,
        ty: &T,
        name: &str,
        budget: &mut Budget<'_>,
    ) -> Result<&'p neutral_ir::composition::CompositionField, E> {
        let Body::Record(fields) = self.body(ty, budget)? else {
            return Err(E::MissingMember);
        };
        for field in fields {
            budget.work(field.name.len() as u64 + 1)?;
            if field.name == name {
                return Ok(field);
            }
        }
        Err(E::MissingMember)
    }

    /// Caps retained Rust projection memory; transport owners separately cap serialized bytes.
    fn output(&self, bytes: u64, budget: &mut Budget<'_>) -> Result<(), E> {
        budget.work(1)?;
        if bytes > self.limits.output_bytes {
            return Err(E::Limit);
        }
        budget.bytes(bytes)
    }

    /// A unique direct root-reuse edge retains the real use site, not a declaration-wide approximation.
    fn root_reuse_occurrence(
        &self,
        subject: &ModuleSymbolIdentity,
        budget: &mut Budget<'_>,
    ) -> Result<PublicOriginLocation, E> {
        let mut found = None;
        for edge in &self.project.complete_ir().provenance {
            budget.authority_symbol(&edge.from)?;
            if &edge.from == subject
                && edge.kind == neutral_ir::project_interface::ProjectPublicEdgeKind::Value
            {
                if found.is_some() {
                    return Ok(PublicOriginLocation::Unavailable(
                        LocationUnavailable::NotRetained,
                    ));
                }
                public_location(self.project, edge.location, budget)?;
                found = Some(edge.location);
            }
        }
        Ok(found.map_or(
            PublicOriginLocation::Unavailable(LocationUnavailable::NotRetained),
            PublicOriginLocation::Available,
        ))
    }

    /// Matches checked companions while accounting for every compared symbol/path byte.
    fn origin_at(
        &self,
        owner: &ModuleSymbolIdentity,
        path: &[MemberSegment<'_>],
        budget: &mut Budget<'_>,
    ) -> Result<&'p neutral_ir::composition::project::CompositionOrigin, E> {
        for origin in &self.project.complete_ir().origins {
            budget.authority_symbol(&origin.binding)?;
            budget.work(origin.path.len() as u64 + 1)?;
            for segment in &origin.path {
                if let ValuePathSegment::Field(name) = segment {
                    budget.work(name.len() as u64)?;
                }
            }
            if &origin.binding == owner && same_path(&origin.path, path) {
                return Ok(origin);
            }
        }
        Err(E::InvalidAttribution)
    }

    /// Distinguishes a newly applied inner default from inherited default classification.
    fn default_owner(
        &self,
        ty: &T,
        field: &str,
        evidence: Option<&CompositionAttribution>,
        budget: &mut Budget<'_>,
    ) -> Result<bool, E> {
        budget.work(1)?;
        match (ty, evidence) {
            (
                T::VocabularyNominal {
                    identity,
                    version,
                    name,
                },
                Some(CompositionAttribution::Vocabulary {
                    identity: actual,
                    version: revision,
                    type_name,
                    field_name,
                    ..
                }),
            ) => {
                budget.work(
                    (actual.len() + revision.len() + type_name.len() + field_name.len()) as u64 + 1,
                )?;
                Ok(identity == actual
                    && version == revision
                    && name == type_name
                    && field == field_name)
            }
            (T::Nominal(owner), Some(CompositionAttribution::Source(location))) => {
                for map in &self.project.complete_ir().source_maps {
                    budget.authority_symbol(&map.declaration)?;
                    if &map.declaration == owner {
                        return Ok(map.location.source() == location.source()
                            && map.location.span().start() <= location.span().start()
                            && location.span().end() <= map.location.span().end());
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}

/// Converts no data: variant tag validity was checked against its actual value during traversal.
fn same_path(retained: &[ValuePathSegment], requested: &[MemberSegment<'_>]) -> bool {
    retained.len() == requested.len()
        && retained.iter().zip(requested).all(|(a, b)| match (a, b) {
            (ValuePathSegment::Field(a), MemberSegment::Field(b)) => a == b,
            (ValuePathSegment::Element(a), MemberSegment::Index(b)) => *a == u64::from(*b),
            (ValuePathSegment::Payload, MemberSegment::Alternative(_)) => true,
            _ => false,
        })
}
