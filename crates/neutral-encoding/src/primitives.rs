// SPDX-License-Identifier: Apache-2.0

//! Bounded CBOR primitives shared by complete project transports.
//! Scalar/type/location grammar is independent of any historical whole-project codec.

use crate::{
    DecodeError, DecodeErrorClass, EncodingError,
    cbor::CborWriter,
    constants::kind,
    decoder::{CborValue, LocatedValue},
};
use ProjectPublicType as T;
use neutral_core::{ByteSpan, SourceContentDigest, SourceLocation, allocation::RetainCapacity};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity, language as tag,
    project::PROJECT_MAX_DEPTH,
    project_interface::{ProjectPublicEdgeKind, ProjectPublicType},
};

/// Closed transport-only nominal discriminator.
const NOMINAL_TAG: &str = "nominal";
/// Closed transport-only vocabulary nominal discriminator.
const VOCABULARY_TAG: &str = "vocabulary";

/// Writes a definite array with one shared item encoder.
pub(crate) fn write_list<T>(
    w: &mut CborWriter,
    values: &[T],
    mut item: impl FnMut(&mut CborWriter, &T) -> Result<(), EncodingError>,
) -> Result<(), EncodingError> {
    w.array(values.len())?;
    for value in values {
        item(w, value)?;
    }
    Ok(())
}

/// Writes a fixed unsigned tuple.
pub(crate) fn write_numbers(w: &mut CborWriter, values: &[u64]) -> Result<(), EncodingError> {
    write_list(w, values, |w, n| w.unsigned(*n))
}

/// Writes an exact module identity.
pub(crate) fn write_module(
    w: &mut CborWriter,
    m: &LogicalModuleIdentity,
) -> Result<(), EncodingError> {
    w.array(2)?;
    w.text(m.language_behavior_version())?;
    w.text(m.module_name())
}

/// Writes a full module-symbol identity.
pub(crate) fn write_symbol(
    w: &mut CborWriter,
    s: &ModuleSymbolIdentity,
) -> Result<(), EncodingError> {
    w.array(2)?;
    write_module(w, s.module())?;
    w.text(s.declaration_name())
}

/// Writes original-byte location evidence.
pub(crate) fn write_location(w: &mut CborWriter, l: SourceLocation) -> Result<(), EncodingError> {
    w.array(3)?;
    w.bytes(&l.source().as_bytes())?;
    w.unsigned(l.span().start())?;
    w.unsigned(l.span().end())
}

/// Writes one bounded project type; tags are the shared language spellings.
pub(crate) fn write_type(
    w: &mut CborWriter,
    t: &ProjectPublicType,
    depth: usize,
) -> Result<(), EncodingError> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(EncodingError::EncodedSizeLimit);
    }
    match t {
        T::Num | T::String | T::Bool | T::Url | T::Path => {
            w.array(1)?;
            w.text(match t {
                T::Num => tag::NUM,
                T::String => tag::STRING,
                T::Bool => tag::BOOL,
                T::Url => tag::URL,
                _ => tag::PATH,
            })
        }
        T::Nominal(s) => {
            w.array(2)?;
            w.text(NOMINAL_TAG)?;
            write_symbol(w, s)
        }
        T::VocabularyNominal {
            identity,
            version,
            name,
        } => {
            w.array(4)?;
            w.text(VOCABULARY_TAG)?;
            w.text(identity)?;
            w.text(version)?;
            w.text(name)
        }
        T::List(inner) | T::Ref(inner) | T::Nullable(inner) => {
            w.array(2)?;
            w.text(match t {
                T::List(_) => tag::LIST,
                T::Ref(_) => tag::REF_TYPE,
                _ => kind::NULLABLE,
            })?;
            write_type(w, inner, depth + 1)
        }
    }
}

/// Reads a definite array already bounded by the lexical parser.
pub(crate) fn array(v: &LocatedValue) -> Result<&[LocatedValue], DecodeError> {
    if let CborValue::Array(a) = &v.value {
        Ok(a)
    } else {
        Err(schema(v))
    }
}

/// Checks exact tuple cardinality before any positional access.
pub(crate) fn tuple<const N: usize>(v: &LocatedValue) -> Result<&[LocatedValue; N], DecodeError> {
    array(v)?.try_into().map_err(|_| schema(v))
}

/// Reads a collection without trusting unvalidated container lengths.
pub(crate) fn read_list<T>(
    v: &LocatedValue,
    mut item: impl FnMut(&LocatedValue) -> Result<T, DecodeError>,
) -> Result<Vec<T>, DecodeError> {
    let items = array(v)?;
    let mut result = Vec::new();
    result
        .try_retain_exact(items.len())
        .map_err(|_| failure(DecodeErrorClass::EncodedSizeLimit))?;
    for value in items {
        result.push(item(value)?);
    }
    Ok(result)
}

/// Borrows UTF-8 text from one lexically validated value.
pub(crate) fn text(v: &LocatedValue) -> Result<&str, DecodeError> {
    if let CborValue::Text(s) = &v.value {
        Ok(s)
    } else {
        Err(schema(v))
    }
}

/// Copies one bounded schema string.
pub(crate) fn owned_text(v: &LocatedValue) -> Result<String, DecodeError> {
    neutral_core::allocation::text(text(v)?)
        .map_err(|_| failure(DecodeErrorClass::EncodedSizeLimit))
}

