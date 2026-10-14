// SPDX-License-Identifier: Apache-2.0

//! Fail-closed envelope checks; borrowed payloads keep validated project authority intact.

use super::{
    ConsumerDiagnostic, ConsumerDiagnosticCode, ConsumerError, ConsumerLimits, ConsumerLocation,
    ConsumerProtocol, DESCRIPTION_SCHEMA, DescriptionRequest, REPORT_SCHEMA, ReportRequest,
    ValidatedConsumerDescription, ValidatedConsumerReport,
};
use crate::composition::CompositionLogicalIdentity;
use crate::composition::{CompositionAttribution, ValidatedCompositionProject};
use neutral_core::{CancellationToken, allocation::RetainCapacity};
use neutral_ir::{
    ModuleSymbolIdentity, composition::ValuePathSegment, project_interface::ProjectPublicType,
};
use neutral_ir::{
    composition::CompositionBinding,
    project_identity::{
        IdentityError, IdentityLimits, MAX_TRANSCRIPT_NODES, canonical_composition_project,
    },
};

/// Explicit registration over a complete validated project; no consumer code runs here.
pub struct ConsumerBoundary<'p, 'a> {
    /// Independently checked complete project authority.
    pub(super) project: &'p ValidatedCompositionProject,
    /// Explicit consumer registration; never invokes a callback.
    protocol: ConsumerProtocol<'a>,
    /// Positive caller controls intersected with hard ceilings.
    pub(super) limits: ConsumerLimits,
    /// Complete logical identity computed once, without consumer context.
    identity: CompositionLogicalIdentity,
}

impl<'p, 'a> ConsumerBoundary<'p, 'a> {
    /// Checks registration and computes complete logical identity with bounded fallible work.
    ///
    /// # Errors
    /// Rejects malformed policy/registration, unavailable public contracts and exhausted work.
    pub fn new(
        project: &'p ValidatedCompositionProject,
        protocol: ConsumerProtocol<'a>,
        limits: ConsumerLimits,
        cancellation: &CancellationToken,
    ) -> Result<Self, ConsumerError> {
        let limits = limits.effective(project.effective_composition_limits().work)?;
        let mut budget = Budget::new(limits, cancellation);
        budget.list(protocol.supported_features, limits.features)?;
        budget.list(protocol.diagnostic_codes, limits.diagnostics)?;
        if protocol
            .diagnostic_codes
            .iter()
            .any(|code| code.starts_with("NEU-"))
        {
            return Err(ConsumerError::InvalidInput);
        }
        public_type(project, protocol.report_contract, &mut budget)?;
        let identity = canonical_composition_project(
            project.complete_ir(),
            IdentityLimits {
                bytes: limits.bytes,
                nodes: limits.work.min(MAX_TRANSCRIPT_NODES),
            },
            cancellation,
        )
        .map_err(|error| match error {
            IdentityError::Cancelled => ConsumerError::Cancelled,
            IdentityError::Limit => ConsumerError::Limit,
            IdentityError::InvalidInput => ConsumerError::IncompleteContract,
        })?
        .identity();
        Ok(Self {
            project,
            protocol,
            limits,
            identity,
        })
    }

    /// Returns complete project identity; external context and report roots cannot alter it.
    #[must_use]
    pub const fn project_identity(&self) -> CompositionLogicalIdentity {
        self.identity
    }

