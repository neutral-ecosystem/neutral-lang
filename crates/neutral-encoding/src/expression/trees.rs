// SPDX-License-Identifier: Apache-2.0

//! Exact typed-tree tuples; semantic annotation and binder authority remain reader-owned.

use super::{check_cancelled, read_list};
use crate::{
    DecodeError as D, DecodeErrorClass as C, EncodingError as E,
    cbor::CborWriter,
    contracts::{read_value, write_value},
    decoder::{CborValue, LocatedValue},
    primitives::{
        array, failure, owned_text, read_symbol, read_type, schema, text, tuple, unsigned,
        write_list, write_symbol, write_type,
    },
};
use neutral_core::{CancellationToken, allocation::boxed};
use neutral_ir::expression::{
    BinaryOperator as B, Expression, ExpressionNode as N, MatchArm, SelectorSegment as S,
    UnaryOperator as U,
    profile::{MAX_DEPTH, MAX_SELECTOR_SEGMENTS},
};

/// Writes every residual node in canonical evaluation order with a checked depth.
pub(super) fn write_tree(w: &mut CborWriter, e: &Expression, depth: usize) -> Result<(), E> {
    if depth > MAX_DEPTH {
        return Err(E::EncodedSizeLimit);
    }
    w.array(match &e.node {
        N::Unary { .. } | N::Match { .. } | N::Variant { .. } | N::Bound { .. } => 4,
        N::Output { .. } | N::Binary { .. } | N::If { .. } => 5,
        _ => 3,
    })?;
    w.text(e.node.kind())?;
    write_type(w, &e.ty, 0)?;
    match &e.node {
        N::Value(v) => write_value(w, v, 0),
        N::Input(s) => write_symbol(w, s),
        N::Output {
            target,
            result_type,
            selector,
        } => {
            if selector.len() > MAX_SELECTOR_SEGMENTS {
                return Err(E::EncodedSizeLimit);
            }
            write_symbol(w, target)?;
            write_type(w, result_type, 0)?;
            write_list(w, selector, |w, segment| {
                w.array(2)?;
                match segment {
                    S::Field(name) => {
                        w.text("field")?;
                        w.text(name)
                    }
                    S::Index(index) => {
                        w.text("index")?;
                        w.unsigned(u64::from(*index))
                    }
                    S::Alternative(tag) => {
                        w.text("alternative")?;
                        w.text(tag)
                    }
                }
            })
        }
        N::Unary { operator, operand } => {
            w.text(operator.spelling())?;
            write_tree(w, operand, depth + 1)
        }
        N::Binary {
            operator,
            left,
            right,
        } => {
            w.text(operator.spelling())?;
            write_tree(w, left, depth + 1)?;
            write_tree(w, right, depth + 1)
        }
        N::If {
            guard,
            then_branch,
            else_branch,
        } => {
            write_tree(w, guard, depth + 1)?;
            write_tree(w, then_branch, depth + 1)?;
            write_tree(w, else_branch, depth + 1)
        }
        N::Match { scrutinee, arms } => {
            write_tree(w, scrutinee, depth + 1)?;
            write_list(w, arms, |w, arm| {
                w.array(3)?;
                w.text(&arm.tag)?;
                write_type(w, &arm.payload_type, 0)?;
                write_tree(w, &arm.body, depth + 1)
            })
        }
        N::Text(operand) => write_tree(w, operand, depth + 1),
        N::Interpolate(items) | N::List(items) => {
            write_list(w, items, |w, item| write_tree(w, item, depth + 1))
        }
        N::Record(fields) => write_list(w, fields, |w, (name, child)| {
            w.array(2)?;
            w.text(name)?;
            if let Some(child) = child {
                write_tree(w, child, depth + 1)
            } else {
                w.array(1)?;
                w.text("omitted")
            }
        }),
        N::Variant { tag, payload } => {
            w.text(tag)?;
            write_tree(w, payload, depth + 1)
        }
        N::Bound { depth, slot } => {
            w.unsigned(*depth)?;
            w.unsigned(*slot)
        }
    }
}

