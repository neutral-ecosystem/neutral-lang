// SPDX-License-Identifier: Apache-2.0

//! All-branch typing and stable-name lowering; ordinary reuse is materialized.
//!
//! Binding resolution memoizes request-local meaning and detects cycles before
//! evaluation. Every branch resolves under explicit lexical frames, including
//! branches that static folding will skip. Stable symbols identify dependencies;
//! source aliases never become semantic identities or host lookup instructions.
//!
//! Expression parsing and typing are contextual: explicit expected types resolve
//! empty lists, typed nulls, nominal record/variant construction and inert location
//! literals. Operators require exact operand/result contracts without implicit
//! numeric/string coercions. Both sides and every conditional/match arm are typed
//! here; short-circuit evaluation belongs to the later shared semantic kernel.
//!
//! Ordinary reuse copies a resolved immutable body, while an explicit reference
//! retains an identity target. External input and future output nodes remain data;
//! output selector legality does not prove a value is currently available. Lexical
//! parameter/payload slots are distinct from global symbols, and substitutions lift
//! free frame depths so nested matches cannot capture a caller's binding.
//!
//! All recursive construction and copies debit the persistent request budget.
//! Successful body memoization is request-local; a failed cycle/name/type check
//! discards compilation state instead of publishing a partial body registry.

use super::{
    B, Budget, ClosedValue, E, ExactNumber, Expression, ExpressionBody, FieldPresence, Frame, Lex,
    Local, MatchArm, ModuleSymbolIdentity, N, Op, ParameterPhase, RetainCapacity, Root,
    SelectorSegment, State, T, TryClone, UnaryOperator, V, alloc, boxed, composition_error,
    copy_slice, error, expression_profile, grow, is_protected_name, is_snake_name, parser, text,
};
use crate::frontend::expression_codes as ec;

