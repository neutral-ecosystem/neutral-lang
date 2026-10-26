// SPDX-License-Identifier: Apache-2.0

//! Definition-scoped applications and checked finite Cartesian enumeration.
//!
//! Templates are typed in definition scope, including unused/private definitions
//! and all branches. Parameters retain declared order, exact types, explicit
//! static/deferred phases and definition-scoped defaults. Applications resolve
//! caller arguments separately, substitute lexical values hygienically and leave
//! no runtime call node. Recursion is rejected through an active-definition stack.
//!
//! Matrices require static finite axes and preflight the full unfiltered Cartesian
//! product before tuple enumeration. Filters/exclusions can remove output but cannot
//! erase visitation costs or renumber original axis positions. Deferred data may
//! populate generated bodies, never choose their count, names or topology. Generated
//! names share the ordinary declaration namespace and preserve original-index suffixes.
//!
//! Nested typing, applications, static checks and copying share persistent expression
//! and expansion allowances. Real application/definition/argument/axis lineage is
//! retained separately from generated semantic identity. Any failure discards the
//! request's unpublished generation instead of returning a partially expanded project.

use super::{
    BindingSyntax, ByteSpan, E, ExpansionAttribution, Expression, ExpressionBody, Frame, Lex,
    Local, MatrixSyntax, ModuleSymbolIdentity, N, ParameterPhase, PublicTemplate, RetainCapacity,
    Root, SourceLocation, State, T, TemplateParameter, V, alloc, composition_error, copy_slice,
    error, grow, parser, symbol,
};
use crate::frontend::expression_codes as ec;