    /// Checks an exact nominal public payload and explicitly negotiated required features.
    ///
    /// # Errors
    /// Fails closed for schema/features, private targets, type mismatch, bounds or cancellation.
    pub fn description<'r>(
        &self,
        request: &'r DescriptionRequest<'r>,
        cancellation: &CancellationToken,
    ) -> Result<ValidatedConsumerDescription<'p, 'r>, ConsumerError> {
        let mut budget = Budget::new(self.limits, cancellation);
        budget.text(request.schema)?;
        if request.schema != DESCRIPTION_SCHEMA {
            return Err(ConsumerError::Unsupported);
        }
        budget.text(request.adapter_revision)?;
        budget.text(request.target_revision)?;
        budget.list(request.required_features, self.limits.features)?;
        for feature in request.required_features {
            let mut supported = false;
            for candidate in self.protocol.supported_features {
                budget.work(candidate.len() as u64 + 1)?;
                supported |= candidate == feature;
            }
            if !supported {
                return Err(ConsumerError::Unsupported);
            }
        }
        public_type(self.project, request.contract, &mut budget)?;
        let payload = binding(self.project, request.payload, &mut budget)?;
        if &payload.ty != request.contract {
            return Err(ConsumerError::TypeMismatch);
        }
        budget.work(1)?;
        Ok(ValidatedConsumerDescription { request, payload })
    }

    /// Validates the entire report before publishing a canonical borrowed diagnostic index.
    ///
    /// Validation authenticates neither the consumer's decision nor producer honesty.
    /// # Errors
    /// Rejects wrong project/context/contracts, private subjects, fabricated evidence and limits.
    pub fn report<'r>(
        &self,
        request: &'r ReportRequest<'r>,
        cancellation: &CancellationToken,
    ) -> Result<ValidatedConsumerReport<'r>, ConsumerError> {
        let mut budget = Budget::new(self.limits, cancellation);
        budget.text(request.schema)?;
        if request.schema != REPORT_SCHEMA {
            return Err(ConsumerError::Unsupported);
        }
        public_type(self.project, request.contract, &mut budget)?;
        if request.contract != self.protocol.report_contract {
            return Err(ConsumerError::TypeMismatch);
        }
        if request.project_identity != self.identity {
            return Err(ConsumerError::InvalidInput);
        }
        budget.text(request.context.adapter_revision)?;
        budget.text(request.context.target_revision)?;
        budget.text(request.context.input_revision)?;
        if let Some(entry) = request.entry {
            binding(self.project, entry, &mut budget)?;
        }
        let count = request.diagnostics.len() as u64;
        if count > self.limits.diagnostics {
            return Err(ConsumerError::Limit);
        }
        let mut locations = 0_u64;
        for diagnostic in request.diagnostics {
            binding(self.project, diagnostic.subject, &mut budget)?;
            occurrence(
                self.project,
                diagnostic.subject,
                diagnostic.path,
                &mut budget,
            )?;
            budget.text(diagnostic.code.code())?;
            if let ConsumerDiagnosticCode::Consumer(code) = diagnostic.code {
                let mut registered = false;
                for candidate in self.protocol.diagnostic_codes {
                    budget.work(candidate.len() as u64 + 1)?;
                    registered |= code == *candidate;
                }
                if !registered {
                    return Err(ConsumerError::InvalidInput);
                }
            }
            locations = locations
                .checked_add(diagnostic.related.len() as u64)
                .and_then(|n| n.checked_add(1))
                .ok_or(ConsumerError::Limit)?;
            if locations > self.limits.locations {
                return Err(ConsumerError::Limit);
            }
            if let ConsumerLocation::Available { subject, path, .. } = diagnostic.primary {
                budget.symbol(subject)?;
                budget.path(path)?;
                if subject != diagnostic.subject || path != diagnostic.path {
                    return Err(ConsumerError::InvalidAttribution);
                }
            }
            location(self.project, diagnostic.primary, &mut budget)?;
            for related in diagnostic.related {
                location(self.project, *related, &mut budget)?;
            }
        }
        let output_bytes = count
            .checked_mul(std::mem::size_of::<&ConsumerDiagnostic<'_>>() as u64)
            .ok_or(ConsumerError::Limit)?;
        if output_bytes > self.limits.output_bytes {
            return Err(ConsumerError::Limit);
        }
        // Charge an upper bound for comparisons before allocation-free unstable sorting.
        budget.work(
            budget
                .bytes
                .checked_add(count)
                .and_then(|n| n.checked_mul(u64::from(count.bit_width()) + 1))
                .ok_or(ConsumerError::Limit)?,
        )?;
        let mut diagnostics: Vec<&ConsumerDiagnostic<'_>> = Vec::new();
        diagnostics
            .try_retain_exact(request.diagnostics.len())
            .map_err(|_| ConsumerError::Limit)?;
        diagnostics.extend(request.diagnostics);
        diagnostics.sort_unstable_by(|a, b| {
            (a.code.phase(), a.subject, a.path, a.code.code()).cmp(&(
                b.code.phase(),
                b.subject,
                b.path,
                b.code.code(),
            ))
        });
        budget.work(1)?;
        Ok(ValidatedConsumerReport {
            request,
            diagnostics,
        })
    }
}

