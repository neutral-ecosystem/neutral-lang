// SPDX-License-Identifier: Apache-2.0

//! Resolve nominal types, closed defaults and shared vocabulary restrictions.
//!
//! The current expression resolver links source and vocabulary contracts and
//! checks record/variant defaults before expression typing. Expansion accounting
//! and cancellation remain owned by the expression request.

use super::{
    B, BTreeMap, ByteSpan, CancellationToken, ClosedValue, CompositionAlternative,
    CompositionDefinition, CompositionError, CompositionField, CompositionValue, E, ExactNumber,
    FieldPresence, FieldRestrictions, LogicalModuleIdentity, ModuleSymbolIdentity,
    PROJECT_MAX_DEPTH, Resolver, RetainCapacity, Root, SourceCompositionDefinition, SourceLocation,
    T, Token, TokenKind, TryClone, TypeExpr, V, V1_SOURCE_PROFILE, boxed, codes, contract, fail,
    graph, parse_name, parse_reference, profile, reserve, retain, split, text,
};

impl Resolver<'_> {
    /// Resolves all source signature owners before any value evaluation.
    pub(in super::super) fn signatures(&mut self, roots: &[Root]) -> Result<(), E> {
        // Resolve every signature before interpreting a default or initializer.
        for root in roots {
            if let Some(ty) = &root.ty {
                let ty = self.ty(root, ty)?;
                self.step(self.types.len() as u64 + 1, Some(root.location))?;
                self.types
                    .insert(retain(root.owner.try_clone())?, ty)
                    .map_err(|_| fail(codes::LIMIT, Some(root.location)))?;
            } else {
                let mut fields = Vec::new();
                let mut alternatives = Vec::new();
                for (name, ty) in &root.fields {
                    let ty = self.ty(root, ty)?;
                    self.step(1, Some(root.location))?;
                    if root.variant {
                        reserve(&mut alternatives, 1)?;
                        alternatives.push(CompositionAlternative {
                            tag: retain(name.try_clone())?,
                            ty,
                        });
                    } else {
                        reserve(&mut fields, 1)?;
                        fields.push(CompositionField {
                            name: retain(name.try_clone())?,
                            ty,
                            presence: FieldPresence::Required,
                            restrictions: FieldRestrictions::default(),
                            default: None,
                        });
                    }
                }
                fields.sort_unstable_by(|a, b| a.name.cmp(&b.name));
                alternatives.sort_unstable_by(|a, b| a.tag.cmp(&b.tag));
                reserve(&mut self.definitions, 1)?;
                self.definitions.push(SourceCompositionDefinition {
                    owner: retain(root.owner.try_clone())?,
                    definition: CompositionDefinition {
                        name: retain(text(root.owner.declaration_name()))?,
                        public: root.public,
                        body: if root.variant {
                            B::Variant(alternatives)
                        } else {
                            B::Record(fields)
                        },
                    },
                });
            }
        }
        Ok(())
    }

    /// Resolves closed source defaults against the complete nominal type set.
    pub(in super::super) fn source_defaults(&mut self, roots: &[Root]) -> Result<(), E> {
        // Parse closed defaults against the complete set of source and vocabulary types.
        let mut defaults = Vec::new();
        for root in roots.iter().filter(|r| r.ty.is_none() && !r.variant) {
            for member in split(&root.tokens)? {
                if let Some(eq) = member
                    .iter()
                    .position(|t| matches!(t.kind, TokenKind::Equals))
                {
                    let Some(Token {
                        kind: TokenKind::Identifier(name),
                        ..
                    }) = member.get(eq.saturating_sub(1))
                    else {
                        return Err(fail(codes::INVALID_SOURCE, Some(root.location)));
                    };
                    let Some(ty) = self
                        .definitions
                        .iter()
                        .find(|d| d.owner == root.owner)
                        .and_then(|d| match &d.definition.body {
                            B::Record(fs) => fs.iter().find(|f| f.name == *name).map(|f| &f.ty),
                            B::Variant(_) => None,
                        })
                    else {
                        return Err(fail(codes::INVALID_SOURCE, Some(root.location)));
                    };
                    let ty = retain(ty.try_clone())?;
                    let value = self.value(root, &ty, &member[eq + 1..], 0)?;
                    reserve(&mut defaults, 1)?;
                    defaults.push((
                        retain(root.owner.try_clone())?,
                        retain(name.try_clone())?,
                        closed(value)?,
                    ));
                }
            }
        }
        for (owner, name, value) in defaults {
            let Some(d) = self.definitions.iter_mut().find(|d| d.owner == owner) else {
                return Err(fail(codes::INVALID_SOURCE, None));
            };
            let B::Record(fields) = &mut d.definition.body else {
                return Err(fail(codes::INVALID_SOURCE, None));
            };
            let Some(f) = fields.iter_mut().find(|f| f.name == name) else {
                return Err(fail(codes::INVALID_SOURCE, None));
            };
            f.presence = FieldPresence::Defaulted;
            f.default = Some(value);
        }
        Ok(())
    }
    /// Charges source/token/key/value work before recursion or proportional materialization.
    pub(in super::super) fn step(
        &mut self,
        n: u64,
        location: Option<SourceLocation>,
    ) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, location));
        }
        self.work = self
            .work
            .checked_add(n)
            .ok_or_else(|| fail(codes::LIMIT, location))?;
        if self.work > self.captured.composition_limits().work {
            return Err(fail(codes::LIMIT, location));
        }
        Ok(())
    }
    /// Resolves explicit local/import/vocabulary qualifiers without path/URL interpretation.
    pub(in super::super) fn name(
        &mut self,
        root: &Root,
        alias: Option<&str>,
        name: &str,
        span: ByteSpan,
        nominal: bool,
    ) -> Result<ModuleSymbolIdentity, E> {
        let module = root.owner.module().module_name();
        self.step(
            (name.len() + alias.map_or(0, str::len)) as u64,
            Some(SourceLocation::new(root.location.source(), span)),
        )?;
        let target = if let Some(alias) = alias {
            self.graph
                .edges()
                .iter()
                .find(|e| e.from() == module && e.alias() == alias)
                .map(graph::Import::target)
                .ok_or_else(|| {
                    fail(
                        codes::INCOMPATIBLE_VALUE,
                        Some(SourceLocation::new(root.location.source(), span)),
                    )
                })?
        } else {
            module
        };
        let owner = ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(retain(text(V1_SOURCE_PROFILE))?, retain(text(target))?),
            retain(text(name))?,
        );
        let declaration = self.roots.get(&owner).ok_or_else(|| {
            fail(
                codes::INCOMPATIBLE_VALUE,
                Some(SourceLocation::new(root.location.source(), span)),
            )
        })?;
        if nominal == declaration.ty.is_some() {
            return Err(fail(codes::INCOMPATIBLE_VALUE, Some(root.location)));
        }
        if target != module && !declaration.public {
            return Err(fail(codes::PRIVATE_EXPOSURE, Some(root.location)));
        }
        Ok(owner)
    }
    /// Converts parsed type wrappers to exact source/vocabulary nominal owners.
    pub(in super::super) fn ty(&mut self, root: &Root, ty: &TypeExpr) -> Result<T, E> {
        self.step(1, Some(root.location))?;
        Ok(match ty {
            TypeExpr::Num => T::Num,
            TypeExpr::String => T::String,
            TypeExpr::Bool => T::Bool,
            TypeExpr::Url => T::Url,
            TypeExpr::Path => T::Path,
            TypeExpr::List(t) => T::List(retain(boxed(self.ty(root, t)?))?),
            TypeExpr::Ref(t) => T::Ref(retain(boxed(self.ty(root, t)?))?),
            TypeExpr::Nullable(t) => T::Nullable(retain(boxed(self.ty(root, t)?))?),
            TypeExpr::Nominal(alias, name, span) => {
                if let Some(v) = alias.as_deref().and_then(|a| {
                    self.captured
                        .vocabulary_for_alias(root.owner.module().module_name(), a)
                }) {
                    let b = self.captured.catalogue();
                    let public = b
                        .bundles()
                        .iter()
                        .find(|b| b.identity == *v)
                        .and_then(|b| b.definitions.iter().find(|d| d.name == *name))
                        .is_some_and(|d| d.public);
                    if !public {
                        return Err(fail(
                            codes::PRIVATE_EXPOSURE,
                            Some(SourceLocation::new(root.location.source(), *span)),
                        ));
                    }
                    T::VocabularyNominal {
                        identity: retain(text(v.identity()))?,
                        version: retain(text(v.version()))?,
                        name: retain(name.try_clone())?,
                    }
                } else {
                    let owner = self.name(root, alias.as_deref(), name, *span, true)?;
                    T::Nominal(owner)
                }
            }
        })
    }
    /// Looks up complete nominal bodies; all alternatives are known before value interpretation.
    pub(in super::super) fn body(&mut self, ty: &T) -> Result<B, E> {
        let catalogue = self.captured.catalogue();
        let body = match ty {
            T::Nominal(owner) => self
                .definitions
                .iter()
                .find(|d| d.owner == *owner)
                .map(|d| &d.definition.body),
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => catalogue
                .bundles()
                .iter()
                .find(|b| b.identity.identity() == identity && b.identity.version() == version)
                .and_then(|b| b.definitions.iter().find(|d| d.name == *name))
                .map(|d| &d.body),
            _ => None,
        }
        .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, None))?;
        let work = body_clone_work(body, self.cancel)?;
        // Charge before retaining a recursive copy; repeated nominal use must not amplify unchecked allocation.
        let remaining = self
            .captured
            .composition_limits()
            .work
            .saturating_sub(self.work);
        if work > remaining {
            return Err(fail(codes::LIMIT, None));
        }
        let Some(work) = self.work.checked_add(work) else {
            return Err(fail(codes::LIMIT, None));
        };
        self.work = work;
        let copied = retain(body.try_clone())?;
        self.step(1, None)?;
        Ok(copied)
    }
    /// Types exact scalar literals without host numeric or inert-location coercion.
    pub(in super::super) fn scalar(
        &self,
        ty: &T,
        token: &Token,
        location: SourceLocation,
    ) -> Result<V, E> {
        match &token.kind {
            TokenKind::Null => {
                if matches!(ty, T::Nullable(_)) {
                    Ok(V::Null)
                } else {
                    Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)))
                }
            }
            TokenKind::True | TokenKind::False => {
                let inner = if let T::Nullable(inner) = ty {
                    inner.as_ref()
                } else {
                    ty
                };
                if !matches!(inner, T::Bool) {
                    return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
                }
                Ok(V::Bool(matches!(token.kind, TokenKind::True)))
            }
            TokenKind::Number(s) => {
                let inner = if let T::Nullable(inner) = ty {
                    inner.as_ref()
                } else {
                    ty
                };
                if !matches!(inner, T::Num) {
                    return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
                }
                ExactNumber::from_source(
                    s,
                    self.captured.composition_limits().json.numeric_digits(),
                    self.captured.composition_limits().json.numeric_scale(),
                )
                .map(V::Number)
                .map_err(|_| fail(codes::LIMIT, Some(location)))
            }
            TokenKind::StringLiteral(s) => {
                let inner = if let T::Nullable(inner) = ty {
                    inner.as_ref()
                } else {
                    ty
                };
                if !matches!(inner, T::String | T::Url | T::Path) {
                    return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
                }
                Ok(match ty {
                    T::Url => V::Url(retain(s.value.try_clone())?),
                    T::Path => V::Path(retain(s.value.try_clone())?),
                    T::Nullable(inner) if matches!(**inner, T::Url) => {
                        V::Url(retain(s.value.try_clone())?)
                    }
                    T::Nullable(inner) if matches!(**inner, T::Path) => {
                        V::Path(retain(s.value.try_clone())?)
                    }
                    _ => V::String(retain(s.value.try_clone())?),
                })
            }
            _ => Err(fail(codes::INVALID_SOURCE, Some(location))),
        }
    }
    /// Indexes borrowed object members with charged shifts and fallible storage.
    pub(in super::super) fn record_members<'a>(
        &mut self,
        tokens: &'a [Token],
        location: SourceLocation,
    ) -> Result<BTreeMap<&'a str, &'a [Token]>, E> {
        let mut members = BTreeMap::new();
        for field in split(&tokens[1..tokens.len() - 1])? {
            let [
                Token {
                    kind: TokenKind::Identifier(name),
                    ..
                },
                Token {
                    kind: TokenKind::Colon,
                    ..
                },
                rest @ ..,
            ] = field
            else {
                return Err(fail(codes::INVALID_SOURCE, Some(location)));
            };
            self.step(members.len() as u64 + 1, Some(location))?;
            if members
                .insert(name.as_str(), rest)
                .map_err(|_| fail(codes::LIMIT, Some(location)))?
                .is_some()
            {
                return Err(fail(codes::DUPLICATE_MEMBER, Some(location)));
            }
        }
        Ok(members)
    }

    /// Types a closed contextual record/variant object with complete member checks.
    pub(in super::super) fn record_value(
        &mut self,
        root: &Root,
        ty: &T,
        tokens: &[Token],
        depth: usize,
    ) -> Result<V, E> {
        let location = SourceLocation::new(root.location.source(), tokens[0].span);
        let members = self.record_members(tokens, location)?;
        match self.body(ty)? {
            B::Record(fields) => {
                let mut value = Vec::new();
                for (name, tokens) in members {
                    let field = fields
                        .iter()
                        .find(|f| f.name == name)
                        .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, Some(location)))?;
                    let v = self.value(root, &field.ty, tokens, depth + 1)?;
                    if field.restrictions != FieldRestrictions::default() {
                        neutral_vocabulary::composition::check_composition_field_restrictions(
                            field,
                            &v,
                            self.captured.composition_limits(),
                            self.cancel,
                        )
                        .map_err(|e| {
                            if e == CompositionError::InvalidValue {
                                fail(codes::RESTRICTION_VIOLATION, Some(location))
                            } else {
                                contract(&e, Some(location))
                            }
                        })?;
                    }
                    reserve(&mut value, 1)?;
                    value.push((retain(text(name))?, Some(v)));
                }
                Ok(V::Record(value))
            }
            B::Variant(alternatives) => {
                if members.len() != 2
                    || !members.contains_key(profile::TAG)
                    || !members.contains_key(profile::PAYLOAD)
                {
                    return Err(fail(codes::DUPLICATE_MEMBER, Some(location)));
                }
                let [
                    Token {
                        kind: TokenKind::StringLiteral(tag),
                        span,
                    },
                ] = members[profile::TAG]
                else {
                    return Err(fail(codes::INVALID_SOURCE, Some(location)));
                };
                let alternative = alternatives
                    .iter()
                    .find(|a| a.tag == tag.value)
                    .ok_or_else(|| {
                        fail(
                            codes::UNKNOWN_TAG,
                            Some(SourceLocation::new(root.location.source(), *span)),
                        )
                    })?;
                let payload =
                    self.value(root, &alternative.ty, members[profile::PAYLOAD], depth + 1)?;
                Ok(V::Variant {
                    tag: retain(tag.value.try_clone())?,
                    payload: retain(boxed(payload))?,
                })
            }
        }
    }

    /// Parses closed default values and exact selected variant payloads using shared contracts.
    pub(in super::super) fn value(
        &mut self,
        root: &Root,
        ty: &T,
        tokens: &[Token],
        depth: usize,
    ) -> Result<V, E> {
        self.step(tokens.len() as u64 + 1, Some(root.location))?;
        if depth as u64 > self.captured.composition_limits().value_depth
            || depth > PROJECT_MAX_DEPTH
        {
            return Err(fail(codes::LIMIT, Some(root.location)));
        }
        let location = tokens.first().map_or(root.location, |t| {
            SourceLocation::new(root.location.source(), t.span)
        });
        if tokens.is_empty() {
            return Err(fail(codes::INVALID_SOURCE, Some(root.location)));
        }
        if parse_reference(tokens, 0) == Some(tokens.len()) {
            return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
        }
        if parse_name(tokens, 0) == Some(tokens.len()) {
            return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
        }
        if let [token] = tokens
            && matches!(
                token.kind,
                TokenKind::Null
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::Number(_)
                    | TokenKind::StringLiteral(_)
            )
        {
            return self.scalar(ty, token, location);
        }
        if let T::Nullable(inner) = ty {
            return self.value(root, inner, tokens, depth);
        }
        if matches!(tokens[0].kind, TokenKind::OpenBracket)
            && matches!(
                tokens.last().map(|t| &t.kind),
                Some(TokenKind::CloseBracket)
            )
        {
            let T::List(inner) = ty else {
                return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
            };
            let mut values = Vec::new();
            for item in split(&tokens[1..tokens.len() - 1])? {
                reserve(&mut values, 1)?;
                values.push(self.value(root, inner, item, depth + 1)?);
            }
            return Ok(V::List(values));
        }
        if matches!(tokens[0].kind, TokenKind::OpenBrace)
            && matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::CloseBrace))
        {
            return self.record_value(root, ty, tokens, depth);
        }
        Err(fail(codes::INVALID_SOURCE, Some(location)))
    }
}