impl<'a> State<'a, '_> {
    /// Resolves exact definition owners using only the captured module import graph.
    fn template_owner(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
    ) -> Result<ModuleSymbolIdentity, E> {
        let mut cursor = 0;
        let (alias, name, _) = parser::name(tokens, &mut cursor)?;
        if cursor != tokens.len() {
            return Err(error(ec::TPL_NAME, Some(root.location)));
        }
        let module = root.owner.module().module_name();
        let target = if let Some(alias) = alias {
            self.resolver
                .graph
                .edges()
                .iter()
                .find(|e| e.from() == module && e.alias() == alias)
                .map(super::graph::Import::target)
                .ok_or_else(|| error(ec::TPL_NAME, Some(root.location)))?
        } else {
            module
        };
        let owner = symbol(target, name)?;
        let template = self
            .templates
            .get(&owner)
            .ok_or_else(|| error(ec::TPL_NAME, Some(root.location)))?;
        if target != module && !template.root.public {
            return Err(error(ec::TPL_NAME, Some(root.location)));
        }
        Ok(owner)
    }

    /// Types every definition, including unused/private definitions and recursive skipped paths.
    /// Validates a non-callable contract; it does not execute arithmetic in unused bodies.
    pub(super) fn template(&mut self, owner: &ModuleSymbolIdentity) -> Result<(), E> {
        self.in_expansion(|state| state.template_inner(owner))
    }

    /// Definition typing runs inside the caller's persistent expansion counters.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn template_inner(&mut self, owner: &ModuleSymbolIdentity) -> Result<(), E> {
        if self.typed_templates.contains_key(owner) {
            return Ok(());
        }
        self.budget.retain::<ModuleSymbolIdentity>(1)?;
        if !alloc(
            self.active_templates
                .insert(self.budget.copy_symbol(owner)?),
        )? {
            return Err(error(ec::TPL_RECURSION, None));
        }
        if self.active_templates.len() as u64 > self.budget.expansion.depth {
            return Err(error(ec::TPL_LIMIT, None));
        }
        let Some(syntax) = self.templates.get(owner) else {
            return Err(error(ec::TPL_NAME, None));
        };
        let root = self.budget.copy_root(&syntax.root)?;
        let body_tokens = self.budget.copy_tokens(&syntax.body)?;
        let mut parameter_syntax = Vec::new();
        self.budget
            .retain::<(&str, ParameterPhase, super::super::TypeExpr, Vec<Lex<'_>>)>(
                syntax.parameters.len(),
            )?;
        grow(&mut parameter_syntax, syntax.parameters.len())?;
        for p in &syntax.parameters {
            parameter_syntax.push((
                p.name,
                p.phase,
                self.budget.copy_syntax_type(&p.ty)?,
                self.budget.copy_tokens(&p.default)?,
            ));
        }
        let result_type = self
            .resolver
            .ty(
                &root,
                match root.ty.as_ref() {
                    Some(ty) => ty,
                    None => return Err(error(ec::TPL_TYPE, Some(root.location))),
                },
            )
            .map_err(composition_error)?;
        let mut parameters = Vec::new();
        let mut frames = Vec::new();
        self.budget.retain::<Frame<'_>>(1)?;
        grow(&mut frames, 1)?;
        frames.push(Vec::new());
        let previous = self.typing_template;
        self.typing_template = true;
        for (name, phase, syntax, default_tokens) in parameter_syntax {
            let ty = self
                .resolver
                .ty(&root, &syntax)
                .map_err(composition_error)?;
            let default = if default_tokens.is_empty() {
                None
            } else {
                Some(self.expression(&root, &default_tokens, Some(&ty), &mut frames, 1)?)
            };
            self.budget.retain::<Local<'_>>(1)?;
            grow(&mut frames[0], 1)?;
            // Insert after typing its default, so that default sees earlier parameters, not itself.
            frames[0].push(Local::Bound(name, self.budget.copy_type(&ty)?, phase));
            self.budget.retain::<TemplateParameter>(1)?;
            grow(&mut parameters, 1)?;
            parameters.push(TemplateParameter {
                name: self.budget.copy_text(name)?,
                phase,
                ty,
                default,
            });
        }
        let body = self.expression(&root, &body_tokens, Some(&result_type), &mut frames, 1)?;
        self.typing_template = previous;
        self.active_templates.remove(owner);
        self.budget.retain::<PublicTemplate>(1)?;
        alloc(self.typed_templates.insert(
            self.budget.copy_symbol(owner)?,
            PublicTemplate {
                identity: self.budget.copy_symbol(owner)?,
                parameters,
                result_type,
                body,
            },
        ))?;
        Ok(())
    }

    /// Lowers one named application without retaining a runtime invocation node.
    pub(super) fn instantiate(
        &mut self,
        caller: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
        frames: &mut Vec<Frame<'a>>,
        depth: u64,
    ) -> Result<Expression, E> {
        let mut cursor = 0;
        parser::name(tokens, &mut cursor)?;
        let owner = self.template_owner(caller, &tokens[..cursor])?;
        let args = parser::group(tokens, &mut cursor, "(", ")")?;
        if cursor != tokens.len() {
            return Err(error(ec::TPL_SYNTAX, Some(caller.location)));
        }
        self.application(caller, &owner, args, expected, frames, depth, None)
    }

    /// Checks all arguments/defaults in declared order and substitutes at definition scope.
    /// Caller expressions and definition defaults use separate frames; this is
    /// compile-time substitution, not evaluation under ambient consumer bindings.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)] // Explicit lexical caller/definition frames are kept at this single application boundary.
    fn application(
        &mut self,
        caller: &Root,
        owner: &ModuleSymbolIdentity,
        args: &[Lex<'a>],
        expected: Option<&T>,
        caller_frames: &mut Vec<Frame<'a>>,
        depth: u64,
        axis_indices: Option<&[u64]>,
    ) -> Result<Expression, E> {
        self.in_expansion(|state| {
            state.application_inner(
                caller,
                owner,
                args,
                expected,
                caller_frames,
                depth,
                axis_indices,
            )
        })
    }

    /// Substitution and queued static checks retain the same expansion authority.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn application_inner(
        &mut self,
        caller: &Root,
        owner: &ModuleSymbolIdentity,
        args: &[Lex<'a>],
        expected: Option<&T>,
        caller_frames: &mut Vec<Frame<'a>>,
        depth: u64,
        axis_indices: Option<&[u64]>,
    ) -> Result<Expression, E> {
        self.budget.applications = match self.budget.applications.checked_add(1) {
            Some(count) => count,
            None => return Err(error(ec::TPL_LIMIT, Some(caller.location))),
        };
        if self.budget.applications > self.budget.expansion.applications
            || depth > self.budget.expansion.depth
        {
            return Err(error(ec::TPL_LIMIT, Some(caller.location)));
        }
        self.budget.expansion(1, 0)?;
        self.template(owner)?;
        let Some(syntax) = self.templates.get(owner) else {
            return Err(error(ec::TPL_NAME, Some(caller.location)));
        };
        let definition = self.budget.copy_root(&syntax.root)?;
        let body_tokens = self.budget.copy_tokens(&syntax.body)?;
        let mut parameters = Vec::new();
        self.budget.retain::<(
            &str,
            ParameterPhase,
            super::super::TypeExpr,
            Vec<Lex<'_>>,
            SourceLocation,
        )>(syntax.parameters.len())?;
        grow(&mut parameters, syntax.parameters.len())?;
        for p in &syntax.parameters {
            parameters.push((
                p.name,
                p.phase,
                self.budget.copy_syntax_type(&p.ty)?,
                self.budget.copy_tokens(&p.default)?,
                p.location,
            ));
        }
        let result_type = self
            .budget
            .copy_type(&self.typed_templates[owner].result_type)?;
        if expected.is_some_and(|t| *t != result_type) {
            return Err(error(ec::TPL_TYPE, Some(caller.location)));
        }
        let mut arguments = Vec::new();
        self.budget.retain::<&[Lex<'_>]>(args.len())?;
        for arg in parser::split(args)? {
            self.budget
                .charge((parameters.len() + arguments.len()) as u64 + 1)?;
            if arg.len() < 3
                || arg[1].spelling != ":"
                || !parameters.iter().any(|p| p.0 == arg[0].spelling)
                || arguments
                    .iter()
                    .any(|(name, _): &(&str, &[Lex<'_>])| *name == arg[0].spelling)
            {
                return Err(error(ec::TPL_NAME, Some(caller.location)));
            }
            self.budget.retain::<(&str, &[Lex<'_>])>(1)?;
            grow(&mut arguments, 1)?;
            arguments.push((arg[0].spelling, &arg[2..]));
        }
        let previous_root = self.application_root.take();
        self.application_root = Some(if let Some(root) = &previous_root {
            self.budget.copy_root(root)?
        } else {
            self.budget.copy_root(caller)?
        });
        let mut bound = Vec::new();
        let mut locations = Vec::new();
        self.budget.retain::<Frame<'_>>(1)?;
        grow(&mut bound, 1)?;
        bound.push(Vec::new());
        for (name, phase, syntax, default_tokens, parameter_location) in parameters {
            let ty = self
                .resolver
                .ty(&definition, &syntax)
                .map_err(composition_error)?;
            let (value, location) =
                if let Some((_, tokens)) = arguments.iter().find(|(n, _)| *n == name) {
                    (
                        self.expression(caller, tokens, Some(&ty), caller_frames, depth + 1)
                            .map_err(|e| {
                                if e.code == ec::EXP_TYPE {
                                    error(ec::TPL_TYPE, e.location)
                                } else {
                                    e
                                }
                            })?,
                        SourceLocation::new(
                            caller.location.source(),
                            match ByteSpan::new(
                                tokens[0].span.start(),
                                match tokens.last() {
                                    Some(token) => token.span.end(),
                                    None => return Err(error(ec::TPL_NAME, Some(caller.location))),
                                },
                            ) {
                                Ok(span) => span,
                                Err(_) => return Err(error(ec::TPL_LIMIT, Some(caller.location))),
                            },
                        ),
                    )
                } else if !default_tokens.is_empty() {
                    (
                        self.expression(
                            &definition,
                            &default_tokens,
                            Some(&ty),
                            &mut bound,
                            depth + 1,
                        )?,
                        parameter_location,
                    )
                } else {
                    return Err(error(ec::TPL_NAME, Some(caller.location)));
                };
            let value = if phase == ParameterPhase::Static {
                if self.typing_template {
                    if !phase_static(&value, caller_frames, &bound) {
                        return Err(error(ec::TPL_TYPE, Some(location)));
                    }
                    value
                } else {
                    let root = self
                        .budget
                        .copy_root(match self.application_root.as_ref() {
                            Some(root) => root,
                            None => return Err(error(ec::TPL_NAME, Some(location))),
                        })?;
                    let copy = self.clone_expression(&value)?;
                    self.budget.retain::<(Root, Expression)>(1)?;
                    grow(&mut self.pending_static, 1)?;
                    self.pending_static.push((root, copy));
                    value
                }
            } else {
                value
            };
            self.budget.retain::<SourceLocation>(1)?;
            grow(&mut locations, 1)?;
            locations.push(location);
            self.budget.retain::<Local<'_>>(1)?;
            grow(&mut bound[0], 1)?;
            bound[0].push(Local::Value(name, value));
        }
        // Reparse only private syntax at definition scope; substitution cannot capture caller names.
        let result = self.expression(
            &definition,
            &body_tokens,
            Some(&result_type),
            &mut bound,
            depth + 1,
        )?;
        if !self.typing_template {
            self.budget.retain::<ExpansionAttribution>(1)?;
            self.budget
                .retain::<u64>(axis_indices.map_or(0, <[u64]>::len))?;
            grow(&mut self.expansions, 1)?;
            self.expansions.push(ExpansionAttribution {
                owner: self
                    .budget
                    .copy_symbol(match self.application_root.as_ref() {
                        Some(root) => &root.owner,
                        None => return Err(error(ec::TPL_NAME, Some(caller.location))),
                    })?,
                application: caller.location,
                definition: definition.location,
                arguments: locations,
                original_axis_indices: axis_indices
                    .map(|indices| alloc(copy_slice(indices)))
                    .transpose()?
                    .unwrap_or_default(),
            });
            let inspected = neutral_ir::expression::inspect_expression_retention(
                &result,
                self.budget.remaining(),
                self.budget.cancel,
            )
            .map_err(|_| error(ec::TPL_LIMIT, Some(caller.location)))?;
            self.budget.charge(inspected.work)?;
            let count = inspected.shape.nodes;
            self.budget.expanded_nodes = match self.budget.expanded_nodes.checked_add(count) {
                Some(count) => count,
                None => return Err(error(ec::TPL_LIMIT, Some(caller.location))),
            };
            if self.budget.expanded_nodes > self.budget.expansion.expanded_nodes {
                return Err(error(ec::TPL_LIMIT, Some(caller.location)));
            }
            self.budget.expansion(count, inspected.retained_bytes)?;
        }
        self.application_root = previous_root;
        Ok(result)
    }

    /// Preflights full static cardinality, then visits original tuples in source axis order.
    #[allow(clippy::too_many_lines)] // Cardinality, filters and original-index publication form one atomic expansion.
    pub(super) fn expand_matrices(&mut self) -> Result<(), E> {
        self.in_expansion(Self::expand_matrices_inner)
    }

    /// Filters and excluded tuples consume work even when they produce no binding.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn expand_matrices_inner(&mut self) -> Result<(), E> {
        let matrices = std::mem::take(&mut self.matrices);
        for matrix in matrices {
            let owner = self.template_owner(&matrix.root, &matrix.template)?;
            self.template(&owner)?;
            let result_type = self
                .budget
                .copy_type(&self.typed_templates[&owner].result_type)?;
            let mut axes = Vec::new();
            // Full unfiltered cardinality is checked before allocating or visiting tuples.
            for (name, tokens) in &matrix.axes {
                let value = self.expression(&matrix.root, tokens, None, &mut Vec::new(), 1)?;
                let T::List(inner) = &value.ty else {
                    return Err(error(ec::TPL_AXIS, Some(matrix.root.location)));
                };
                let inner = self.budget.copy_type(inner)?;
                let Some(V::List(items)) = self.static_value(&matrix.root, &value)? else {
                    return Err(error(ec::TPL_TYPE, Some(matrix.root.location)));
                };
                if items.len() as u64 > self.budget.expansion.axis_items {
                    return Err(error(ec::TPL_LIMIT, Some(matrix.root.location)));
                }
                self.budget.retain::<(&str, T, Vec<V>)>(1)?;
                grow(&mut axes, 1)?;
                axes.push((*name, inner, items));
            }
            let mut product = u64::from(!axes.iter().any(|(_, _, values)| values.is_empty()));
            // An empty axis annihilates the product before multiplication, even after large preceding axes.
            for (_, _, values) in &axes {
                product = product
                    .checked_mul(values.len() as u64)
                    .filter(|n| *n <= self.budget.expansion.product_tuples)
                    .ok_or_else(|| error(ec::TPL_LIMIT, Some(matrix.root.location)))?;
            }
            self.budget.tuples = match self.budget.tuples.checked_add(product) {
                Some(count) => count,
                None => return Err(error(ec::TPL_LIMIT, Some(matrix.root.location))),
            };
            if self.budget.tuples > self.budget.expansion.aggregate_tuples {
                return Err(error(ec::TPL_LIMIT, Some(matrix.root.location)));
            }
            let exclusions = self.exclusions(&matrix, &axes)?;
            // Shape validation is independent of retained tuple count, including empty axes.
            let mut axis_frames = Vec::new();
            self.budget.retain::<Frame<'_>>(1)?;
            grow(&mut axis_frames, 1)?;
            axis_frames.push(Vec::new());
            for (name, ty, _) in &axes {
                self.budget.retain::<Local<'_>>(1)?;
                grow(&mut axis_frames[0], 1)?;
                axis_frames[0].push(Local::Bound(
                    name,
                    self.budget.copy_type(ty)?,
                    ParameterPhase::Static,
                ));
            }
            let before = self.typing_template;
            self.typing_template = true;
            if !matrix.filter.is_empty() {
                let filter = self.expression(
                    &matrix.root,
                    &matrix.filter,
                    Some(&T::Bool),
                    &mut axis_frames,
                    1,
                )?;
                if !phase_static(&filter, &axis_frames, &[]) {
                    return Err(error(ec::TPL_AXIS, Some(matrix.root.location)));
                }
            }
            self.application(
                &matrix.root,
                &owner,
                &matrix.arguments,
                Some(&result_type),
                &mut axis_frames,
                1,
                None,
            )?;
            self.typing_template = before;
            let mut indices = Vec::new();
            self.budget.retain::<u64>(axes.len())?;
            grow(&mut indices, axes.len())?;
            indices.resize(axes.len(), 0_u64);
            for _ in 0..product {
                self.budget.expansion(1, 0)?;
                let mut frames = Vec::new();
                self.budget.retain::<Frame<'_>>(1)?;
                grow(&mut frames, 1)?;
                frames.push(Vec::new());
                let mut tuple = Vec::new();
                self.budget.retain::<V>(axes.len())?;
                grow(&mut tuple, axes.len())?;
                for ((name, ty, values), index) in axes.iter().zip(&indices) {
                    let value = self.budget.copy_value(
                        match values.get(match usize::try_from(*index) {
                            Ok(index) => index,
                            Err(_) => return Err(error(ec::TPL_LIMIT, Some(matrix.root.location))),
                        }) {
                            Some(value) => value,
                            None => return Err(error(ec::TPL_LIMIT, Some(matrix.root.location))),
                        },
                    )?;
                    tuple.push(self.budget.copy_value(&value)?);
                    self.budget.retain::<Local<'_>>(1)?;
                    grow(&mut frames[0], 1)?;
                    frames[0].push(Local::Value(
                        name,
                        Expression {
                            ty: self.budget.copy_type(ty)?,
                            node: N::Value(value),
                        },
                    ));
                }
                let retain = if matrix.filter.is_empty() {
                    true
                } else {
                    let filter = self.expression(
                        &matrix.root,
                        &matrix.filter,
                        Some(&T::Bool),
                        &mut frames,
                        1,
                    )?;
                    match self.static_value(&matrix.root, &filter)? {
                        Some(V::Bool(value)) => value,
                        _ => {
                            return Err(error(ec::TPL_AXIS, Some(matrix.root.location)));
                        }
                    }
                };
                if retain && !exclusions.contains(&tuple) {
                    self.budget.expanded = match self.budget.expanded.checked_add(1) {
                        Some(count) => count,
                        None => return Err(error(ec::TPL_LIMIT, Some(matrix.root.location))),
                    };
                    if self.budget.expanded > self.budget.expansion.expanded_bindings {
                        return Err(error(ec::TPL_LIMIT, Some(matrix.root.location)));
                    }
                    let mut name = self
                        .budget
                        .copy_text(matrix.root.owner.declaration_name())?;
                    let Some(extra) = (indices.len() as u64).checked_mul(22) else {
                        return Err(error(ec::TPL_LIMIT, Some(matrix.root.location)));
                    };
                    let limit = self.captured.composition_limits().json.string_bytes();
                    if (name.len() as u64)
                        .checked_add(extra)
                        .is_none_or(|n| n > limit)
                    {
                        return Err(error(ec::TPL_LIMIT, Some(matrix.root.location)));
                    }
                    self.budget.bytes(extra)?;
                    alloc(name.try_retain(match usize::try_from(extra) {
                        Ok(extra) => extra,
                        Err(_) => return Err(error(ec::TPL_LIMIT, Some(matrix.root.location))),
                    }))?;
                    for index in &indices {
                        use std::fmt::Write;
                        if write!(name, "__{index}").is_err() {
                            return Err(error(ec::TPL_LIMIT, Some(matrix.root.location)));
                        }
                    }
                    let generated = symbol(matrix.root.owner.module().module_name(), &name)?;
                    self.check_name(&generated)?;
                    let mut root = self.budget.copy_root(&matrix.root)?;
                    root.owner = self.budget.copy_symbol(&generated)?;
                    root.ty = self.templates[&owner]
                        .root
                        .ty
                        .as_ref()
                        .map(|ty| self.budget.copy_syntax_type(ty))
                        .transpose()?;
                    // The resolved definition-scope result type is retained directly; no alias rebinding.
                    alloc(self.resolver.types.insert(
                        self.budget.copy_symbol(&generated)?,
                        self.budget.copy_type(&result_type)?,
                    ))?;
                    self.insert_root(root)?;
                    let mut binding_root = self.budget.copy_root(&matrix.root)?;
                    binding_root.owner = self.budget.copy_symbol(&generated)?;
                    let expression = self.application(
                        &binding_root,
                        &owner,
                        &matrix.arguments,
                        Some(&result_type),
                        &mut frames,
                        1,
                        Some(&indices),
                    )?;
                    alloc(self.bodies.insert(
                        self.budget.copy_symbol(&generated)?,
                        ExpressionBody::Expression(expression),
                    ))?;
                    alloc(self.bindings.insert(
                        generated,
                        BindingSyntax {
                            tokens: Vec::new(),
                            input: false,
                            axes: Vec::new(),
                        },
                    ))?;
                }
                // Mixed-radix counting preserves original axis positions, including filtered tuples.
                for axis in (0..indices.len()).rev() {
                    indices[axis] += 1;
                    if indices[axis] < axes[axis].2.len() as u64 {
                        break;
                    }
                    indices[axis] = 0;
                }
            }
        }
        Ok(())
    }

    /// Checks exact axis coverage, static values and duplicate exclusion tuples.
    fn exclusions(
        &mut self,
        matrix: &MatrixSyntax<'a>,
        axes: &[(&'a str, T, Vec<V>)],
    ) -> Result<Vec<Vec<V>>, E> {
        let mut exclusions = Vec::new();
        if matrix.exclusions.is_empty() {
            return Ok(exclusions);
        }
        let mut cursor = 0;
        self.budget.retain::<&[Lex<'_>]>(matrix.exclusions.len())?;
        let records = parser::group(&matrix.exclusions, &mut cursor, "[", "]")?;
        if cursor != matrix.exclusions.len() {
            return Err(error(ec::TPL_AXIS, Some(matrix.root.location)));
        }
        for record in parser::split(records)? {
            self.budget.retain::<&[Lex<'_>]>(record.len())?;
            let mut cursor = 0;
            let fields = parser::split(parser::group(record, &mut cursor, "{", "}")?)?;
            if cursor != record.len() || fields.len() != axes.len() {
                return Err(error(ec::TPL_AXIS, Some(matrix.root.location)));
            }
            let mut tuple = Vec::new();
            self.budget.retain::<V>(axes.len())?;
            grow(&mut tuple, axes.len())?;
            for (name, ty, values) in axes {
                self.budget.charge(fields.len() as u64 + 1)?;
                let mut members = fields
                    .iter()
                    .filter(|f| f.first().is_some_and(|t| t.spelling == *name));
                let Some(member) = members.next() else {
                    return Err(error(ec::TPL_AXIS, Some(matrix.root.location)));
                };
                if members.next().is_some() || member.len() < 3 || member[1].spelling != ":" {
                    return Err(error(ec::TPL_AXIS, Some(matrix.root.location)));
                }
                let expression =
                    self.expression(&matrix.root, &member[2..], Some(ty), &mut Vec::new(), 1)?;
                let value = self
                    .static_value(&matrix.root, &expression)?
                    .ok_or_else(|| error(ec::TPL_AXIS, Some(matrix.root.location)))?;
                if !values.contains(&value) {
                    return Err(error(ec::TPL_AXIS, Some(matrix.root.location)));
                }
                tuple.push(value);
            }
            if exclusions.contains(&tuple) {
                return Err(error(ec::TPL_AXIS, Some(matrix.root.location)));
            }
            self.budget.retain::<Vec<V>>(1)?;
            grow(&mut exclusions, 1)?;
            exclusions.push(tuple);
        }
        Ok(exclusions)
    }
}

/// Static lexical parameters remain symbolic while typing definitions, without executing them.
fn phase_static(expression: &Expression, caller: &[Frame<'_>], defaults: &[Frame<'_>]) -> bool {
    match &expression.node {
        N::Value(_) => true,
        N::Input(_) | N::Output { .. } => false,
        N::Bound { depth, slot } => {
            let (Ok(depth), Ok(slot)) = (usize::try_from(*depth), usize::try_from(*slot)) else {
                return false;
            };
            let frame = match caller.iter().rev().nth(depth) {
                Some(frame) => Some(frame),
                None => defaults.iter().rev().nth(depth),
            };
            frame.and_then(|f| f.get(slot)).is_some_and(|l| {
                matches!(
                    l,
                    Local::Bound(_, _, ParameterPhase::Static)
                        | Local::Value(
                            _,
                            Expression {
                                node: N::Value(_),
                                ..
                            }
                        )
                )
            })
        }
        N::Unary { operand, .. } | N::Text(operand) => phase_static(operand, caller, defaults),
        N::Binary { left, right, .. } => {
            phase_static(left, caller, defaults) && phase_static(right, caller, defaults)
        }
        N::If {
            guard,
            then_branch,
            else_branch,
        } => {
            phase_static(guard, caller, defaults)
                && phase_static(then_branch, caller, defaults)
                && phase_static(else_branch, caller, defaults)
        }
        N::Match { scrutinee, arms } => {
            phase_static(scrutinee, caller, defaults)
                && arms.iter().all(|a| phase_static(&a.body, caller, defaults))
        }
        N::List(items) | N::Interpolate(items) => {
            items.iter().all(|e| phase_static(e, caller, defaults))
        }
        N::Record(fields) => fields
            .iter()
            .all(|(_, e)| e.as_ref().is_none_or(|e| phase_static(e, caller, defaults))),
        N::Variant { payload, .. } => phase_static(payload, caller, defaults),
    }
}