/// Reads an unsigned integer without coercion.
pub(crate) fn unsigned(v: &LocatedValue) -> Result<u64, DecodeError> {
    if let CborValue::Unsigned(n) = v.value {
        Ok(n)
    } else {
        Err(schema(v))
    }
}

/// Reads a signed integer with checked positive conversion.
pub(crate) fn signed(v: &LocatedValue) -> Result<i64, DecodeError> {
    match v.value {
        CborValue::Negative(n) => Ok(n),
        CborValue::Unsigned(n) => i64::try_from(n).map_err(|_| schema(v)),
        _ => Err(schema(v)),
    }
}

/// Reads an explicit Boolean without numeric coercion.
pub(crate) fn boolean(v: &LocatedValue) -> Result<bool, DecodeError> {
    if let CborValue::Boolean(b) = v.value {
        Ok(b)
    } else {
        Err(schema(v))
    }
}

/// Reads an exact typed digest payload, never a lossy text identity.
pub(crate) fn digest(v: &LocatedValue) -> Result<[u8; 32], DecodeError> {
    if let CborValue::Bytes(b) = &v.value {
        b.as_slice().try_into().map_err(|_| schema(v))
    } else {
        Err(schema(v))
    }
}

/// Reads one exact module identity; the independent reader checks grammar/profile.
pub(crate) fn read_module(v: &LocatedValue) -> Result<LogicalModuleIdentity, DecodeError> {
    let a = tuple::<2>(v)?;
    Ok(LogicalModuleIdentity::new(
        owned_text(&a[0])?,
        owned_text(&a[1])?,
    ))
}

/// Reads one exact declaration identity.
pub(crate) fn read_symbol(v: &LocatedValue) -> Result<ModuleSymbolIdentity, DecodeError> {
    let a = tuple::<2>(v)?;
    Ok(ModuleSymbolIdentity::new(
        read_module(&a[0])?,
        owned_text(&a[1])?,
    ))
}

/// Reads an ordered original-byte span; ownership is independently validated.
pub(crate) fn read_location(v: &LocatedValue) -> Result<SourceLocation, DecodeError> {
    let a = tuple::<3>(v)?;
    Ok(SourceLocation::new(
        SourceContentDigest::from_raw_bytes(digest(&a[0])?),
        ByteSpan::new(unsigned(&a[1])?, unsigned(&a[2])?).map_err(|_| schema(v))?,
    ))
}

/// Reads bounded closed type tuples, rejecting every unknown discriminator.
pub(crate) fn read_type(v: &LocatedValue, depth: usize) -> Result<ProjectPublicType, DecodeError> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(failure(DecodeErrorClass::EncodedSizeLimit));
    }
    let a = array(v)?;
    let tag = a.first().ok_or_else(|| schema(v)).and_then(text)?;
    Ok(match (tag, a.len()) {
        (tag::NUM, 1) => T::Num,
        (tag::STRING, 1) => T::String,
        (tag::BOOL, 1) => T::Bool,
        (tag::URL, 1) => T::Url,
        (tag::PATH, 1) => T::Path,
        (NOMINAL_TAG, 2) => T::Nominal(read_symbol(&a[1])?),
        (VOCABULARY_TAG, 4) => T::VocabularyNominal {
            identity: owned_text(&a[1])?,
            version: owned_text(&a[2])?,
            name: owned_text(&a[3])?,
        },
        (tag::LIST | tag::REF_TYPE | kind::NULLABLE, 2) => {
            let inner = neutral_core::allocation::boxed(read_type(&a[1], depth + 1)?)
                .map_err(|_| failure(DecodeErrorClass::EncodedSizeLimit))?;
            match tag {
                tag::LIST => T::List(inner),
                tag::REF_TYPE => T::Ref(inner),
                _ => T::Nullable(inner),
            }
        }
        _ => return Err(schema(v)),
    })
}

/// Maps the closed edge kind to its stable wire ordinal.
pub(crate) const fn edge_tag(k: ProjectPublicEdgeKind) -> u64 {
    match k {
        ProjectPublicEdgeKind::Type => 0,
        ProjectPublicEdgeKind::ReferenceType => 1,
        ProjectPublicEdgeKind::Value => 2,
        ProjectPublicEdgeKind::Reference => 3,
    }
}

/// Rejects unknown edge ordinals instead of interpreting them as value dependencies.
pub(crate) fn read_edge(n: u64) -> Result<ProjectPublicEdgeKind, DecodeError> {
    match n {
        0 => Ok(ProjectPublicEdgeKind::Type),
        1 => Ok(ProjectPublicEdgeKind::ReferenceType),
        2 => Ok(ProjectPublicEdgeKind::Value),
        3 => Ok(ProjectPublicEdgeKind::Reference),
        _ => Err(failure(DecodeErrorClass::InvalidEncodedSchema)),
    }
}

/// Produces a safe bounded failure with no hostile input text.
pub(crate) const fn failure(class: DecodeErrorClass) -> DecodeError {
    DecodeError::new(class, None)
}

/// Associates closed-schema rejection with its lexical byte offset.
pub(crate) const fn schema(v: &LocatedValue) -> DecodeError {
    DecodeError::new(DecodeErrorClass::InvalidEncodedSchema, Some(v.offset))
}