/// Counts stable owner text before retaining a copied reference key.
fn symbol_work(owner: &ModuleSymbolIdentity) -> u64 {
    (owner.module().language_behavior_version().len()
        + owner.module().module_name().len()
        + owner.declaration_name().len()) as u64
        + 1
}

/// Bounds recursive value copies iteratively before reuse/default expansion can allocate them.
fn value_clone_work<R>(
    value: &CompositionValue<R>,
    cancel: &CancellationToken,
    reference: impl Fn(&R) -> u64,
) -> Result<u64, E> {
    if cancel.is_cancelled() {
        return Err(fail(codes::CANCELLED, None));
    }
    let mut work = 0_u64;
    let mut stack = Vec::new();
    reserve(&mut stack, 1)?;
    stack.push(value);
    while let Some(value) = stack.pop() {
        if cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, None));
        }
        let extra = match value {
            CompositionValue::Number(n) => n.coefficient().len() as u64,
            CompositionValue::String(s) | CompositionValue::Url(s) | CompositionValue::Path(s) => {
                s.len() as u64
            }
            CompositionValue::Reference(target) => reference(target),
            CompositionValue::List(items) => {
                stack
                    .try_retain(items.len())
                    .map_err(|_| fail(codes::LIMIT, None))?;
                stack.extend(items);
                items.len() as u64
            }
            CompositionValue::Record(fields) => {
                stack
                    .try_retain(fields.len())
                    .map_err(|_| fail(codes::LIMIT, None))?;
                let mut bytes = 0_u64;
                for (name, value) in fields {
                    let Some(next) = bytes.checked_add(name.len() as u64 + 1) else {
                        return Err(fail(codes::LIMIT, None));
                    };
                    bytes = next;
                    if let Some(value) = value {
                        stack.push(value);
                    }
                }
                bytes
            }
            CompositionValue::Variant { tag, payload } => {
                reserve(&mut stack, 1)?;
                stack.push(payload);
                tag.len() as u64
            }
            CompositionValue::Null | CompositionValue::Bool(_) => 0,
        };
        let Some(next) = work.checked_add(extra).and_then(|n| n.checked_add(1)) else {
            return Err(fail(codes::LIMIT, None));
        };
        work = next;
        if work > profile::MAX_ITEMS {
            return Err(fail(codes::LIMIT, None));
        }
    }
    Ok(work)
}

