// SPDX-License-Identifier: Apache-2.0

//! Resolve names, types and value dependencies across captured modules.
//!
//! Import aliases select visible symbols; nominal identities determine type
//! compatibility. Public closure and exposed-reference checks prevent private
//! targets leaking through exports, while dependency walks detect semantic cycles.
//! Source occurrences are retained for diagnostics; no path or URL is acquired.

use super::{
    BTreeMap, BTreeSet, ByteSpan, ModuleContext, NameOccurrence, ProjectDependency,
    ProjectDependencyKind, ProjectSemanticDiagnostic, ProjectSymbolKind, ProjectVocabularySet,
    Root, SourceContentDigest, SourceLocation, Token, TokenKind, TypeExpr, diagnostic, diagnostics,
    key,
};

/// Resolves nominal type occurrences and public signature closure.
#[expect(
    clippy::too_many_arguments,
    reason = "resolution requires explicit source, declaration, and result sinks"
)]
pub(super) fn resolve_type(
    ty: &TypeExpr,
    owner: &(String, String),
    root: &Root,
    module: &ModuleContext,
    roots: &BTreeMap<(String, String), Root>,
    vocabularies: &ProjectVocabularySet,
    edges: &mut Vec<ProjectDependency>,
    errors: &mut Vec<ProjectSemanticDiagnostic>,
    under_ref: bool,
) {
    match ty {
        TypeExpr::Nominal(alias, name, span) => {
            if let Some(vocabulary) = alias
                .as_ref()
                .and_then(|alias| vocabularies.resolve(&owner.0, alias))
            {
                if vocabulary.public_type(name).is_none() {
                    errors.push(diagnostic(
                        diagnostics::PRIVATE_VOCABULARY_TYPE,
                        &owner.0,
                        module.digest,
                        *span,
                    ));
                }
                return;
            }
            let occurrence = NameOccurrence {
                alias: alias.clone(),
                name: name.clone(),
                span: *span,
            };
            if let Some(target) = resolve_name(&occurrence, owner, module, roots, errors) {
                let target_root = &roots[&target];
                if target_root.symbol.kind == ProjectSymbolKind::Record {
                    if root.symbol.is_public && !target_root.symbol.is_public {
                        errors.push(diagnostic(
                            diagnostics::PRIVATE_PUBLIC_TYPE,
                            &owner.0,
                            module.digest,
                            *span,
                        ));
                    }
                    let kind = if under_ref {
                        ProjectDependencyKind::ReferenceType
                    } else {
                        ProjectDependencyKind::Type
                    };
                    edges.push(dependency(root, target_root, kind, module.digest, *span));
                } else {
                    errors.push(diagnostic(
                        diagnostics::INACCESSIBLE_NAME,
                        &owner.0,
                        module.digest,
                        *span,
                    ));
                }
            }
        }
        TypeExpr::List(inner) | TypeExpr::Nullable(inner) => {
            resolve_type(
                inner,
                owner,
                root,
                module,
                roots,
                vocabularies,
                edges,
                errors,
                under_ref,
            );
        }
        TypeExpr::Ref(inner) => {
            resolve_type(
                inner,
                owner,
                root,
                module,
                roots,
                vocabularies,
                edges,
                errors,
                true,
            );
        }
        _ => {}
    }
}