/// Per-request accounting; existing validated payload storage is borrowed, not rematerialized.
pub(super) struct Budget<'a> {
    /// Effective independent request controls.
    limits: ConsumerLimits,
    /// Request-local signal checked before every bounded operation.
    cancellation: &'a CancellationToken,
    /// Conservative aggregate envelope size, excluding borrowed authority scans.
    bytes: u64,
    /// Charged validation and comparison work.
    work: u64,
}
impl<'a> Budget<'a> {
    /// Preserves already charged work across a dependent public projection.
    pub(super) fn remaining_work(&self) -> u64 {
        self.limits.work - self.work
    }
    /// Starts independent accounting for one atomic request.
    pub(super) fn new(limits: ConsumerLimits, cancellation: &'a CancellationToken) -> Self {
        Self {
            limits,
            cancellation,
            bytes: 0,
            work: 0,
        }
    }
    /// Checks cancellation and charges work before traversal or comparison.
    pub(super) fn work(&mut self, count: u64) -> Result<(), ConsumerError> {
        if self.cancellation.is_cancelled() {
            return Err(ConsumerError::Cancelled);
        }
        self.work = self.work.checked_add(count).ok_or(ConsumerError::Limit)?;
        if self.work > self.limits.work {
            return Err(ConsumerError::Limit);
        }
        Ok(())
    }
    /// Charges aggregate envelope data before retention or expansion.
    pub(super) fn bytes(&mut self, count: u64) -> Result<(), ConsumerError> {
        self.work(1)?;
        self.bytes = self.bytes.checked_add(count).ok_or(ConsumerError::Limit)?;
        if self.bytes > self.limits.bytes {
            return Err(ConsumerError::Limit);
        }
        Ok(())
    }
    /// Bounds a public identifier; caller ownership, not heuristics, excludes secrets.
    pub(super) fn text(&mut self, text: &str) -> Result<(), ConsumerError> {
        self.work(text.len() as u64 + 1)?;
        if text.len() as u64 > self.limits.text_bytes {
            return Err(ConsumerError::Limit);
        }
        self.bytes(text.len() as u64 + 8)?;
        if text.is_empty() || text.chars().any(char::is_control) {
            return Err(ConsumerError::InvalidInput);
        }
        Ok(())
    }
    /// Checks every identity component before comparing it with reader authority.
    pub(super) fn symbol(&mut self, symbol: &ModuleSymbolIdentity) -> Result<(), ConsumerError> {
        self.text(symbol.module().language_behavior_version())?;
        self.text(symbol.module().module_name())?;
        self.text(symbol.declaration_name())
    }
    /// Charges comparisons against already checked project keys without retaining them.
    pub(super) fn authority_symbol(
        &mut self,
        symbol: &ModuleSymbolIdentity,
    ) -> Result<(), ConsumerError> {
        self.work(symbol.module().language_behavior_version().len() as u64)?;
        self.work(symbol.module().module_name().len() as u64)?;
        self.work(symbol.declaration_name().len() as u64 + 1)
    }
    /// Bounds typed path depth and textual field components before any lookup.
    fn path(&mut self, path: &[ValuePathSegment]) -> Result<(), ConsumerError> {
        self.work(path.len() as u64 + 1)?;
        if path.len() as u64 > self.limits.path_segments {
            return Err(ConsumerError::Limit);
        }
        self.bytes(
            (path.len() as u64)
                .checked_mul(16)
                .ok_or(ConsumerError::Limit)?,
        )?;
        for segment in path {
            if let ValuePathSegment::Field(name) = segment {
                self.text(name)?;
            }
        }
        Ok(())
    }
    /// Charges comparison of existing project paths without applying request retention bounds.
    fn authority_path(&mut self, path: &[ValuePathSegment]) -> Result<(), ConsumerError> {
        self.work(path.len() as u64 + 1)?;
        for segment in path {
            if let ValuePathSegment::Field(name) = segment {
                self.work(name.len() as u64)?;
            }
        }
        Ok(())
    }
    /// Validates a closed canonical registry or feature request without allocating.
    pub(super) fn list(&mut self, values: &[&str], maximum: u64) -> Result<(), ConsumerError> {
        self.work(1)?;
        if values.len() as u64 > maximum {
            return Err(ConsumerError::Limit);
        }
        for value in values {
            self.text(value)?;
        }
        if values.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ConsumerError::InvalidInput);
        }
        Ok(())
    }
}

/// Looks up a public binding while keeping absent and private targets indistinguishable.
pub(super) fn binding<'p>(
    project: &'p ValidatedCompositionProject,
    owner: &ModuleSymbolIdentity,
    budget: &mut Budget<'_>,
) -> Result<&'p CompositionBinding, ConsumerError> {
    budget.symbol(owner)?;
    for binding in project.bindings().bindings() {
        let binding = binding.binding();
        budget.authority_symbol(&binding.owner)?;
        if &binding.owner == owner {
            return if binding.public {
                Ok(binding)
            } else {
                Err(ConsumerError::PrivateTarget)
            };
        }
    }
    Err(ConsumerError::PrivateTarget)
}

