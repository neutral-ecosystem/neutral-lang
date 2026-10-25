// SPDX-License-Identifier: Apache-2.0

//! Owned syntax spans share the existing atomic cache; semantics and companions are never cached.

use super::{
    Budget, ByteSpan, E, Lex, MatrixSyntax, ModuleSymbolIdentity, ParameterPhase, Root,
    SourceLocation, State, TemplateSyntax, TypeExpr, alloc, compile_with_parser, ec, error,
    expression_profile::SemanticProfiles, grow, parser,
};
use crate::{CapturedExpressionProject, CapturedProjectSource};
use neutral_core::ordered::OrderedMap;
use neutral_core::{CancellationToken, SourceContentDigest, allocation::Shared};

/// Independent retained-unit and exact-byte budgets for one caller-owned cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectCacheLimits {
    /// Maximum retained parsed source units.
    pub source_units: u64,
    /// Maximum aggregate retained original source bytes.
    pub source_bytes: u64,
}

/// Observed unit parser execution, not captured replay or inferred equivalence.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProjectCacheStats {
    /// Units actually sent through the lexer and declaration parser this run.
    pub parsed_units: u64,
    /// Units reused from the previous successful cache generation.
    pub reused_units: u64,
    /// Existing module entries rejected for changed bytes or identity context.
    pub rejected_entries: u64,
}

/// Private exact-input syntax entry, never accepted from callers or deserialized.
pub(super) struct Entry {
    /// Immutable feature context used by the parser.
    features: Shared<Vec<String>>,
    /// Exact-byte hash is only a fast filter; bytes must also match.
    digest: SourceContentDigest,
    /// Original bytes reject stale entries and digest collisions.
    bytes: Shared<Vec<u8>>,
    /// Current `/3` expression syntax; semantic authority is never cached.
    syntax: Shared<Syntax>,
}

/// Caller-owned, bounded syntax cache with atomic successful generations.
pub struct ExpressionCompilationCache {
    /// Independent retained original-byte/unit controls.
    limits: ProjectCacheLimits,
    /// Previous successful generation in canonical logical-module order.
    entries: OrderedMap<String, Entry>,
    /// Exact total original source bytes retained.
    source_bytes: u64,
}
impl ExpressionCompilationCache {
    /// Creates an empty cache with positive independent retention budgets.
    #[must_use]
    pub fn new(limits: ProjectCacheLimits) -> Option<Self> {
        (limits.source_units != 0 && limits.source_bytes != 0).then(|| Self {
            limits,
            entries: OrderedMap::new(),
            source_bytes: 0,
        })
    }
    /// Returns retained units, not request or semantic-result counts.
    #[must_use]
    pub fn retained_units(&self) -> usize {
        self.entries.len()
    }
    /// Returns exact retained original bytes, not a promise about total heap usage.
    #[must_use]
    pub const fn retained_source_bytes(&self) -> u64 {
        self.source_bytes
    }
}