/// Resolves every value and reference occurrence within one immutable binding.
#[expect(
    clippy::too_many_lines,
    reason = "value and identity references share one ordered token walk"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "explicit context prevents ambient resolution"
)]
pub(super) fn resolve_value(
    root: &Root,
    owner: &(String, String),
    module: &ModuleContext,
    modules: &BTreeMap<String, ModuleContext>,
    roots: &BTreeMap<(String, String), Root>,
    vocabularies: &ProjectVocabularySet,
    edges: &mut Vec<ProjectDependency>,
    errors: &mut Vec<ProjectSemanticDiagnostic>,
) {
    let tokens = &root.value;
    let mut index = 0;
    while index < tokens.len() {
        if matches!(tokens[index].kind, TokenKind::RefValue) {
            let Some((occurrence, end)) = parse_reference(tokens, index) else {
                errors.push(diagnostic(
                    diagnostics::INVALID_REFERENCE,
                    &owner.0,
                    module.digest,
                    tokens[index].span,
                ));
                index += 1;
                continue;
            };
            if let Some(target) = resolve_name(&occurrence, owner, module, roots, errors) {
                let target_root = &roots[&target];
                if target_root.symbol.kind == ProjectSymbolKind::Binding {
                    if root.symbol.is_public && !target_root.symbol.is_public {
                        errors.push(diagnostic(
                            diagnostics::PRIVATE_REFERENCE,
                            &owner.0,
                            module.digest,
                            occurrence.span,
                        ));
                    }
                    if tokens.len() == end && index == 0 {
                        if let Some(TypeExpr::Ref(inner)) = &root.declared_type {
                            if !target_root
                                .declared_type
                                .as_ref()
                                .is_some_and(|target_type| {
                                    same_type(
                                        inner,
                                        &owner.0,
                                        target_type,
                                        &target.0,
                                        modules,
                                        vocabularies,
                                    )
                                })
                            {
                                errors.push(diagnostic(
                                    diagnostics::TYPE_MISMATCH,
                                    &owner.0,
                                    module.digest,
                                    occurrence.span,
                                ));
                            }
                        } else {
                            errors.push(diagnostic(
                                diagnostics::TYPE_MISMATCH,
                                &owner.0,
                                module.digest,
                                occurrence.span,
                            ));
                        }
                    }
                    edges.push(dependency(
                        root,
                        target_root,
                        ProjectDependencyKind::Reference,
                        module.digest,
                        occurrence.span,
                    ));
                } else {
                    errors.push(diagnostic(
                        diagnostics::INVALID_REFERENCE,
                        &owner.0,
                        module.digest,
                        occurrence.span,
                    ));
                }
            }
            index = end;
            continue;
        }
        if let Some((occurrence, end)) = parse_name(tokens, index) {
            if !matches!(
                tokens.get(end).map(|token| &token.kind),
                Some(TokenKind::Colon)
            ) && let Some(target) = resolve_name(&occurrence, owner, module, roots, errors)
            {
                let target_root = &roots[&target];
                if target_root.symbol.kind == ProjectSymbolKind::Binding {
                    if index == 0
                        && end == tokens.len()
                        && !root
                            .declared_type
                            .as_ref()
                            .zip(target_root.declared_type.as_ref())
                            .is_some_and(|(left, right)| {
                                same_type(left, &owner.0, right, &target.0, modules, vocabularies)
                            })
                    {
                        errors.push(diagnostic(
                            diagnostics::TYPE_MISMATCH,
                            &owner.0,
                            module.digest,
                            occurrence.span,
                        ));
                    }
                    edges.push(dependency(
                        root,
                        target_root,
                        ProjectDependencyKind::Value,
                        module.digest,
                        occurrence.span,
                    ));
                } else {
                    errors.push(diagnostic(
                        diagnostics::INACCESSIBLE_NAME,
                        &owner.0,
                        module.digest,
                        occurrence.span,
                    ));
                }
            }
            index = end;
        } else {
            index += 1;
        }
    }
}

/// Compares types by resolved nominal module identity, never alias spelling.
fn same_type(
    left: &TypeExpr,
    left_module: &str,
    right: &TypeExpr,
    right_module: &str,
    modules: &BTreeMap<String, ModuleContext>,
    vocabularies: &ProjectVocabularySet,
) -> bool {
    match (left, right) {
        (TypeExpr::Num, TypeExpr::Num)
        | (TypeExpr::String, TypeExpr::String)
        | (TypeExpr::Bool, TypeExpr::Bool)
        | (TypeExpr::Url, TypeExpr::Url)
        | (TypeExpr::Path, TypeExpr::Path) => true,
        (
            TypeExpr::Nominal(left_alias, left_name, _),
            TypeExpr::Nominal(right_alias, right_name, _),
        ) => {
            let left_vocabulary = left_alias
                .as_ref()
                .and_then(|alias| vocabularies.resolve(left_module, alias));
            let right_vocabulary = right_alias
                .as_ref()
                .and_then(|alias| vocabularies.resolve(right_module, alias));
            if left_vocabulary.is_some() || right_vocabulary.is_some() {
                return left_vocabulary
                    .zip(right_vocabulary)
                    .is_some_and(|(left, right)| {
                        left.identity() == right.identity()
                            && left.version() == right.version()
                            && left_name == right_name
                    });
            }
            let left_owner = left_alias
                .as_ref()
                .and_then(|alias| modules[left_module].aliases.get(alias))
                .map_or(left_module, String::as_str);
            let right_owner = right_alias
                .as_ref()
                .and_then(|alias| modules[right_module].aliases.get(alias))
                .map_or(right_module, String::as_str);
            left_owner == right_owner && left_name == right_name
        }
        (TypeExpr::List(left), TypeExpr::List(right))
        | (TypeExpr::Ref(left), TypeExpr::Ref(right))
        | (TypeExpr::Nullable(left), TypeExpr::Nullable(right)) => same_type(
            left,
            left_module,
            right,
            right_module,
            modules,
            vocabularies,
        ),
        _ => false,
    }
}