/// Requires an exact public nominal source or vocabulary contract, never duck typing.
pub(super) fn public_type(
    project: &ValidatedCompositionProject,
    ty: &ProjectPublicType,
    budget: &mut Budget<'_>,
) -> Result<(), ConsumerError> {
    let scope = project.bindings().scope();
    public_scope_type(scope, ty, budget)
}

/// Shares exact public nominal-contract checks without conflating artifact profiles.
pub(super) fn public_scope_type(
    scope: &neutral_vocabulary::composition::ValidatedCompositionScope,
    ty: &ProjectPublicType,
    budget: &mut Budget<'_>,
) -> Result<(), ConsumerError> {
    match ty {
        ProjectPublicType::Nominal(owner) => {
            budget.symbol(owner)?;
            for source in scope.sources() {
                budget.authority_symbol(&source.owner)?;
                if &source.owner == owner && source.definition.public {
                    return Ok(());
                }
            }
        }
        ProjectPublicType::VocabularyNominal {
            identity,
            version,
            name,
        } => {
            budget.text(identity)?;
            budget.text(version)?;
            budget.text(name)?;
            for bundle in scope.catalogue().bundles() {
                budget.work(bundle.identity.identity().len() as u64 + 1)?;
                budget.work(bundle.identity.version().len() as u64 + 1)?;
                if bundle.identity.identity() == identity && bundle.identity.version() == version {
                    for definition in &bundle.definitions {
                        budget.work(definition.name.len() as u64 + 1)?;
                        if &definition.name == name && definition.public {
                            return Ok(());
                        }
                    }
                }
            }
        }
        _ => return Err(ConsumerError::TypeMismatch),
    }
    Err(ConsumerError::PrivateTarget)
}

/// Validates materialized occurrence membership without implementing selector traversal.
fn occurrence(
    project: &ValidatedCompositionProject,
    subject: &ModuleSymbolIdentity,
    path: &[ValuePathSegment],
    budget: &mut Budget<'_>,
) -> Result<(), ConsumerError> {
    budget.path(path)?;
    if path.is_empty() {
        return Ok(());
    }
    for origin in &project.complete_ir().origins {
        budget.authority_symbol(&origin.binding)?;
        budget.authority_path(&origin.path)?;
        if &origin.binding == subject && origin.path == path {
            return Ok(());
        }
    }
    Err(ConsumerError::MissingMember)
}

/// Checks exact public occurrence ownership, not merely a digest/range in the same source.
fn location(
    project: &ValidatedCompositionProject,
    evidence: ConsumerLocation<'_>,
    budget: &mut Budget<'_>,
) -> Result<(), ConsumerError> {
    budget.work(1)?;
    budget.bytes(64)?;
    let ConsumerLocation::Available {
        subject,
        path,
        location,
    } = evidence
    else {
        return Ok(());
    };
    binding(project, subject, budget).map_err(|error| {
        if error == ConsumerError::PrivateTarget {
            ConsumerError::InvalidAttribution
        } else {
            error
        }
    })?;
    budget.path(path)?;
    public_location(project, location, budget)?;
    // Only evidence owned by this public occurrence can escape, never a private reuse origin.
    if path.is_empty() {
        for map in &project.complete_ir().source_maps {
            budget.authority_symbol(&map.declaration)?;
            if &map.declaration == subject && map.location == location {
                return Ok(());
            }
        }
    }
    for origin in &project.complete_ir().origins {
        budget.authority_symbol(&origin.binding)?;
        budget.authority_path(&origin.path)?;
        if &origin.binding == subject
            && origin.path == path
            && origin.attribution == Some(CompositionAttribution::Source(location))
        {
            return Ok(());
        }
    }
    Err(ConsumerError::InvalidAttribution)
}

/// Reuse evidence can belong to a private declaration even when its consuming binding is public.
pub(super) fn public_location(
    project: &ValidatedCompositionProject,
    location: neutral_core::SourceLocation,
    budget: &mut Budget<'_>,
) -> Result<(), ConsumerError> {
    let ir = project.complete_ir();
    let mut public = false;
    // Complete reader validation guarantees matching canonical declaration/map arrays.
    for (declaration, map) in ir.declarations.iter().zip(&ir.source_maps) {
        budget.work(1)?;
        if map.location.source() != location.source() {
            continue;
        }
        let span = map.location.span();
        let evidence = location.span();
        if declaration.public && span.start() <= evidence.start() && evidence.end() <= span.end() {
            public = true;
        }
        if !declaration.public && span.start() < evidence.end() && evidence.start() < span.end() {
            return Err(ConsumerError::InvalidAttribution);
        }
    }
    if public {
        Ok(())
    } else {
        Err(ConsumerError::InvalidAttribution)
    }
}