/// A token keeps only its original byte range; no cached borrow can outlive a capture.
type Tokens = Vec<ByteSpan>;
/// Ordered parameter syntax, with original source-backed name/defaults.
type Parameter = (ByteSpan, ParameterPhase, TypeExpr, Tokens, SourceLocation);
/// Ordinary source bindings never retain resolved values or generated axes.
type Binding = (ModuleSymbolIdentity, bool, Tokens);
/// An owned template definition, not a callable or normalized public contract.
struct Template {
    /// Original declaration and result-type syntax.
    root: Root,
    /// Definition-order parameters.
    parameters: Vec<Parameter>,
    /// Original body tokens.
    body: Tokens,
}
/// An unexpanded matrix, never generated values or topology.
struct Matrix {
    /// Original declaration owner and occurrence.
    root: Root,
    /// Qualified template name tokens.
    template: Tokens,
    /// Application arguments.
    arguments: Tokens,
    /// Ordered source axes.
    axes: Vec<(ByteSpan, Tokens)>,
    /// Static filter syntax.
    filter: Tokens,
    /// Exclusion syntax.
    exclusions: Tokens,
}
/// Borrowed restoration order preserves the clean parser's first failing occurrence.
enum Declaration<'a> {
    /// Ordinary nominal or value declaration.
    Root(&'a Root),
    /// Definition-scoped template.
    Template(&'a Template),
    /// Original unexpanded matrix.
    Matrix(&'a Matrix),
}
impl Declaration<'_> {
    /// Exact source order, independent of canonical symbol-map order.
    fn start(&self) -> u64 {
        match self {
            Self::Root(root) => root.location.span().start(),
            Self::Template(template) => template.root.location.span().start(),
            Self::Matrix(matrix) => matrix.root.location.span().start(),
        }
    }
}
/// Only private source syntax from a fully successful generation may reach this type.
pub(in crate::frontend::project_semantics::composition) struct Syntax {
    /// Exact semantic tuple, independent of package versions.
    profiles: Shared<SemanticProfiles>,
    /// Parser acceptance is policy-dependent even for unused private syntax.
    capture_limits: crate::ProjectCaptureLimits,
    /// Nominal parser and contract controls.
    composition_limits: neutral_vocabulary::composition::CompositionLimits,
    /// Expression syntax/retention controls.
    expression_limits: super::expression_profile::ExpressionLimits,
    /// Private template and matrix grammar controls.
    expansion_limits: neutral_ir::expression::templates::ExpansionLimits,
    /// Ordinary nominal/binding declarations.
    roots: Vec<Root>,
    /// Unresolved initializer/input declarations.
    bindings: Vec<Binding>,
    /// Definition-scoped templates.
    templates: Vec<Template>,
    /// Unexpanded finite matrices.
    matrices: Vec<Matrix>,
    /// Original parse charges are replayed so warm syntax cannot bypass caller controls.
    parse_work: u64,
    /// Original parse storage, separate from fallible cache-copy storage.
    parse_bytes: u64,
}

/// Retains original token spans with a proportional work/storage preflight.
fn freeze(tokens: &[Lex<'_>], budget: &mut Budget<'_>) -> Result<Tokens, E> {
    budget.retain::<ByteSpan>(tokens.len())?;
    let mut result = Vec::new();
    grow(&mut result, tokens.len())?;
    result.extend(tokens.iter().map(|token| token.span));
    Ok(result)
}

/// Captures a source-backed name without a self-reference or unsafe lifetime extension.
fn name_span(source: &CapturedProjectSource, name: &str) -> Result<ByteSpan, E> {
    let start = name
        .as_ptr()
        .addr()
        .checked_sub(source.bytes().as_ptr().addr())
        .ok_or_else(|| error(ec::EXP_LIMIT, None))?;
    let end = start
        .checked_add(name.len())
        .ok_or_else(|| error(ec::EXP_LIMIT, None))?;
    if source.bytes().get(start..end) != Some(name.as_bytes()) {
        return Err(error(ec::EXP_LIMIT, None));
    }
    ByteSpan::new(start as u64, end as u64).map_err(|_| error(ec::EXP_LIMIT, None))
}

/// Resolves a checked private byte range against this request's exact source bytes.
fn spelling(source: &CapturedProjectSource, span: ByteSpan) -> Result<&str, E> {
    let start = usize::try_from(span.start()).map_err(|_| error(ec::EXP_LIMIT, None))?;
    let end = usize::try_from(span.end()).map_err(|_| error(ec::EXP_LIMIT, None))?;
    std::str::from_utf8(
        source
            .bytes()
            .get(start..end)
            .ok_or_else(|| error(ec::EXP_LIMIT, None))?,
    )
    .map_err(|_| error(ec::EXP_SYNTAX, None))
}

/// Rebinds spans to current immutable capture storage; never reparses or evaluates.
fn thaw<'a>(
    source: &'a CapturedProjectSource,
    tokens: &[ByteSpan],
    budget: &mut Budget<'_>,
) -> Result<Vec<Lex<'a>>, E> {
    budget.retain::<Lex<'_>>(tokens.len())?;
    let mut result = Vec::new();
    grow(&mut result, tokens.len())?;
    for span in tokens {
        result.push(Lex {
            spelling: spelling(source, *span)?,
            span: *span,
        });
    }
    Ok(result)
}