/// Finds a private reference reachable by ordinary reuse from a public root.
pub(super) fn exposed_private_reference(
    roots: &BTreeMap<(String, String), Root>,
    edges: &[ProjectDependency],
) -> Option<(String, String)> {
    let mut queue = roots
        .iter()
        .filter(|(_, root)| root.symbol.is_public && root.symbol.kind == ProjectSymbolKind::Binding)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    while let Some(current) = queue.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        for edge in edges.iter().filter(|edge| key(edge.from()) == current) {
            let target = key(edge.to());
            if edge.kind == ProjectDependencyKind::Reference && !roots[&target].symbol.is_public {
                return Some(current);
            }
            if edge.kind == ProjectDependencyKind::Value {
                queue.push(target);
            }
        }
    }
    None
}

/// Parses a `ref(name)` or `ref(alias::name)` target at one token offset.
pub(super) fn parse_reference(tokens: &[Token], index: usize) -> Option<(NameOccurrence, usize)> {
    if !matches!(tokens.get(index + 1)?.kind, TokenKind::OpenParen) {
        return None;
    }
    let (name, end) = parse_name(tokens, index + 2)?;
    matches!(tokens.get(end)?.kind, TokenKind::CloseParen).then_some((name, end + 1))
}

/// Parses one unqualified or alias-qualified name occurrence.
pub(super) fn parse_name(tokens: &[Token], index: usize) -> Option<(NameOccurrence, usize)> {
    let first = tokens.get(index)?;
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if matches!(
        tokens.get(index + 1).map(|token| &token.kind),
        Some(TokenKind::DoubleColon)
    ) {
        let second = tokens.get(index + 2)?;
        let TokenKind::Identifier(target) = &second.kind else {
            return None;
        };
        Some((
            NameOccurrence {
                alias: Some(name.clone()),
                name: target.clone(),
                span: second.span,
            },
            index + 3,
        ))
    } else {
        Some((
            NameOccurrence {
                alias: None,
                name: name.clone(),
                span: first.span,
            },
            index + 1,
        ))
    }
}

/// Resolves one name using only the local module or one explicit import alias.
pub(super) fn resolve_name(
    occurrence: &NameOccurrence,
    owner: &(String, String),
    module: &ModuleContext,
    roots: &BTreeMap<(String, String), Root>,
    errors: &mut Vec<ProjectSemanticDiagnostic>,
) -> Option<(String, String)> {
    let target_module = match &occurrence.alias {
        Some(alias) => module.aliases.get(alias).cloned(),
        None => Some(owner.0.clone()),
    };
    let target = target_module.map(|module| (module, occurrence.name.clone()));
    match target {
        Some(ref key)
            if roots
                .get(key)
                .is_some_and(|root| occurrence.alias.is_none() || root.symbol.is_public) =>
        {
            Some(key.clone())
        }
        _ => {
            errors.push(diagnostic(
                diagnostics::INACCESSIBLE_NAME,
                &owner.0,
                module.digest,
                occurrence.span,
            ));
            None
        }
    }
}

/// Constructs an identity edge with separate source provenance.
pub(super) fn dependency(
    from: &Root,
    to: &Root,
    kind: ProjectDependencyKind,
    digest: SourceContentDigest,
    span: ByteSpan,
) -> ProjectDependency {
    ProjectDependency {
        from: from.symbol.identity.clone(),
        to: to.symbol.identity.clone(),
        kind,
        source: SourceLocation::new(digest, span),
    }
}

/// Returns the least root in an ordinary value or embedded-type dependency cycle.
pub(super) fn cycle_root(
    roots: &BTreeMap<(String, String), Root>,
    edges: &[ProjectDependency],
) -> Option<(String, String)> {
    let mut remaining = roots.keys().cloned().collect::<BTreeSet<_>>();
    let mut dependencies = roots
        .keys()
        .cloned()
        .map(|key| (key, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for edge in edges {
        if matches!(
            edge.kind,
            ProjectDependencyKind::Type | ProjectDependencyKind::Value
        ) {
            dependencies
                .get_mut(&key(edge.from()))
                .expect("edge owner is collected")
                .insert(key(edge.to()));
        }
    }
    loop {
        let ready = remaining
            .iter()
            .filter(|node| dependencies[*node].is_disjoint(&remaining))
            .cloned()
            .collect::<Vec<_>>();
        if ready.is_empty() {
            break;
        }
        for node in ready {
            remaining.remove(&node);
        }
    }
    remaining.into_iter().next()
}