/// Accounts for wrapper depth and all nominal owner text before cloning a type.
fn type_clone_work(mut ty: &T) -> u64 {
    let mut work = 1;
    loop {
        match ty {
            T::List(inner) | T::Ref(inner) | T::Nullable(inner) => {
                work += 1;
                ty = inner;
            }
            T::Nominal(owner) => return work + symbol_work(owner),
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => return work + (identity.len() + version.len() + name.len()) as u64,
            _ => return work,
        }
    }
}

/// Charges copied contract members, choices and defaults independently of later materialization.
fn body_clone_work(body: &B, cancel: &CancellationToken) -> Result<u64, E> {
    let mut work = 1_u64;
    match body {
        B::Record(fields) => {
            for field in fields {
                let Some(next) =
                    work.checked_add(field.name.len() as u64 + type_clone_work(&field.ty) + 1)
                else {
                    return Err(fail(codes::LIMIT, None));
                };
                work = next;
                for value in field
                    .default
                    .iter()
                    .chain(field.restrictions.choices.iter().flatten())
                {
                    let Some(next) =
                        work.checked_add(value_clone_work(value, cancel, |never| match *never {})?)
                    else {
                        return Err(fail(codes::LIMIT, None));
                    };
                    work = next;
                }
                for bound in [&field.restrictions.minimum, &field.restrictions.maximum]
                    .into_iter()
                    .flatten()
                {
                    let Some(next) = work.checked_add(bound.coefficient().len() as u64) else {
                        return Err(fail(codes::LIMIT, None));
                    };
                    work = next;
                }
            }
        }
        B::Variant(alternatives) => {
            for alternative in alternatives {
                let Some(next) = work.checked_add(
                    alternative.tag.len() as u64 + type_clone_work(&alternative.ty) + 1,
                ) else {
                    return Err(fail(codes::LIMIT, None));
                };
                work = next;
            }
        }
    }
    Ok(work)
}