impl Syntax {
    /// Snapshots source-only syntax before any resolution, normalization or matrix expansion.
    #[allow(clippy::too_many_lines)]
    fn retain(
        source: &CapturedProjectSource,
        state: &mut State<'_, '_>,
        work: u64,
        bytes: u64,
    ) -> Result<Self, E> {
        let module = source.module_id();
        state.budget.charge(
            (state.resolver.roots.len() + state.templates.len() + state.matrices.len()) as u64 + 1,
        )?;
        let mut roots = Vec::new();
        let mut bindings = Vec::new();
        for (owner, root) in &state.resolver.roots {
            if owner.module().module_name() != module {
                continue;
            }
            state.budget.retain::<Root>(1)?;
            grow(&mut roots, 1)?;
            roots.push(state.budget.copy_root(root)?);
            if let Some(binding) = state.bindings.get(owner) {
                state.budget.retain::<Binding>(1)?;
                grow(&mut bindings, 1)?;
                bindings.push((
                    state.budget.copy_symbol(owner)?,
                    binding.input,
                    freeze(&binding.tokens, &mut state.budget)?,
                ));
            }
        }
        let mut templates = Vec::new();
        for template in state
            .templates
            .values()
            .filter(|t| t.root.owner.module().module_name() == module)
        {
            state.budget.retain::<Template>(1)?;
            grow(&mut templates, 1)?;
            state
                .budget
                .retain::<Parameter>(template.parameters.len())?;
            let mut parameters = Vec::new();
            grow(&mut parameters, template.parameters.len())?;
            for p in &template.parameters {
                parameters.push((
                    name_span(source, p.name)?,
                    p.phase,
                    state.budget.copy_syntax_type(&p.ty)?,
                    freeze(&p.default, &mut state.budget)?,
                    p.location,
                ));
            }
            templates.push(Template {
                root: state.budget.copy_root(&template.root)?,
                parameters,
                body: freeze(&template.body, &mut state.budget)?,
            });
        }
        let mut matrices = Vec::new();
        for matrix in state
            .matrices
            .iter()
            .filter(|m| m.root.owner.module().module_name() == module)
        {
            state.budget.retain::<Matrix>(1)?;
            grow(&mut matrices, 1)?;
            state
                .budget
                .retain::<(ByteSpan, Tokens)>(matrix.axes.len())?;
            let mut axes = Vec::new();
            grow(&mut axes, matrix.axes.len())?;
            for (name, tokens) in &matrix.axes {
                axes.push((name_span(source, name)?, freeze(tokens, &mut state.budget)?));
            }
            matrices.push(Matrix {
                root: state.budget.copy_root(&matrix.root)?,
                template: freeze(&matrix.template, &mut state.budget)?,
                arguments: freeze(&matrix.arguments, &mut state.budget)?,
                axes,
                filter: freeze(&matrix.filter, &mut state.budget)?,
                exclusions: freeze(&matrix.exclusions, &mut state.budget)?,
            });
        }
        state.budget.retain::<Self>(1)?;
        Ok(Self {
            profiles: state.captured.shared_profiles(),
            capture_limits: state.captured.limits(),
            composition_limits: state.captured.composition_limits(),
            expression_limits: state.captured.expression_limits(),
            expansion_limits: state.captured.expansion_limits(),
            roots,
            bindings,
            templates,
            matrices,
            parse_work: work,
            parse_bytes: bytes,
        })
    }

    /// Restores only syntax and reruns namespace/limit checks under current controls.
    #[allow(clippy::too_many_lines)]
    fn restore<'a>(
        &self,
        source: &'a CapturedProjectSource,
        state: &mut State<'a, '_>,
    ) -> Result<(), E> {
        state.budget.charge(self.parse_work)?;
        state.budget.storage(self.parse_bytes)?;
        let count = self.roots.len() + self.templates.len() + self.matrices.len();
        state.budget.retain::<Declaration<'_>>(count)?;
        state
            .budget
            .charge((count as u64).saturating_mul(u64::from(count.bit_width()) + 1))?;
        let mut declarations = Vec::new();
        grow(&mut declarations, count)?;
        declarations.extend(self.roots.iter().map(Declaration::Root));
        declarations.extend(self.templates.iter().map(Declaration::Template));
        declarations.extend(self.matrices.iter().map(Declaration::Matrix));
        declarations.sort_unstable_by_key(Declaration::start);
        for declaration in declarations {
            match declaration {
                Declaration::Root(root) => {
                    let root = state.budget.copy_root(root)?;
                    state.insert_root(root)?;
                }
                Declaration::Template(template) => Self::restore_template(template, source, state)?,
                Declaration::Matrix(matrix) => Self::restore_matrix(matrix, source, state)?,
            }
        }
        for (owner, input, tokens) in &self.bindings {
            state.budget.retain::<parser::BindingSyntax<'_>>(1)?;
            let owner = state.budget.copy_symbol(owner)?;
            let tokens = thaw(source, tokens, &mut state.budget)?;
            alloc(state.bindings.insert(
                owner,
                parser::BindingSyntax {
                    tokens,
                    input: *input,
                    axes: Vec::new(),
                },
            ))?;
        }
        Ok(())
    }

    /// Rebinds one definition's source parameters without resolving or evaluating them.
    fn restore_template<'a>(
        template: &Template,
        source: &'a CapturedProjectSource,
        state: &mut State<'a, '_>,
    ) -> Result<(), E> {
        state.check_name(&template.root.owner)?;
        if state.templates.len() as u64 >= state.budget.expansion.definitions {
            return Err(error(ec::TPL_LIMIT, Some(template.root.location)));
        }
        state
            .budget
            .retain::<parser::ParameterSyntax<'_>>(template.parameters.len())?;
        let mut parameters = Vec::new();
        grow(&mut parameters, template.parameters.len())?;
        for (name, phase, ty, default, location) in &template.parameters {
            parameters.push(parser::ParameterSyntax {
                name: spelling(source, *name)?,
                phase: *phase,
                ty: state.budget.copy_syntax_type(ty)?,
                default: thaw(source, default, &mut state.budget)?,
                location: *location,
            });
        }
        let root = state.budget.copy_root(&template.root)?;
        let body = thaw(source, &template.body, &mut state.budget)?;
        let owner = state.budget.copy_symbol(&root.owner)?;
        alloc(state.templates.insert(
            owner,
            TemplateSyntax {
                root,
                parameters,
                body,
            },
        ))?;
        Ok(())
    }

    /// Rebinds a matrix before its normal static-cardinality and filtering phase.
    fn restore_matrix<'a>(
        matrix: &Matrix,
        source: &'a CapturedProjectSource,
        state: &mut State<'a, '_>,
    ) -> Result<(), E> {
        state.check_name(&matrix.root.owner)?;
        state
            .budget
            .retain::<(&str, Vec<Lex<'_>>)>(matrix.axes.len())?;
        let mut axes = Vec::new();
        grow(&mut axes, matrix.axes.len())?;
        for (name, tokens) in &matrix.axes {
            axes.push((
                spelling(source, *name)?,
                thaw(source, tokens, &mut state.budget)?,
            ));
        }
        state.budget.retain::<MatrixSyntax<'_>>(1)?;
        grow(&mut state.matrices, 1)?;
        state.matrices.push(MatrixSyntax {
            root: state.budget.copy_root(&matrix.root)?,
            template: thaw(source, &matrix.template, &mut state.budget)?,
            arguments: thaw(source, &matrix.arguments, &mut state.budget)?,
            axes,
            filter: thaw(source, &matrix.filter, &mut state.budget)?,
            exclusions: thaw(source, &matrix.exclusions, &mut state.budget)?,
        });
        Ok(())
    }
}

