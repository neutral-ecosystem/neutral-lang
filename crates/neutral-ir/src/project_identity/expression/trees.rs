// SPDX-License-Identifier: Apache-2.0

//! Every frozen typed tree and template tuple, preserving semantic child/parameter order.

use crate::{
    expression::{
        Expression, ExpressionNode as N, MatchArm, SelectorSegment,
        profile::{MAX_DEPTH, MAX_SELECTOR_SEGMENTS},
        templates::{ExpansionLimits, ParameterPhase, PublicTemplate},
    },
    project_identity::{
        IdentityError as E,
        expression::contract::symbol,
        expression::contract::{type_transcript, value},
        framing::{Writer, ordered},
    },
};

/// Frames one retained tree; lexical scope and nominal typing are independently reader-validated.
pub(super) fn tree(w: &mut Writer<'_>, expression: &Expression, depth: usize) -> Result<(), E> {
    w.check()?;
    if depth > MAX_DEPTH {
        return Err(E::Limit);
    }
    w.frame("expression", |w| {
        w.leaf("kind", expression.node.kind().as_bytes())?;
        w.frame("type", |w| type_transcript(w, &expression.ty, 0))?;
        match &expression.node {
            N::Value(v) => {
                super::logical::value_keys(w, v, 0, &mut 0)?;
                w.frame("value", |w| value(w, v, 0))
            }
            N::Input(owner) => w.frame("symbol", |w| symbol(w, owner)),
            N::Output {
                target,
                result_type,
                selector: segments,
            } => {
                w.frame("target", |w| symbol(w, target))?;
                w.frame("result-type", |w| type_transcript(w, result_type, 0))?;
                selector(w, segments)
            }
            N::Unary { operator, operand } => {
                w.leaf("operator", operator.spelling().as_bytes())?;
                child(w, "operand", operand, depth)
            }
            N::Binary {
                operator,
                left,
                right,
            } => {
                w.leaf("operator", operator.spelling().as_bytes())?;
                child(w, "left", left, depth)?;
                child(w, "right", right, depth)
            }
            N::If {
                guard,
                then_branch,
                else_branch,
            } => {
                child(w, "guard", guard, depth)?;
                child(w, "then", then_branch, depth)?;
                child(w, "else", else_branch, depth)
            }
            N::Match { scrutinee, arms } => {
                child(w, "scrutinee", scrutinee, depth)?;
                match_arms(w, arms, depth)
            }
            N::Text(operand) => child(w, "operand", operand, depth),
            N::Interpolate(items) | N::List(items) => {
                w.items(items.len())?;
                w.frame("items", |w| {
                    for item in items {
                        child(w, "item", item, depth)?;
                    }
                    Ok(())
                })
            }
            N::Record(fields) => {
                w.items(fields.len())?;
                for (name, _) in fields {
                    w.text(name)?;
                    if name.is_empty() {
                        return Err(E::InvalidInput);
                    }
                }
                ordered(fields.iter().map(|(name, _)| name))?;
                w.frame("fields", |w| {
                    for (name, expression) in fields {
                        w.frame("field", |w| {
                            w.leaf("name", name.as_bytes())?;
                            w.frame("value", |w| match expression {
                                Some(expression) => tree(w, expression, depth + 1),
                                None => w.leaf("omitted", &[]),
                            })
                        })?;
                    }
                    Ok(())
                })
            }
            N::Variant { tag, payload } => {
                if tag.is_empty() {
                    return Err(E::InvalidInput);
                }
                w.leaf("tag", tag.as_bytes())?;
                child(w, "payload", payload, depth)
            }
            N::Bound { depth, slot } => {
                w.number("depth", *depth)?;
                w.number("slot", *slot)
            }
        }
    })
}

/// Preserves canonical arm tags and complete independently typed payload bodies.
fn match_arms(w: &mut Writer<'_>, arms: &[MatchArm], depth: usize) -> Result<(), E> {
    w.items(arms.len())?;
    if arms.is_empty() {
        return Err(E::InvalidInput);
    }
    for arm in arms {
        w.text(&arm.tag)?;
        if arm.tag.is_empty() {
            return Err(E::InvalidInput);
        }
    }
    ordered(arms.iter().map(|arm| &arm.tag))?;
    w.frame("arms", |w| {
        for arm in arms {
            w.frame("arm", |w| {
                w.leaf("tag", arm.tag.as_bytes())?;
                w.frame("payload-type", |w| type_transcript(w, &arm.payload_type, 0))?;
                child(w, "body", &arm.body, depth)
            })?;
        }
        Ok(())
    })
}

/// Frames the closed typed output selector without evaluating availability.
fn selector(w: &mut Writer<'_>, segments: &[SelectorSegment]) -> Result<(), E> {
    w.items(segments.len())?;
    if segments.len() > MAX_SELECTOR_SEGMENTS {
        return Err(E::Limit);
    }
    w.frame("selector", |w| {
        for segment in segments {
            w.frame("segment", |w| match segment {
                SelectorSegment::Field(name) | SelectorSegment::Alternative(name) => {
                    if name.is_empty() {
                        return Err(E::InvalidInput);
                    }
                    w.leaf(
                        "kind",
                        if matches!(segment, SelectorSegment::Field(_)) {
                            b"field"
                        } else {
                            b"alternative"
                        },
                    )?;
                    w.leaf("name", name.as_bytes())
                }
                SelectorSegment::Index(index) => {
                    w.leaf("kind", b"index")?;
                    w.number("index", u64::from(*index))
                }
            })?;
        }
        Ok(())
    })
}

/// Uses the exact named child wrapper around a complete expression tree.
fn child(w: &mut Writer<'_>, tag: &str, expression: &Expression, depth: usize) -> Result<(), E> {
    w.frame(tag, |w| tree(w, expression, depth + 1))
}

/// Public parameters remain ordered and names remain semantic; binder spelling is absent.
pub(super) fn templates(w: &mut Writer<'_>, templates: &[PublicTemplate]) -> Result<(), E> {
    w.frame("public-templates", |w| {
        for template in templates {
            w.frame("template", |w| {
                symbol(w, &template.identity)?;
                w.items(template.parameters.len())?;
                if template.parameters.len() as u64 > ExpansionLimits::hard().parameters {
                    return Err(E::Limit);
                }
                for (index, parameter) in template.parameters.iter().enumerate() {
                    w.text(&parameter.name)?;
                    if parameter.name.is_empty()
                        || template.parameters[..index]
                            .iter()
                            .any(|p| p.name == parameter.name)
                    {
                        return Err(E::InvalidInput);
                    }
                }
                w.frame("parameters", |w| {
                    for parameter in &template.parameters {
                        w.frame("parameter", |w| {
                            w.leaf("name", parameter.name.as_bytes())?;
                            w.leaf(
                                "phase",
                                match parameter.phase {
                                    ParameterPhase::Static => b"static",
                                    ParameterPhase::Deferred => b"deferred",
                                },
                            )?;
                            w.frame("type", |w| type_transcript(w, &parameter.ty, 0))?;
                            w.frame("default", |w| match &parameter.default {
                                Some(default) => tree(w, default, 1),
                                None => w.leaf("absent", &[]),
                            })
                        })?;
                    }
                    Ok(())
                })?;
                w.frame("result-type", |w| {
                    type_transcript(w, &template.result_type, 0)
                })?;
                w.frame("body", |w| tree(w, &template.body, 1))
            })?;
        }
        Ok(())
    })
}