/// Reads the closed grammar using fallible boxes; all semantic annotations remain untrusted.
pub(super) fn read_tree(
    v: &LocatedValue,
    depth: usize,
    cancel: &CancellationToken,
) -> Result<Expression, D> {
    check_cancelled(cancel)?;
    if depth > MAX_DEPTH {
        return Err(failure(C::EncodedSizeLimit));
    }
    let a = array(v)?;
    let tag = text(a.first().ok_or_else(|| schema(v))?)?;
    let ty = read_type(a.get(1).ok_or_else(|| schema(v))?, 0)?;
    let child = |v| {
        let expression = read_tree(v, depth + 1, cancel)?;
        check_cancelled(cancel)?;
        boxed(expression).map_err(|_| failure(C::EncodedSizeLimit))
    };
    let node = match (tag, a.len()) {
        ("value", 3) => N::Value(read_value(&a[2], 0)?),
        ("input", 3) => N::Input(read_symbol(&a[2])?),
        ("output", 5) => {
            if array(&a[4])?.len() > MAX_SELECTOR_SEGMENTS {
                return Err(failure(C::EncodedSizeLimit));
            }
            N::Output {
                target: read_symbol(&a[2])?,
                result_type: read_type(&a[3], 0)?,
                selector: read_list(&a[4], cancel, |v| {
                    let a = tuple::<2>(v)?;
                    match text(&a[0])? {
                        "field" => Ok(S::Field(owned_text(&a[1])?)),
                        "index" => Ok(S::Index(
                            u32::try_from(unsigned(&a[1])?).map_err(|_| schema(v))?,
                        )),
                        "alternative" => Ok(S::Alternative(owned_text(&a[1])?)),
                        _ => Err(schema(v)),
                    }
                })?,
            }
        }
        ("unary", 4) => N::Unary {
            operator: unary_operator(text(&a[2])?).ok_or_else(|| schema(v))?,
            operand: child(&a[3])?,
        },
        ("binary", 5) => N::Binary {
            operator: binary_operator(text(&a[2])?).ok_or_else(|| schema(v))?,
            left: child(&a[3])?,
            right: child(&a[4])?,
        },
        ("if", 5) => N::If {
            guard: child(&a[2])?,
            then_branch: child(&a[3])?,
            else_branch: child(&a[4])?,
        },
        ("match", 4) => N::Match {
            scrutinee: child(&a[2])?,
            arms: read_list(&a[3], cancel, |v| {
                let a = tuple::<3>(v)?;
                Ok(MatchArm {
                    tag: owned_text(&a[0])?,
                    payload_type: read_type(&a[1], 0)?,
                    body: read_tree(&a[2], depth + 1, cancel)?,
                })
            })?,
        },
        ("text", 3) => N::Text(child(&a[2])?),
        ("interpolate", 3) => N::Interpolate(read_list(&a[2], cancel, |v| {
            read_tree(v, depth + 1, cancel)
        })?),
        ("list", 3) => N::List(read_list(&a[2], cancel, |v| {
            read_tree(v, depth + 1, cancel)
        })?),
        ("record", 3) => N::Record(read_list(&a[2], cancel, |v| {
            let a = tuple::<2>(v)?;
            let omitted = matches!(&a[1].value, CborValue::Array(items)
                if items.len() == 1 && text(&items[0]).ok() == Some("omitted"));
            Ok((
                owned_text(&a[0])?,
                if omitted {
                    None
                } else {
                    Some(read_tree(&a[1], depth + 1, cancel)?)
                },
            ))
        })?),
        ("variant", 4) => N::Variant {
            tag: owned_text(&a[2])?,
            payload: child(&a[3])?,
        },
        ("bound", 4) => N::Bound {
            depth: unsigned(&a[2])?,
            slot: unsigned(&a[3])?,
        },
        _ => return Err(schema(v)),
    };
    check_cancelled(cancel)?;
    Ok(Expression { ty, node })
}

/// Recognizes only the two exact frozen unary tokens.
pub(super) fn unary_operator(spelling: &str) -> Option<U> {
    match spelling {
        "!" => Some(U::Not),
        "-" => Some(U::Negate),
        _ => None,
    }
}

/// Recognizes only the thirteen exact frozen binary tokens.
pub(super) fn binary_operator(spelling: &str) -> Option<B> {
    Some(match spelling {
        "*" => B::Multiply,
        "/" => B::Divide,
        "+" => B::Add,
        "-" => B::Subtract,
        "++" => B::Concat,
        "<" => B::Less,
        "<=" => B::LessEqual,
        ">" => B::Greater,
        ">=" => B::GreaterEqual,
        "==" => B::Equal,
        "!=" => B::NotEqual,
        "&&" => B::And,
        "||" => B::Or,
        _ => return None,
    })
}