impl ExpressionCompilationCache {
    /// Reuses exact `/3` source syntax, rebuilding every semantic/evidence/identity phase.
    ///
    /// # Errors
    /// Any compilation, allocation or cancellation failure preserves the prior generation.
    pub fn compile_expression(
        &mut self,
        captured: &CapturedExpressionProject,
        cancel: &CancellationToken,
    ) -> Result<
        (
            Shared<neutral_ir::expression::project::ExpressionProjectIr>,
            ProjectCacheStats,
        ),
        E,
    > {
        let mut pending = OrderedMap::new();
        let mut stats = ProjectCacheStats::default();
        let mut bytes = 0_u64;
        let result = compile_with_parser(captured, cancel, &mut |source, state| {
            let previous = self.entries.get(source.module_id());
            let reusable = previous
                .filter(|entry| {
                    entry.digest == source.digest()
                        && entry.bytes.as_ref() == source.bytes()
                        && entry.features.as_slice() == captured.required_features()
                })
                .map(|entry| &entry.syntax)
                .filter(|syntax| {
                    syntax.profiles.as_ref() == captured.semantic_profiles()
                        && syntax.capture_limits == captured.limits()
                        && syntax.composition_limits == captured.composition_limits()
                        && syntax.expression_limits == captured.expression_limits()
                        && syntax.expansion_limits == captured.expansion_limits()
                        && syntax
                            .roots
                            .iter()
                            .all(|r| r.owner.module().module_name() == source.module_id())
                });
            let retain = (pending.len() as u64) < self.limits.source_units
                && bytes
                    .checked_add(source.bytes().len() as u64)
                    .is_some_and(|n| n <= self.limits.source_bytes);
            let syntax = if let Some(syntax) = reusable {
                stats.reused_units += 1;
                syntax.restore(source, state)?;
                Some(syntax.clone())
            } else {
                stats.parsed_units += 1;
                stats.rejected_entries += u64::from(previous.is_some());
                let before = (state.budget.work, state.budget.bytes);
                parser::parse_source(source, state)?;
                if retain {
                    let work = state.budget.work - before.0;
                    let retained = state.budget.bytes - before.1;
                    Some(alloc(Shared::try_new(Syntax::retain(
                        source, state, work, retained,
                    )?))?)
                } else {
                    None
                }
            };
            if retain {
                let key = state.budget.copy_text(source.module_id())?;
                state.budget.retain::<Entry>(1)?;
                alloc(pending.insert(
                    key,
                    Entry {
                        features: captured.composition.shared_features(),
                        digest: source.digest(),
                        bytes: source.shared_bytes(),
                        syntax: syntax.ok_or_else(|| error(ec::EXP_LIMIT, None))?,
                    },
                ))?;
                bytes += source.bytes().len() as u64;
            }
            Ok(())
        })?;
        if cancel.is_cancelled() {
            return Err(error(ec::EXP_CANCELLED, None));
        }
        self.entries = pending;
        self.source_bytes = bytes;
        Ok((result, stats))
    }
}
