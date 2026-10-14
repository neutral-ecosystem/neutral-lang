// SPDX-License-Identifier: Apache-2.0

//! All-branch typing and stable-name lowering; ordinary reuse is materialized.
//!
//! Binding resolution memoizes request-local meaning and detects cycles before
//! evaluation. Every branch resolves under explicit lexical frames, including
//! branches that static folding will skip. Stable symbols identify dependencies;
//! source aliases never become semantic identities or host lookup instructions.

use super::{
    B, Budget, ClosedValue, E, ExactNumber, Expression, ExpressionBody, FieldPresence, Frame, Lex,
    Local, MatchArm, ModuleSymbolIdentity, N, Op, ParameterPhase, RetainCapacity, Root,
    SelectorSegment, State, T, TryClone, UnaryOperator, V, alloc, boxed, composition_error,
    copy_slice, error, expression_profile, grow, is_protected_name, is_snake_name, parser, text,
};
use crate::frontend::expression_codes as ec;

impl<'a> State<'a, '_> {
    /// Resolves ordinary reuse once per request and rejects value cycles before evaluation.
    pub(super) fn binding(&mut self, owner: &ModuleSymbolIdentity) -> Result<ExpressionBody, E> {
        self.budget.charge(1)?;
        if let Some(body) = self.bodies.get(owner) {
            return Ok(match body {
                ExpressionBody::Input => ExpressionBody::Input,
                ExpressionBody::Value(v) => ExpressionBody::Value(
                    neutral_ir::expression::try_clone_value_bounded(
                        v,
                        self.budget.expression,
                        self.budget.cancel,
                    )
                    .map_err(|_| error(ec::EXP_LIMIT, None))?,
                ),
                ExpressionBody::Expression(e) => ExpressionBody::Expression(
                    e.try_clone_bounded(self.budget.expression, self.budget.cancel)
                        .map_err(|_| error(ec::EXP_LIMIT, None))?,
                ),
            });
        }
        if self.active.len() as u64 >= self.budget.expression.syntax_depth
            || !alloc(self.active.insert(alloc(owner.try_clone())?))?
        {
            return Err(error(ec::EXP_CYCLE, None));
        }
        let root = alloc(
            self.resolver
                .roots
                .get(owner)
                .ok_or_else(|| error(ec::EXP_NAME, None))?
                .try_clone(),
        )?;
        let syntax = self
            .bindings
            .get(owner)
            .ok_or_else(|| error(ec::EXP_NAME, Some(root.location)))?;
        let input = syntax.input;
        let tokens = alloc(copy_slice(&syntax.tokens))?;
        let mut axes = Vec::new();
        grow(&mut axes, syntax.axes.len())?;
        for (name, value) in &syntax.axes {
            axes.push(Local::Value(
                name,
                value
                    .try_clone_bounded(self.budget.expression, self.budget.cancel)
                    .map_err(|_| error(ec::EXP_LIMIT, Some(root.location)))?,
            ));
        }
        let ty = alloc(
            self.resolver
                .types
                .get(owner)
                .ok_or_else(|| error(ec::EXP_NAME, Some(root.location)))?
                .try_clone(),
        )?;
        let mut frames = Vec::new();
        if !axes.is_empty() {
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
        let retained = self.copy_body(&body)?;
        alloc(self.bodies.insert(alloc(owner.try_clone())?, retained))?;
        Ok(body)
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
                    alloc(left.ty.try_clone())?
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
    fn literal(ty: T, value: V, expected: Option<&T>, root: &Root) -> Result<Expression, E> {
        let ty = if expected.is_some_and(|t| matches!(t,T::Nullable(inner) if **inner==ty)) {
            alloc(
                expected
                    .ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?
                    .try_clone(),
            )?
        } else {
            ty
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
                return Self::literal(ty, v, expected, root);
            }
            if matches!(first, "true" | "false") {
                return Self::literal(T::Bool, V::Bool(first == "true"), expected, root);
            }
            if first == "null" {
                let ty = expected
                    .filter(|t| matches!(t, T::Nullable(_)))
                    .ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?;
                return Ok(Expression {
                    ty: alloc(ty.try_clone())?,
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
                return Self::literal(T::Num, V::Number(value), expected, root);
            }
            let mut lexical_depth = 0_u64;
            for frame in frames.iter().rev() {
                for (slot, local) in frame.iter().enumerate() {
                    match local {
                        Local::Bound(name, ty, _) if *name == first => {
                            return Self::checked(
                                Expression {
                                    ty: alloc(ty.try_clone())?,
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
                            let mut value = value
                                .try_clone_bounded(self.budget.expression, self.budget.cancel)
                                .map_err(|_| error(ec::EXP_LIMIT, Some(root.location)))?;
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
        let ty = alloc(self.resolver.types[&owner].try_clone())?;
        let node = match self.binding(&owner)? {
            ExpressionBody::Input => N::Input(alloc(owner.try_clone())?),
            ExpressionBody::Value(v) => N::Value(v),
            ExpressionBody::Expression(e) => return Self::checked(e, expected, root),
        };
        Self::checked(Expression { ty, node }, expected, root)
    }
    /// Types a Boolean guard and both exact common-result branches.
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
                ty: alloc(then_branch.ty.try_clone())?,
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
        let mut result = expected.map(|t| alloc(t.try_clone())).transpose()?;
        for member in parser::split(members)? {
            let tag = member
                .first()
                .ok_or_else(|| error(ec::EXP_MATCH, Some(root.location)))?
                .spelling;
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
                alloc(alternative.ty.try_clone())?,
                ParameterPhase::Deferred,
            ));
            grow(frames, 1)?;
            frames.push(frame);
            let body = self.expression(root, &member[i..], result.as_ref(), frames, depth + 1)?;
            frames.pop();
            if result.is_none() {
                result = Some(alloc(body.ty.try_clone())?);
            }
            grow(&mut arms, 1)?;
            arms.push(MatchArm {
                tag: alloc(text(tag))?,
                payload_type: alloc(alternative.ty.try_clone())?,
                body,
            });
        }
        if arms.len() != alternatives.len() {
            return Err(error(ec::EXP_MATCH, Some(root.location)));
        }
        arms.sort_unstable_by(|a, b| a.tag.cmp(&b.tag));
        Ok(Expression {
            ty: result.ok_or_else(|| error(ec::EXP_MATCH, Some(root.location)))?,
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
        let mut element = inner.map(|t| alloc(t.try_clone())).transpose()?;
        let mut values = Vec::new();
        for item in parser::split(items)? {
            let expression = self.expression(root, item, element.as_ref(), frames, depth + 1)?;
            if element.is_none() {
                element = Some(alloc(expression.ty.try_clone())?);
            }
            grow(&mut values, 1)?;
            values.push(expression);
        }
        let ty = T::List(alloc(boxed(
            element.ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?,
        ))?);
        let ty = if expected.is_some_and(|t| matches!(t,T::Nullable(inner) if **inner==ty)) {
            alloc(
                expected
                    .ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?
                    .try_clone(),
            )?
        } else {
            ty
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
                            FieldPresence::Defaulted => Some(Expression {
                                ty: alloc(field.ty.try_clone())?,
                                node: N::Value(closed_binding(
                                    field
                                        .default
                                        .as_ref()
                                        .ok_or_else(|| error(ec::EXP_TYPE, Some(root.location)))?,
                                )?),
                            }),
                        }
                    };
                    values.push((field.name, value));
                }
                N::Record(values)
            }
        };
        Ok(Expression {
            ty: alloc(ty.try_clone())?,
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
        let ty = T::Ref(alloc(boxed(alloc(
            self.resolver.types[&owner].try_clone(),
        )?))?);
        Self::literal(ty, V::Reference(owner), expected, root)
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
        let source = self
            .captured
            .sources()
            .iter()
            .find(|s| s.digest() == root.location.source())
            .ok_or_else(|| error(ec::EXP_NAME, Some(root.location)))?;
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
            let segment = match kind.as_deref() {
                Some("field") if index.is_none() => SelectorSegment::Field(
                    name.ok_or_else(|| error(ec::EXP_RESTRICTION, Some(root.location)))?,
                ),
                Some("alternative") if index.is_none() => SelectorSegment::Alternative(
                    name.ok_or_else(|| error(ec::EXP_RESTRICTION, Some(root.location)))?,
                ),
                Some("index") if name.is_none() => SelectorSegment::Index(
                    index.ok_or_else(|| error(ec::EXP_RESTRICTION, Some(root.location)))?,
                ),
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
                let inner = parser::lex(
                    &bytes[i + 2..end],
                    usize::try_from(tokens[0].span.start())
                        .map_err(|_| error(ec::EXP_LIMIT, Some(root.location)))?
                        .checked_add(i + 2)
                        .ok_or_else(|| error(ec::EXP_LIMIT, Some(root.location)))?,
                    &mut self.budget,
                )?;
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
            let c = raw[i..]
                .chars()
                .next()
                .ok_or_else(|| error(ec::EXP_SYNTAX, None))?;
            literal.push(c);
            i += c.len_utf8();
        }
    }
    literal.push('"');
    parser::string(&literal, budget)
}
/// Closed defaults retain all optional/null distinctions when entering binding values.
pub(super) fn closed_binding(value: &ClosedValue) -> Result<V, E> {
    Ok(match value {
        ClosedValue::Number(v) => V::Number(alloc(v.try_clone())?),
        ClosedValue::String(v) => V::String(alloc(v.try_clone())?),
        ClosedValue::Bool(v) => V::Bool(*v),
        ClosedValue::Url(v) => V::Url(alloc(v.try_clone())?),
        ClosedValue::Path(v) => V::Path(alloc(v.try_clone())?),
        ClosedValue::Null => V::Null,
        ClosedValue::List(items) => {
            let mut values = Vec::new();
            grow(&mut values, items.len())?;
            for item in items {
                values.push(closed_binding(item)?);
            }
            V::List(values)
        }
        ClosedValue::Record(fields) => {
            let mut values = Vec::new();
            grow(&mut values, fields.len())?;
            for (name, value) in fields {
                values.push((
                    alloc(name.try_clone())?,
                    value.as_ref().map(closed_binding).transpose()?,
                ));
            }
            V::Record(values)
        }
        ClosedValue::Variant { tag, payload } => V::Variant {
            tag: alloc(tag.try_clone())?,
            payload: alloc(boxed(closed_binding(payload)?))?,
        },
        ClosedValue::Reference(never) => match *never {},
    })
}