impl<'a> State<'a, '_> {
    /// Resolves ordinary reuse once per request and rejects value cycles before evaluation.
    /// Returns an owned budgeted copy; use `ensure_binding` when only completion is needed.
    pub(super) fn binding(&mut self, owner: &ModuleSymbolIdentity) -> Result<ExpressionBody, E> {
        self.ensure_binding(owner)?;
        self.budget.charge(1)?;
        match self.bodies.get(owner) {
            Some(body) => Ok(match body {
                ExpressionBody::Input => ExpressionBody::Input,
                ExpressionBody::Value(v) => ExpressionBody::Value(self.budget.copy_value(v)?),
                ExpressionBody::Expression(e) => {
                    ExpressionBody::Expression(self.budget.copy_expression(e)?)
                }
            }),
            None => Err(error(ec::EXP_NAME, None)),
        }
    }

    /// Retains each resolved body once; callers that only require resolution do not copy it.
    pub(super) fn ensure_binding(&mut self, owner: &ModuleSymbolIdentity) -> Result<(), E> {
        self.budget.charge(1)?;
        if self.bodies.contains_key(owner) {
            return Ok(());
        }
        self.budget.retain::<ModuleSymbolIdentity>(1)?;
        if self.active.len() as u64 >= self.budget.expression.syntax_depth
            || !alloc(self.active.insert(self.budget.copy_symbol(owner)?))?
        {
            return Err(error(ec::EXP_CYCLE, None));
        }
        let root = self.budget.copy_root(
            self.resolver
                .roots
                .get(owner)
                .ok_or_else(|| error(ec::EXP_NAME, None))?,
        )?;
        let syntax = self
            .bindings
            .get(owner)
            .ok_or_else(|| error(ec::EXP_NAME, Some(root.location)))?;
        let input = syntax.input;
        self.budget.retain::<Lex<'_>>(syntax.tokens.len())?;
        let tokens = alloc(copy_slice(&syntax.tokens))?;
        let mut axes = Vec::new();
        self.budget.retain::<Local<'_>>(syntax.axes.len())?;
        grow(&mut axes, syntax.axes.len())?;
        for (name, value) in &syntax.axes {
            axes.push(Local::Value(
                name,
                self.budget.copy_expression(value).map_err(|mut e| {
                    e.location = Some(root.location);
                    e
                })?,
            ));
        }
        let ty = self.budget.copy_type(
            self.resolver
                .types
                .get(owner)
                .ok_or_else(|| error(ec::EXP_NAME, Some(root.location)))?,
        )?;
        let mut frames = Vec::new();
        if !axes.is_empty() {
            self.budget.retain::<Frame<'_>>(1)?;
            grow(&mut frames, 1)?;
            frames.push(axes);
        }
        let body = if input {
            ExpressionBody::Input
        } else {
            ExpressionBody::Expression(self.expression(
                &root,
                &tokens,
                Some(&ty),
                &mut frames,
                1,
            )?)
        };
        self.active.remove(owner);
        // Only completed bodies enter the memo table; active markers are never accepted as results.
        self.budget.retain::<ExpressionBody>(1)?;
        alloc(self.bodies.insert(self.budget.copy_symbol(owner)?, body))?;
        Ok(())
    }

    /// Types all branches and lowers contextual syntax into closed ordered expression nodes.
    #[allow(clippy::too_many_lines)] // One closed precedence dispatch preserves branch/type-check order.
    pub(super) fn expression(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
        frames: &mut Vec<Frame<'a>>,
        depth: u64,
    ) -> Result<Expression, E> {
        self.budget.node(depth)?;
        if tokens.is_empty() {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        // The last weakest operator gives left associativity while groups remain opaque.
        let mut nesting = 0_i64;
        let mut selected = None;
        let mut previous = None;
        let mut relation_count = 0;
        let mut generic = 0_u64;
        let mut primary_head = false;
        for (i, t) in tokens.iter().enumerate() {
            if generic > 0 {
                match t.spelling {
                    "<" => generic += 1,
                    ">" => generic -= 1,
                    _ => {}
                }
                continue;
            }
            if t.spelling == "<" && i > 0 && tokens[i - 1].spelling == "output" {
                generic = 1;
                continue;
            }
            if nesting == 0 && matches!(t.spelling, "if" | "match") {
                primary_head = true;
            }
            match t.spelling {
                "{" if nesting == 0 => {
                    primary_head = false;
                    nesting += 1;
                }
                "(" | "[" | "{" => nesting += 1,
                ")" | "]" | "}" => nesting -= 1,
                _ => {}
            }
            if nesting < 0 {
                return Err(error(ec::EXP_SYNTAX, Some(root.location)));
            }
            // Guard/scrutinee operators belong to their primary, not its surrounding expression.
            if nesting != 0 || primary_head {
                continue;
            }
            let Some((precedence, operator)) = binary(t.spelling) else {
                previous = Some(t.spelling);
                continue;
            };
            if i == 0
                || previous.is_none_or(|p| {
                    matches!(
                        p,
                        "(" | "["
                            | "{"
                            | ","
                            | ":"
                            | "=>"
                            | "!"
                            | "-"
                            | "+"
                            | "*"
                            | "/"
                            | "++"
                            | "&&"
                            | "||"
                            | "<"
                            | "<="
                            | ">"
                            | ">="
                            | "=="
                            | "!="
                    )
                })
            {
                previous = Some(t.spelling);
                continue;
            }
            if matches!(precedence, 3 | 4) {
                relation_count += 1;
            }
            if selected.is_none_or(|(_, p, _)| precedence <= p) {
                selected = Some((i, precedence, operator));
            }
            previous = Some(t.spelling);
        }
        if nesting != 0 || generic != 0 {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        if let Some((i, precedence, operator)) = selected {
            if matches!(precedence, 3 | 4) && relation_count > 1 {
                return Err(error(ec::EXP_SYNTAX, Some(root.location)));
            }
            let operand_context = match operator {
                Op::Add | Op::Subtract | Op::Multiply | Op::Divide => Some(&T::Num),
                Op::And | Op::Or => Some(&T::Bool),
                Op::Concat => expected,
                _ => None,
            };
            let left = self.expression(root, &tokens[..i], operand_context, frames, depth + 1)?;
            let right =
                self.expression(root, &tokens[i + 1..], Some(&left.ty), frames, depth + 1)?;
            let ty = match operator {
                Op::Add | Op::Subtract | Op::Multiply | Op::Divide
                    if left.ty == T::Num && right.ty == T::Num =>
                {
                    T::Num
                }
                Op::And | Op::Or if left.ty == T::Bool && right.ty == T::Bool => T::Bool,
                Op::Concat if left.ty == right.ty && matches!(left.ty, T::String | T::List(_)) => {
                    self.budget.copy_type(&left.ty)?
                }
                Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual
                    if left.ty == right.ty && matches!(left.ty, T::Num | T::String) =>
                {
                    T::Bool
                }
                Op::Equal | Op::NotEqual if left.ty == right.ty => T::Bool,
                _ => return Err(error(ec::EXP_TYPE, Some(root.location))),
            };
            return Self::checked(
                Expression {
                    ty,
                    node: N::Binary {
                        operator,
                        left: alloc(boxed(left))?,
                        right: alloc(boxed(right))?,
                    },
                },
                expected,
                root,
            );
        }
        match tokens[0].spelling {
            "!" | "-" => {
                let operator = if tokens[0].spelling == "!" {
                    UnaryOperator::Not
                } else {
                    UnaryOperator::Negate
                };
                let ty = if operator == UnaryOperator::Not {
                    T::Bool
                } else {
                    T::Num
                };
                let operand = self.expression(root, &tokens[1..], Some(&ty), frames, depth + 1)?;
                Self::checked(
                    Expression {
                        ty,
                        node: N::Unary {
                            operator,
                            operand: alloc(boxed(operand))?,
                        },
                    },
                    expected,
                    root,
                )
            }
            "(" => {
                let mut cursor = 0;
                let inner = parser::group(tokens, &mut cursor, "(", ")")?;
                if cursor != tokens.len() {
                    return Err(error(ec::EXP_SYNTAX, Some(root.location)));
                }
                self.expression(root, inner, expected, frames, depth + 1)
            }
            "if" => self.conditional(root, tokens, expected, frames, depth),
            "match" => self.matching(root, tokens, expected, frames, depth),
            "[" => self.list(root, tokens, expected, frames, depth),
            "{" => self.record(root, tokens, expected, frames, depth),
            "text" => {
                let mut cursor = 1;
                let inner = parser::group(tokens, &mut cursor, "(", ")")?;
                if cursor != tokens.len() {
                    return Err(error(ec::EXP_SYNTAX, Some(root.location)));
                }
                let operand = self.expression(root, inner, None, frames, depth + 1)?;
                if !matches!(operand.ty, T::Num | T::Bool | T::String) {
                    return Err(error(ec::EXP_TYPE, Some(root.location)));
                }
                Self::checked(
                    Expression {
                        ty: T::String,
                        node: N::Text(alloc(boxed(operand))?),
                    },
                    expected,
                    root,
                )
            }
            "output" => self.output(root, tokens, expected),
            "ref" => self.reference(root, tokens, expected),
            "instantiate" => self.instantiate(root, &tokens[1..], expected, frames, depth),
            s if s.starts_with("$\"") => self.interpolate(root, tokens, expected, frames, depth),
            _ => self.leaf(root, tokens, expected, frames),
        }
    }

    /// Requires exact invariant result typing without coercion.
    fn checked(value: Expression, expected: Option<&T>, root: &Root) -> Result<Expression, E> {
        if expected.is_some_and(|t| *t != value.ty) {
            return Err(error(ec::EXP_TYPE, Some(root.location)));
        }
        Ok(value)
    }
    /// Gives literals their exact contextual nullable type when explicitly expected.
    fn literal(
        &mut self,
        ty: T,
        value: V,
        expected: Option<&T>,
        root: &Root,
    ) -> Result<Expression, E> {
        let ty = match expected {
            Some(context @ T::Nullable(inner)) if **inner == ty => {
                self.budget.copy_type(context)?
            }
            _ => ty,
        };
        Self::checked(
            Expression {
                ty,
                node: N::Value(value),
            },
            expected,
            root,
        )
    }
    /// Resolves scalar literals, lexical slots and ordinary immutable binding reuse.
    #[allow(clippy::too_many_lines)] // Name resolution and its exact source diagnostic stay together.
    fn leaf(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
        frames: &[Frame<'a>],
    ) -> Result<Expression, E> {
        let first = tokens[0].spelling;
        if tokens.len() == 1 {
            if first.starts_with('"') {
                let value = parser::string(first, &mut self.budget)?;
                let ty = match expected {
                    Some(T::Url) => T::Url,
                    Some(T::Path) => T::Path,
                    _ => T::String,
                };
                let v = match ty {
                    T::Url => V::Url(value),
                    T::Path => V::Path(value),
                    _ => V::String(value),
                };
                return self.literal(ty, v, expected, root);
            }
            if matches!(first, "true" | "false") {
                return self.literal(T::Bool, V::Bool(first == "true"), expected, root);
            }
            if first == "null" {
                let ty = expected
                    .filter(|t| matches!(t, T::Nullable(_)))
                    .ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?;
                return Ok(Expression {
                    ty: self.budget.copy_type(ty)?,
                    node: N::Value(V::Null),
                });
            }
            if first.as_bytes().first().is_some_and(u8::is_ascii_digit) {
                let value = ExactNumber::from_source(
                    first,
                    self.budget.expression.coefficient_digits,
                    self.budget.expression.absolute_scale,
                )
                .map_err(|e| {
                    error(
                        if e == neutral_ir::IrError::ExactNumberLimitExceeded {
                            ec::EXP_LIMIT
                        } else {
                            ec::EXP_SYNTAX
                        },
                        Some(root.location),
                    )
                })?;
                return self.literal(T::Num, V::Number(value), expected, root);
            }
            let mut lexical_depth = 0_u64;
            for frame in frames.iter().rev() {
                for (slot, local) in frame.iter().enumerate() {
                    match local {
                        Local::Bound(name, ty, _) if *name == first => {
                            return Self::checked(
                                Expression {
                                    ty: self.budget.copy_type(ty)?,
                                    node: N::Bound {
                                        depth: lexical_depth,
                                        slot: slot as u64,
                                    },
                                },
                                expected,
                                root,
                            );
                        }
                        Local::Value(name, value) if *name == first => {
                            let mut value =
                                self.budget.copy_expression(value).map_err(|mut e| {
                                    e.location = Some(root.location);
                                    e
                                })?;
                            lift(&mut value, lexical_depth, 0)?;
                            return Self::checked(value, expected, root);
                        }
                        _ => {}
                    }
                }
                if frame.iter().any(|l| matches!(l, Local::Bound(..))) {
                    lexical_depth += 1;
                }
            }
        }
        let mut cursor = 0;
        let (alias, name, span) = parser::name(tokens, &mut cursor)?;
        if cursor != tokens.len() {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        let owner = self
            .resolver
            .name(root, alias, name, span, false)
            .map_err(|e| error(ec::EXP_NAME, e.location))?;
        if self.typing_template
            && self.templates.contains_key(&root.owner)
            && !self.resolver.roots[&owner].public
        {
            return Err(error(ec::TPL_NAME, Some(root.location)));
        }
        let ty = self.budget.copy_type(&self.resolver.types[&owner])?;
        let node = match self.binding(&owner)? {
            ExpressionBody::Input => N::Input(self.budget.copy_symbol(&owner)?),
            ExpressionBody::Value(v) => N::Value(v),
            ExpressionBody::Expression(e) => return Self::checked(e, expected, root),
        };
        Self::checked(Expression { ty, node }, expected, root)
    }
    /// Types a Boolean guard and both exact common-result branches.
    /// Does not evaluate the guard: even a statically skipped branch must resolve
    /// its names and match the invariant result type before normalization.
    fn conditional(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
        frames: &mut Vec<Frame<'a>>,
        depth: u64,
    ) -> Result<Expression, E> {
        let open = first_group(tokens, 1)?;
        let guard = self.expression(root, &tokens[1..open], Some(&T::Bool), frames, depth + 1)?;
        let mut cursor = open;
        let then_tokens = parser::group(tokens, &mut cursor, "{", "}")?;
        parser::expect(tokens, &mut cursor, "else")?;
        let else_tokens = parser::group(tokens, &mut cursor, "{", "}")?;
        if cursor != tokens.len() {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        let then_branch = self.expression(root, then_tokens, expected, frames, depth + 1)?;
        let else_branch =
            self.expression(root, else_tokens, Some(&then_branch.ty), frames, depth + 1)?;
        Self::checked(
            Expression {
                ty: self.budget.copy_type(&then_branch.ty)?,
                node: N::If {
                    guard: alloc(boxed(guard))?,
                    then_branch: alloc(boxed(then_branch))?,
                    else_branch: alloc(boxed(else_branch))?,
                },
            },
            expected,
            root,
        )
    }
    /// Types every closed-variant arm with an isolated payload frame and canonical tags.
    fn matching(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
        frames: &mut Vec<Frame<'a>>,
        depth: u64,
    ) -> Result<Expression, E> {
        let open = first_group(tokens, 1)?;
        let scrutinee = self.expression(root, &tokens[1..open], None, frames, depth + 1)?;
        let B::Variant(alternatives) = self
            .resolver
            .body(&scrutinee.ty)
            .map_err(composition_error)?
        else {
            return Err(error(ec::EXP_TYPE, Some(root.location)));
        };
        let mut cursor = open;
        let members = parser::group(tokens, &mut cursor, "{", "}")?;
        if cursor != tokens.len() {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        let mut arms = Vec::new();
        let mut result = expected.map(|t| self.budget.copy_type(t)).transpose()?;
        for member in parser::split(members)? {
            let Some(first) = member.first() else {
                return Err(error(ec::EXP_MATCH, Some(root.location)));
            };
            let tag = first.spelling;
            let alternative = alternatives
                .iter()
                .find(|a| a.tag == tag)
                .ok_or_else(|| error(ec::EXP_MATCH, Some(root.location)))?;
            if arms.iter().any(|a: &MatchArm| a.tag == tag) {
                return Err(error(ec::EXP_MATCH, Some(root.location)));
            }
            let mut i = 1;
            let binder = parser::group(member, &mut i, "(", ")")?;
            if binder.len() != 1
                || !is_snake_name(binder[0].spelling)
                || is_protected_name(binder[0].spelling)
            {
                return Err(error(ec::EXP_SYNTAX, Some(root.location)));
            }
            parser::expect(member, &mut i, "=>")?;
            let mut frame = Vec::new();
            grow(&mut frame, 1)?;
            frame.push(Local::Bound(
                binder[0].spelling,
                self.budget.copy_type(&alternative.ty)?,
                ParameterPhase::Deferred,
            ));
            grow(frames, 1)?;
            frames.push(frame);
            // Each arm sees its own payload frame; sibling binder names cannot leak across arms.
            let body = self.expression(root, &member[i..], result.as_ref(), frames, depth + 1)?;
            frames.pop();
            if result.is_none() {
                result = Some(self.budget.copy_type(&body.ty)?);
            }
            grow(&mut arms, 1)?;
            arms.push(MatchArm {
                tag: alloc(text(tag))?,
                payload_type: self.budget.copy_type(&alternative.ty)?,
                body,
            });
        }
        if arms.len() != alternatives.len() {
            return Err(error(ec::EXP_MATCH, Some(root.location)));
        }
        arms.sort_unstable_by(|a, b| a.tag.cmp(&b.tag));
        let Some(ty) = result else {
            return Err(error(ec::EXP_MATCH, Some(root.location)));
        };
        Ok(Expression {
            ty,
            node: N::Match {
                scrutinee: alloc(boxed(scrutinee))?,
                arms,
            },
        })
    }
    /// Constructs an ordered homogeneous list; empty lists require a contextual element type.
    fn list(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
        frames: &mut Vec<Frame<'a>>,
        depth: u64,
    ) -> Result<Expression, E> {
        let mut cursor = 0;
        let items = parser::group(tokens, &mut cursor, "[", "]")?;
        if cursor != tokens.len() {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        let inner = match expected {
            Some(T::List(t)) => Some(t.as_ref()),
            Some(T::Nullable(t)) => match t.as_ref() {
                T::List(i) => Some(i.as_ref()),
                _ => None,
            },
            _ => None,
        };
        let mut element = inner.map(|t| self.budget.copy_type(t)).transpose()?;
        let mut values = Vec::new();
        for item in parser::split(items)? {
            let expression = self.expression(root, item, element.as_ref(), frames, depth + 1)?;
            if element.is_none() {
                element = Some(self.budget.copy_type(&expression.ty)?);
            }
            grow(&mut values, 1)?;
            values.push(expression);
        }
        let ty = T::List(alloc(boxed(
            element.ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?,
        ))?);
        let ty = match expected {
            Some(context @ T::Nullable(inner)) if **inner == ty => {
                self.budget.copy_type(context)?
            }
            _ => ty,
        };
        Self::checked(
            Expression {
                ty,
                node: N::List(values),
            },
            expected,
            root,
        )
    }
    /// Constructs exact nominal fields/alternatives and materializes closed defaults.
    fn record(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
        frames: &mut Vec<Frame<'a>>,
        depth: u64,
    ) -> Result<Expression, E> {
        let ty = expected.ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?;
        let nominal = match ty {
            T::Nullable(t) => t.as_ref(),
            t => t,
        };
        let body = self.resolver.body(nominal).map_err(composition_error)?;
        let mut cursor = 0;
        let contents = parser::group(tokens, &mut cursor, "{", "}")?;
        if cursor != tokens.len() {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        let members = parser::split(contents)?;
        let mut supplied = Vec::new();
        for member in members {
            if member.len() < 3
                || member[1].spelling != ":"
                || !parser::identifier(member[0].spelling)
                || supplied
                    .iter()
                    .any(|(name, _): &(&str, &[Lex<'_>])| *name == member[0].spelling)
            {
                return Err(error(ec::EXP_SYNTAX, Some(root.location)));
            }
            grow(&mut supplied, 1)?;
            supplied.push((member[0].spelling, &member[2..]));
        }
        let node = match body {
            B::Variant(alternatives) => {
                if supplied.len() != 2 {
                    return Err(error(ec::EXP_TYPE, Some(root.location)));
                }
                let tag_tokens = supplied
                    .iter()
                    .find(|(n, _)| *n == "tag")
                    .map(|(_, v)| *v)
                    .ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?;
                if tag_tokens.len() != 1 {
                    return Err(error(ec::EXP_TYPE, Some(root.location)));
                }
                let tag = parser::string(tag_tokens[0].spelling, &mut self.budget)?;
                let payload_tokens = supplied
                    .iter()
                    .find(|(n, _)| *n == "payload")
                    .map(|(_, v)| *v)
                    .ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?;
                let payload_type = &alternatives
                    .iter()
                    .find(|a| a.tag == tag)
                    .ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?
                    .ty;
                let payload =
                    self.expression(root, payload_tokens, Some(payload_type), frames, depth + 1)?;
                N::Variant {
                    tag,
                    payload: alloc(boxed(payload))?,
                }
            }
            B::Record(fields) => {
                if supplied
                    .iter()
                    .any(|(n, _)| !fields.iter().any(|f| f.name == *n))
                {
                    return Err(error(ec::EXP_TYPE, Some(root.location)));
                }
                let mut values = Vec::new();
                grow(&mut values, fields.len())?;
                for field in fields {
                    let value = if let Some((_, tokens)) =
                        supplied.iter().find(|(n, _)| *n == field.name)
                    {
                        Some(self.expression(root, tokens, Some(&field.ty), frames, depth + 1)?)
                    } else {
                        match field.presence {
                            FieldPresence::Required => {
                                return Err(error(ec::EXP_TYPE, Some(root.location)));
                            }
                            FieldPresence::Optional => None,
                            FieldPresence::Defaulted => {
                                let Some(default) = field.default.as_ref() else {
                                    return Err(error(ec::EXP_TYPE, Some(root.location)));
                                };
                                Some(Expression {
                                    ty: self.budget.copy_type(&field.ty)?,
                                    node: N::Value(closed_binding(default, &mut self.budget)?),
                                })
                            }
                        }
                    };
                    values.push((field.name, value));
                }
                N::Record(values)
            }
        };
        Ok(Expression {
            ty: self.budget.copy_type(ty)?,
            node,
        })
    }
    /// Retains an identity-only reference without evaluating its target.
    fn reference(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
    ) -> Result<Expression, E> {
        let mut cursor = 1;
        let args = parser::group(tokens, &mut cursor, "(", ")")?;
        if cursor != tokens.len() {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        let mut i = 0;
        let (alias, name, span) = parser::name(args, &mut i)?;
        if i != args.len() {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        let owner = self
            .resolver
            .name(root, alias, name, span, false)
            .map_err(|e| error(ec::EXP_NAME, e.location))?;
        if self.typing_template
            && self.templates.contains_key(&root.owner)
            && !self.resolver.roots[&owner].public
        {
            return Err(error(ec::TPL_NAME, Some(root.location)));
        }
        let ty = T::Ref(alloc(boxed(
            self.budget.copy_type(&self.resolver.types[&owner])?,
        ))?);
        self.literal(ty, V::Reference(owner), expected, root)
    }
    /// Parses explicit future output/result/selector contracts without acquisition.
    #[allow(clippy::too_many_lines)] // Exact typed selector grammar is validated as one source operation.
    fn output(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
    ) -> Result<Expression, E> {
        let mut cursor = 1;
        let type_tokens = parser::group(tokens, &mut cursor, "<", ">")?;
        let Some(source) = self
            .captured
            .sources()
            .iter()
            .find(|s| s.digest() == root.location.source())
        else {
            return Err(error(ec::EXP_NAME, Some(root.location)));
        };
        let syntax = parser::ty(type_tokens, source.bytes())?;
        let ty = self.resolver.ty(root, &syntax).map_err(composition_error)?;
        let args = parser::split(parser::group(tokens, &mut cursor, "(", ")")?)?;
        if cursor != tokens.len() || args.len() != 3 {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        let target_expr = self.reference(root, args[0], None)?;
        let N::Value(V::Reference(target)) = target_expr.node else {
            return Err(error(ec::EXP_TYPE, Some(root.location)));
        };
        if !self.resolver.roots[&target].public
            || !matches!(
                self.resolver.types[&target],
                T::Nominal(_) | T::VocabularyNominal { .. }
            )
        {
            return Err(error(ec::EXP_NAME, Some(root.location)));
        }
        let result_syntax = parser::ty(args[1], source.bytes())?;
        let result_type = self
            .resolver
            .ty(root, &result_syntax)
            .map_err(composition_error)?;
        if !matches!(result_type, T::Nominal(_) | T::VocabularyNominal { .. }) {
            return Err(error(ec::EXP_TYPE, Some(root.location)));
        }
        let mut selector = Vec::new();
        let mut i = 0;
        let segments = parser::group(args[2], &mut i, "[", "]")?;
        if i != args[2].len() {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        for segment in parser::split(segments)? {
            let mut i = 0;
            let fields = parser::group(segment, &mut i, "{", "}")?;
            if i != segment.len() {
                return Err(error(ec::EXP_SYNTAX, Some(root.location)));
            }
            let fields = parser::split(fields)?;
            if fields.len() != 2 {
                return Err(error(ec::EXP_SYNTAX, Some(root.location)));
            }
            let mut kind = None;
            let mut name = None;
            let mut index = None;
            for field in fields {
                if field.len() != 3 || field[1].spelling != ":" {
                    return Err(error(ec::EXP_SYNTAX, Some(root.location)));
                }
                match field[0].spelling {
                    "kind" if kind.is_none() => {
                        kind = Some(parser::string(field[2].spelling, &mut self.budget)?);
                    }
                    "name" if name.is_none() => {
                        name = Some(parser::string(field[2].spelling, &mut self.budget)?);
                    }
                    "index" if index.is_none() => {
                        index = Some(
                            field[2]
                                .spelling
                                .parse::<u32>()
                                .map_err(|_| error(ec::EXP_RESTRICTION, Some(root.location)))?,
                        );
                    }
                    _ => {
                        return Err(error(ec::EXP_SYNTAX, Some(root.location)));
                    }
                }
            }
            let segment = match (kind.as_deref(), name, index) {
                (Some("field"), Some(name), None) => SelectorSegment::Field(name),
                (Some("alternative"), Some(name), None) => SelectorSegment::Alternative(name),
                (Some("index"), None, Some(index)) => SelectorSegment::Index(index),
                _ => {
                    return Err(error(ec::EXP_RESTRICTION, Some(root.location)));
                }
            };
            if selector.len() >= expression_profile::MAX_SELECTOR_SEGMENTS {
                return Err(error(ec::EXP_LIMIT, Some(root.location)));
            }
            grow(&mut selector, 1)?;
            selector.push(segment);
        }
        Self::checked(
            Expression {
                ty,
                node: N::Output {
                    target,
                    result_type,
                    selector,
                },
            },
            expected,
            root,
        )
    }
    /// Parses inserted string expressions with the same contextual expression parser.
    fn interpolate(
        &mut self,
        root: &Root,
        tokens: &[Lex<'a>],
        expected: Option<&T>,
        frames: &mut Vec<Frame<'a>>,
        depth: u64,
    ) -> Result<Expression, E> {
        if tokens.len() != 1 {
            return Err(error(ec::EXP_SYNTAX, Some(root.location)));
        }
        let raw = tokens[0].spelling;
        let bytes = raw.as_bytes();
        let mut items = Vec::new();
        let mut i = 2;
        let mut start = i;
        while i + 1 < bytes.len() {
            self.budget.charge(1)?;
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'{') {
                if i > start {
                    grow(&mut items, 1)?;
                    items.push(Expression {
                        ty: T::String,
                        node: N::Value(V::String(interpolation_text(
                            &raw[start..i],
                            &mut self.budget,
                        )?)),
                    });
                }
                let end = parser::interpolation_end(raw, i + 2, &mut self.budget)?;
                let Ok(offset) = usize::try_from(tokens[0].span.start()) else {
                    return Err(error(ec::EXP_LIMIT, Some(root.location)));
                };
                let Some(offset) = offset.checked_add(i + 2) else {
                    return Err(error(ec::EXP_LIMIT, Some(root.location)));
                };
                let inner = parser::lex(&bytes[i + 2..end], offset, &mut self.budget)?;
                let expression =
                    self.expression(root, &inner, Some(&T::String), frames, depth + 1)?;
                grow(&mut items, 1)?;
                items.push(expression);
                i = end + 1;
                start = i;
            } else {
                i += 1;
            }
        }
        if start < bytes.len() - 1 || items.is_empty() {
            grow(&mut items, 1)?;
            items.push(Expression {
                ty: T::String,
                node: N::Value(V::String(interpolation_text(
                    &raw[start..bytes.len() - 1],
                    &mut self.budget,
                )?)),
            });
        }
        Self::checked(
            Expression {
                ty: T::String,
                node: N::Interpolate(items),
            },
            expected,
            root,
        )
    }
}

/// Frozen precedence and closed operator tags; no host operation dispatch is possible.
fn binary(spelling: &str) -> Option<(u8, Op)> {
    Some(match spelling {
        "||" => (1, Op::Or),
        "&&" => (2, Op::And),
        "==" => (3, Op::Equal),
        "!=" => (3, Op::NotEqual),
        "<" => (4, Op::Less),
        "<=" => (4, Op::LessEqual),
        ">" => (4, Op::Greater),
        ">=" => (4, Op::GreaterEqual),
        "++" => (5, Op::Concat),
        "+" => (6, Op::Add),
        "-" => (6, Op::Subtract),
        "*" => (7, Op::Multiply),
        "/" => (7, Op::Divide),
        _ => return None,
    })
}
/// Lifts only free lexical references; local payload frames keep their original slots.
/// `cutoff` counts frames introduced inside the substituted tree. References below
/// it are locally bound; references at/above it must move outward by `amount`.
fn lift(expression: &mut Expression, amount: u64, cutoff: u64) -> Result<(), E> {
    match &mut expression.node {
        N::Bound { depth, .. } if *depth >= cutoff => {
            *depth = depth
                .checked_add(amount)
                .ok_or_else(|| error(ec::EXP_LIMIT, None))?;
        }
        N::Unary { operand, .. } | N::Text(operand) => lift(operand, amount, cutoff)?,
        N::Binary { left, right, .. } => {
            lift(left, amount, cutoff)?;
            lift(right, amount, cutoff)?;
        }
        N::If {
            guard,
            then_branch,
            else_branch,
        } => {
            lift(guard, amount, cutoff)?;
            lift(then_branch, amount, cutoff)?;
            lift(else_branch, amount, cutoff)?;
        }
        N::Match { scrutinee, arms } => {
            lift(scrutinee, amount, cutoff)?;
            for arm in arms {
                // An arm adds one payload frame; do not shift references owned by that frame.
                lift(&mut arm.body, amount, cutoff + 1)?;
            }
        }
        N::List(items) | N::Interpolate(items) => {
            for item in items {
                lift(item, amount, cutoff)?;
            }
        }
        N::Record(fields) => {
            for (_, item) in fields {
                if let Some(item) = item {
                    lift(item, amount, cutoff)?;
                }
            }
        }
        N::Variant { payload, .. } => lift(payload, amount, cutoff)?,
        _ => {}
    }
    Ok(())
}
/// Finds the first guard/scrutinee body brace outside primary groups.
fn first_group(tokens: &[Lex<'_>], start: usize) -> Result<usize, E> {
    let mut nesting = 0;
    for (i, t) in tokens.iter().enumerate().skip(start) {
        match t.spelling {
            "(" | "[" => nesting += 1,
            ")" | "]" => nesting -= 1,
            "{" if nesting == 0 => return Ok(i),
            _ => {}
        }
    }
    Err(error(ec::EXP_SYNTAX, None))
}
/// Preserves ordinary string escapes plus the frozen escaped interpolation opener.
fn interpolation_text(raw: &str, budget: &mut Budget<'_>) -> Result<String, E> {
    budget.bytes(raw.len() as u64 + 2)?;
    let mut literal = String::new();
    alloc(literal.try_retain(raw.len() + 2))?;
    literal.push('"');
    let mut i = 0;
    while i < raw.len() {
        if raw.as_bytes()[i] == b'\\'
            && raw.as_bytes().get(i + 1) == Some(&b'$')
            && raw.as_bytes().get(i + 2) == Some(&b'{')
        {
            literal.push_str("${");
            i += 3;
        } else {
            let Some(c) = raw[i..].chars().next() else {
                return Err(error(ec::EXP_SYNTAX, None));
            };
            literal.push(c);
            i += c.len_utf8();
        }
    }
    literal.push('"');
    parser::string(&literal, budget)
}
/// Closed defaults retain all optional/null distinctions when entering binding values.
pub(super) fn closed_binding(value: &ClosedValue, budget: &mut Budget<'_>) -> Result<V, E> {
    budget.retain::<V>(1)?;
    Ok(match value {
        ClosedValue::Number(v) => {
            budget.bytes(v.coefficient().len() as u64)?;
            V::Number(alloc(v.try_clone())?)
        }
        ClosedValue::String(v) => V::String(budget.copy_text(v)?),
        ClosedValue::Bool(v) => V::Bool(*v),
        ClosedValue::Url(v) => V::Url(budget.copy_text(v)?),
        ClosedValue::Path(v) => V::Path(budget.copy_text(v)?),
        ClosedValue::Null => V::Null,
        ClosedValue::List(items) => {
            let mut values = Vec::new();
            budget.retain::<V>(items.len())?;
            grow(&mut values, items.len())?;
            for item in items {
                values.push(closed_binding(item, budget)?);
            }
            V::List(values)
        }
        ClosedValue::Record(fields) => {
            let mut values = Vec::new();
            budget.retain::<(String, Option<V>)>(fields.len())?;
            grow(&mut values, fields.len())?;
            for (name, value) in fields {
                values.push((
                    budget.copy_text(name)?,
                    value
                        .as_ref()
                        .map(|value| closed_binding(value, budget))
                        .transpose()?,
                ));
            }
            V::Record(values)
        }
        ClosedValue::Variant { tag, payload } => V::Variant {
            tag: budget.copy_text(tag)?,
            payload: alloc(boxed(closed_binding(payload, budget)?))?,
        },
        ClosedValue::Reference(never) => match *never {},
    })
}

#[cfg(test)]
#[path = "../../../../tests/expression_source/resolution.rs"]
mod tests;