/// Converts parser-produced closed values while making identity references unrepresentable.
pub(in super::super) fn closed(v: V) -> Result<ClosedValue, E> {
    Ok(match v {
        V::Number(n) => CompositionValue::Number(n),
        V::String(s) => CompositionValue::String(s),
        V::Bool(b) => CompositionValue::Bool(b),
        V::Url(s) => CompositionValue::Url(s),
        V::Path(s) => CompositionValue::Path(s),
        V::Null => CompositionValue::Null,
        V::Reference(_) => return Err(fail(codes::INCOMPATIBLE_VALUE, None)),
        V::List(vs) => {
            let mut result = Vec::new();
            reserve(&mut result, vs.len())?;
            for v in vs {
                result.push(closed(v)?);
            }
            CompositionValue::List(result)
        }
        V::Record(fs) => {
            let mut result = Vec::new();
            reserve(&mut result, fs.len())?;
            for (n, v) in fs {
                result.push((n, v.map(closed).transpose()?));
            }
            CompositionValue::Record(result)
        }
        V::Variant { tag, payload } => CompositionValue::Variant {
            tag,
            payload: retain(boxed(closed(*payload)?))?,
        },
    })
}

#[cfg(test)]
#[path = "../../../../tests/nominal/resolver.rs"]
mod tests;
