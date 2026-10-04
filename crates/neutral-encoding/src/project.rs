// SPDX-License-Identifier: Apache-2.0

//! Closed, bounded project transport, separate from legacy document framing.
//!
//! Positional tuples reject missing/extra fields. Export signatures and public
//! edges are derived from complete declarations/provenance, never duplicated.
//! The declared interface fingerprint is still independently checked. Transport
//! bytes and source companions are not a project semantic fingerprint.

use crate::constants::kind;
use crate::{
    DecodeError, DecodeErrorClass, DecodeLimits, EncodingError,
    cbor::CborWriter,
    decoder::{CborValue, LocatedValue, parse_section},
};
use ProjectPublicType as T;
use ProjectValue as V;
use neutral_core::{
    ByteSpan, CancellationToken, SemanticDigest, SourceContentDigest, SourceLocation,
    VocabularyContentDigest,
};
use neutral_ir::language as tag;
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    project::{
        PROJECT_IR_SCHEMA, PROJECT_MAX_DEPTH, ProjectDeclaration, ProjectIr, ProjectLimits,
        ProjectModule, ProjectProvenance, ProjectResourceFacts, ProjectSource, ProjectSourceMap,
        ProjectValue, ProjectVocabularyRecord, ProjectVocabularySource,
    },
    project_interface::{
        ProjectInterface, ProjectPublicEdgeKind, ProjectPublicField, ProjectPublicSignature,
        ProjectPublicType, ProjectPublicVocabulary,
    },
};
use neutral_reader::{ProjectReadError, ValidatedProject};
use std::sync::Arc;

/// Closed transport-only nominal discriminator.
const NOMINAL_TAG: &str = "nominal";
/// Closed transport-only vocabulary nominal discriminator.
const VOCABULARY_TAG: &str = "vocabulary";

/// Versioned project transport identification, independent of package versions.
pub const ENCODING: &str = "NIR-PROJECT-CBOR/1";
/// Distinct project frame prefix; legacy document decoders cannot misinterpret it.
pub const MAGIC: [u8; 8] = *b"NEUPR\r\n\x1a";

/// Returns transport-derived hard inspection bounds; producer limits still apply.
#[must_use]
pub const fn hard_project_limits() -> ProjectLimits {
    ProjectLimits {
        modules: crate::constants::MAXIMUM_CONTAINER_ITEMS as u64,
        declarations: crate::constants::MAXIMUM_CONTAINER_ITEMS as u64,
        import_edges: crate::constants::MAXIMUM_TRAVERSAL_NODES as u64,
        nodes: crate::constants::MAXIMUM_TRAVERSAL_NODES as u64,
        text_bytes: crate::constants::MAXIMUM_ARTIFACT_BYTES as u64,
        artifact_bytes: crate::constants::MAXIMUM_ARTIFACT_BYTES as u64,
    }
}

/// Encodes complete validated data, never a root-pruned project.
///
/// # Errors
/// Returns a bounded size/depth failure or cancellation without partial bytes.
pub fn encode_project(
    project: &ValidatedProject,
    cancellation: &CancellationToken,
) -> Result<Vec<u8>, EncodingError> {
    if cancellation.is_cancelled() {
        return Err(EncodingError::Cancelled);
    }
    let payload = write_ir(project.complete_ir(), Some(cancellation))?;
    // Enforce the same aggregate lexical/depth ceilings as an independent reader.
    parse_section(&payload, MAGIC.len(), DecodeLimits::hard(), cancellation).map_err(|e| {
        if e.class() == DecodeErrorClass::Cancelled {
            EncodingError::Cancelled
        } else {
            EncodingError::EncodedSizeLimit
        }
    })?;
    let mut bytes = MAGIC.to_vec();
    bytes.extend(payload);
    if bytes.len() > crate::constants::MAXIMUM_ARTIFACT_BYTES
        || bytes.len() as u64 > project.complete_ir().limits.artifact_bytes
    {
        return Err(EncodingError::EncodedSizeLimit);
    }
    if cancellation.is_cancelled() {
        return Err(EncodingError::Cancelled);
    }
    Ok(bytes)
}

/// Decodes and independently validates a complete project under consumer bounds.
///
/// # Errors
/// Rejects unknown/extra fields, malformed values, companions, limits, or cancellation.
/// No partial project or view is published on failure.
pub fn decode_project(
    bytes: &[u8],
    wire_limits: DecodeLimits,
    project_limits: ProjectLimits,
    cancellation: &CancellationToken,
) -> Result<ValidatedProject, DecodeError> {
    if cancellation.is_cancelled() {
        return Err(failure(DecodeErrorClass::Cancelled));
    }
    if bytes.len() > wire_limits.maximum_artifact_bytes()
        || bytes.len() as u64 > project_limits.artifact_bytes
    {
        return Err(failure(DecodeErrorClass::EncodedSizeLimit));
    }
    let payload = bytes
        .strip_prefix(&MAGIC)
        .ok_or_else(|| failure(DecodeErrorClass::MalformedFrame))?;
    if payload.len() > wire_limits.maximum_section_bytes() {
        return Err(failure(DecodeErrorClass::EncodedSizeLimit));
    }
    let parsed = parse_section(payload, MAGIC.len(), wire_limits, cancellation)?;
    let mut ir = read_ir(&parsed)?;
    let limits = ir.limits;
    if bytes.len() as u64 > limits.artifact_bytes {
        return Err(failure(DecodeErrorClass::EncodedSizeLimit));
    }
    let fingerprint = ir.public_interface.fingerprint();
    // Bound before constructing derived indices or cloning untrusted signatures.
    if !ir.within_schema_limits(limits.intersect(project_limits)) {
        return Err(failure(DecodeErrorClass::EncodedSizeLimit));
    }
    let computed = ir
        .recompute_public_interface()
        .map_err(|_| failure(DecodeErrorClass::InvalidLogicalIr))?;
    ir.public_interface = ProjectInterface::from_parts_with_vocabularies(
        computed.vocabularies().to_vec(),
        computed.exports().to_vec(),
        computed.edges().to_vec(),
        fingerprint,
    );
    ValidatedProject::from_ir(Arc::new(ir), project_limits, cancellation).map_err(reader_error)
}

/// Writes a definite array with one shared item encoder.
fn write_list<T>(
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
fn write_numbers(w: &mut CborWriter, values: &[u64]) -> Result<(), EncodingError> {
    write_list(w, values, |w, n| w.unsigned(*n))
}
/// Writes an exact module identity.
fn write_module(w: &mut CborWriter, m: &LogicalModuleIdentity) -> Result<(), EncodingError> {
    w.array(2)?;
    w.text(m.language_behavior_version())?;
    w.text(m.module_name())
}
/// Writes a full module-symbol identity.
fn write_symbol(w: &mut CborWriter, s: &ModuleSymbolIdentity) -> Result<(), EncodingError> {
    w.array(2)?;
    write_module(w, s.module())?;
    w.text(s.declaration_name())
}
/// Writes original-byte location evidence.
fn write_location(w: &mut CborWriter, l: SourceLocation) -> Result<(), EncodingError> {
    w.array(3)?;
    w.bytes(&l.source().as_bytes())?;
    w.unsigned(l.span().start())?;
    w.unsigned(l.span().end())
}
/// Writes a binding or record signature with a closed discriminant.
fn write_signature(w: &mut CborWriter, s: &ProjectPublicSignature) -> Result<(), EncodingError> {
    w.array(2)?;
    match s {
        ProjectPublicSignature::Binding(t) => {
            w.boolean(false)?;
            write_type(w, t, 0)
        }
        ProjectPublicSignature::Record(fields) => {
            w.boolean(true)?;
            write_list(w, fields, |w, f| {
                w.array(2)?;
                w.text(f.name())?;
                write_type(w, f.ty(), 0)
            })
        }
    }
}
/// Writes one bounded project type; tags are the shared language spellings.
fn write_type(
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
/// Writes materialized values, retaining exact numbers and inert location kinds.
fn write_value(w: &mut CborWriter, v: &ProjectValue, depth: usize) -> Result<(), EncodingError> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(EncodingError::EncodedSizeLimit);
    }
    match v {
        V::Null => {
            w.array(1)?;
            w.text(tag::NULL)
        }
        V::Number(n) => {
            w.array(4)?;
            w.text(tag::NUM)?;
            w.boolean(n.is_negative())?;
            w.text(n.coefficient())?;
            w.signed(n.scale())
        }
        V::String(s) | V::Url(s) | V::Path(s) => {
            w.array(2)?;
            w.text(match v {
                V::String(_) => tag::STRING,
                V::Url(_) => tag::URL,
                _ => tag::PATH,
            })?;
            w.text(s)
        }
        V::Bool(b) => {
            w.array(2)?;
            w.text(tag::BOOL)?;
            w.boolean(*b)
        }
        V::Reference(s) => {
            w.array(2)?;
            w.text(tag::REF_TYPE)?;
            write_symbol(w, s)
        }
        V::List(values) => {
            w.array(2)?;
            w.text(tag::LIST)?;
            write_list(w, values, |w, v| write_value(w, v, depth + 1))
        }
        V::Record(fields) => {
            w.array(2)?;
            w.text(tag::RECORD)?;
            write_fields(w, fields, depth + 1)
        }
    }
}
/// Writes canonical fields without permitting a duplicate-key map projection.
fn write_fields(
    w: &mut CborWriter,
    fields: &[(String, ProjectValue)],
    depth: usize,
) -> Result<(), EncodingError> {
    write_list(w, fields, |w, (n, v)| {
        w.array(2)?;
        w.text(n)?;
        write_value(w, v, depth)
    })
}
/// Reads a definite array already bounded by the lexical parser.
fn array(v: &LocatedValue) -> Result<&[LocatedValue], DecodeError> {
    if let CborValue::Array(a) = &v.value {
        Ok(a)
    } else {
        Err(schema(v))
    }
}
/// Checks exact tuple cardinality before any positional access.
fn tuple<const N: usize>(v: &LocatedValue) -> Result<&[LocatedValue; N], DecodeError> {
    array(v)?.try_into().map_err(|_| schema(v))
}
/// Reads a collection without trusting unvalidated container lengths.
fn read_list<T>(
    v: &LocatedValue,
    item: impl FnMut(&LocatedValue) -> Result<T, DecodeError>,
) -> Result<Vec<T>, DecodeError> {
    array(v)?.iter().map(item).collect()
}
/// Borrows UTF-8 text from one lexically validated value.
fn text(v: &LocatedValue) -> Result<&str, DecodeError> {
    if let CborValue::Text(s) = &v.value {
        Ok(s)
    } else {
        Err(schema(v))
    }
}
/// Copies one bounded schema string.
fn owned_text(v: &LocatedValue) -> Result<String, DecodeError> {
    Ok(text(v)?.to_owned())
}
/// Reads an unsigned integer without coercion.
fn unsigned(v: &LocatedValue) -> Result<u64, DecodeError> {
    if let CborValue::Unsigned(n) = v.value {
        Ok(n)
    } else {
        Err(schema(v))
    }
}
/// Reads a signed integer with checked positive conversion.
fn signed(v: &LocatedValue) -> Result<i64, DecodeError> {
    match v.value {
        CborValue::Negative(n) => Ok(n),
        CborValue::Unsigned(n) => i64::try_from(n).map_err(|_| schema(v)),
        _ => Err(schema(v)),
    }
}
/// Reads an explicit Boolean without numeric coercion.
fn boolean(v: &LocatedValue) -> Result<bool, DecodeError> {
    if let CborValue::Boolean(b) = v.value {
        Ok(b)
    } else {
        Err(schema(v))
    }
}
/// Reads an exact typed digest payload, never a lossy text identity.
fn digest(v: &LocatedValue) -> Result<[u8; 32], DecodeError> {
    if let CborValue::Bytes(b) = &v.value {
        b.as_slice().try_into().map_err(|_| schema(v))
    } else {
        Err(schema(v))
    }
}
/// Reads one exact module identity; the independent reader checks grammar/profile.
fn read_module(v: &LocatedValue) -> Result<LogicalModuleIdentity, DecodeError> {
    let a = tuple::<2>(v)?;
    Ok(LogicalModuleIdentity::new(text(&a[0])?, text(&a[1])?))
}
/// Reads one exact declaration identity.
fn read_symbol(v: &LocatedValue) -> Result<ModuleSymbolIdentity, DecodeError> {
    let a = tuple::<2>(v)?;
    Ok(ModuleSymbolIdentity::new(read_module(&a[0])?, text(&a[1])?))
}
/// Reads an ordered original-byte span; ownership is independently validated.
fn read_location(v: &LocatedValue) -> Result<SourceLocation, DecodeError> {
    let a = tuple::<3>(v)?;
    Ok(SourceLocation::new(
        SourceContentDigest::from_raw_bytes(digest(&a[0])?),
        ByteSpan::new(unsigned(&a[1])?, unsigned(&a[2])?).map_err(|_| schema(v))?,
    ))
}
/// Reads a signature with no unknown or ignored members.
fn read_signature(v: &LocatedValue) -> Result<ProjectPublicSignature, DecodeError> {
    let a = tuple::<2>(v)?;
    Ok(if boolean(&a[0])? {
        ProjectPublicSignature::Record(read_list(&a[1], |v| {
            let a = tuple::<2>(v)?;
            Ok(ProjectPublicField::new(text(&a[0])?, read_type(&a[1], 0)?))
        })?)
    } else {
        ProjectPublicSignature::Binding(read_type(&a[1], 0)?)
    })
}
/// Reads bounded closed type tuples, rejecting every unknown discriminator.
fn read_type(v: &LocatedValue, depth: usize) -> Result<ProjectPublicType, DecodeError> {
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
            let inner = Box::new(read_type(&a[1], depth + 1)?);
            match tag {
                tag::LIST => T::List(inner),
                tag::REF_TYPE => T::Ref(inner),
                _ => T::Nullable(inner),
            }
        }
        _ => return Err(schema(v)),
    })
}
/// Reads exact materialized value tuples; validation still checks contextual types.
fn read_value(v: &LocatedValue, depth: usize) -> Result<ProjectValue, DecodeError> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(failure(DecodeErrorClass::EncodedSizeLimit));
    }
    let a = array(v)?;
    let tag = a.first().ok_or_else(|| schema(v)).and_then(text)?;
    Ok(match (tag, a.len()) {
        (tag::NULL, 1) => V::Null,
        (tag::NUM, 4) => V::Number(
            ExactNumber::from_normalized_parts(
                boolean(&a[1])?,
                text(&a[2])?,
                signed(&a[3])?,
                crate::constants::MAXIMUM_EXACT_NUMBER_DIGITS as u64,
                crate::constants::MAXIMUM_TEXT_BYTES as u64,
            )
            .map_err(|_| schema(v))?,
        ),
        (tag::STRING, 2) => V::String(owned_text(&a[1])?),
        (tag::URL, 2) => V::Url(owned_text(&a[1])?),
        (tag::PATH, 2) => V::Path(owned_text(&a[1])?),
        (tag::BOOL, 2) => V::Bool(boolean(&a[1])?),
        (tag::REF_TYPE, 2) => V::Reference(read_symbol(&a[1])?),
        (tag::LIST, 2) => V::List(read_list(&a[1], |v| read_value(v, depth + 1))?),
        (tag::RECORD, 2) => V::Record(read_fields(&a[1], depth + 1)?),
        _ => return Err(schema(v)),
    })
}
/// Reads field occurrences without silently discarding duplicates.
fn read_fields(v: &LocatedValue, depth: usize) -> Result<Vec<(String, ProjectValue)>, DecodeError> {
    read_list(v, |v| {
        let a = tuple::<2>(v)?;
        Ok((owned_text(&a[0])?, read_value(&a[1], depth)?))
    })
}
/// Maps the closed edge kind to its stable wire ordinal.
const fn edge_tag(k: ProjectPublicEdgeKind) -> u64 {
    match k {
        ProjectPublicEdgeKind::Type => 0,
        ProjectPublicEdgeKind::ReferenceType => 1,
        ProjectPublicEdgeKind::Value => 2,
        ProjectPublicEdgeKind::Reference => 3,
    }
}
/// Rejects unknown edge ordinals instead of interpreting them as value dependencies.
fn read_edge(n: u64) -> Result<ProjectPublicEdgeKind, DecodeError> {
    match n {
        0 => Ok(ProjectPublicEdgeKind::Type),
        1 => Ok(ProjectPublicEdgeKind::ReferenceType),
        2 => Ok(ProjectPublicEdgeKind::Value),
        3 => Ok(ProjectPublicEdgeKind::Reference),
        _ => Err(failure(DecodeErrorClass::InvalidEncodedSchema)),
    }
}
/// Produces a safe bounded failure with no hostile input text.
const fn failure(class: DecodeErrorClass) -> DecodeError {
    DecodeError::new(class, None)
}
/// Associates closed-schema rejection with its lexical byte offset.
const fn schema(v: &LocatedValue) -> DecodeError {
    DecodeError::new(DecodeErrorClass::InvalidEncodedSchema, Some(v.offset))
}
/// Preserves cancellation/limit/companion distinctions across the transport boundary.
fn reader_error(e: ProjectReadError) -> DecodeError {
    failure(match e {
        ProjectReadError::Cancelled => DecodeErrorClass::Cancelled,
        ProjectReadError::Limit => DecodeErrorClass::EncodedSizeLimit,
        ProjectReadError::Schema => DecodeErrorClass::UnsupportedVersion,
        ProjectReadError::Companion => DecodeErrorClass::InvalidSourceMap,
        ProjectReadError::Resources => DecodeErrorClass::InvalidDerivation,
        _ => DecodeErrorClass::InvalidLogicalIr,
    })
}

/// Writes non-semantic evidence and resource bounds in the frozen tuple order.
fn write_companions(w: &mut CborWriter, ir: &ProjectIr) -> Result<(), EncodingError> {
    write_list(w, &ir.sources, |w, s| {
        w.array(4)?;
        w.text(&s.module)?;
        w.text(&s.source_id)?;
        w.bytes(&s.digest.as_bytes())?;
        w.unsigned(s.byte_len)
    })?;
    write_list(w, &ir.source_maps, |w, m| {
        w.array(2)?;
        write_symbol(w, &m.declaration)?;
        write_location(w, m.location)
    })?;
    write_list(w, &ir.provenance, |w, p| {
        w.array(4)?;
        write_symbol(w, &p.from)?;
        write_symbol(w, &p.to)?;
        w.unsigned(edge_tag(p.kind))?;
        write_location(w, p.location)
    })?;
    write_numbers(
        w,
        &[
            ir.limits.modules,
            ir.limits.declarations,
            ir.limits.import_edges,
            ir.limits.nodes,
            ir.limits.text_bytes,
            ir.limits.artifact_bytes,
        ],
    )?;
    let r = ir.resources;
    write_numbers(
        w,
        &[
            r.source_units,
            r.source_bytes,
            r.vocabulary_units,
            r.vocabulary_bytes,
            r.declarations,
            r.import_edges,
            r.value_nodes,
        ],
    )?;
    write_list(w, &ir.vocabulary_sources, |w, s| {
        w.array(4)?;
        w.text(&s.identity)?;
        w.text(&s.version)?;
        w.bytes(&s.digest.as_bytes())?;
        w.unsigned(s.byte_len)
    })?;
    Ok(())
}

/// Reads one closed declaration tuple before independent validation.
fn read_declaration(v: &LocatedValue) -> Result<ProjectDeclaration, DecodeError> {
    let a = tuple::<5>(v)?;
    Ok(ProjectDeclaration {
        identity: read_symbol(&a[0])?,
        public: boolean(&a[1])?,
        signature: read_signature(&a[2])?,
        value: if matches!(a[3].value, CborValue::Null) {
            None
        } else {
            Some(read_value(&a[3], 0)?)
        },
        defaults: read_fields(&a[4], 0)?,
    })
}

/// Reads one closed vocabulary record tuple before independent validation.
fn read_vocabulary_record(v: &LocatedValue) -> Result<ProjectVocabularyRecord, DecodeError> {
    let a = tuple::<5>(v)?;
    Ok(ProjectVocabularyRecord {
        identity: owned_text(&a[0])?,
        version: owned_text(&a[1])?,
        name: owned_text(&a[2])?,
        public: boolean(&a[3])?,
        fields: read_list(&a[4], |v| {
            let a = tuple::<2>(v)?;
            Ok((owned_text(&a[0])?, read_type(&a[1], 0)?))
        })?,
    })
}

/// Reconstructs the closed project tuple without publishing a trusted reader.
fn read_ir(parsed: &LocatedValue) -> Result<ProjectIr, DecodeError> {
    let p = tuple::<12>(parsed)?;
    if text(&p[0])? != PROJECT_IR_SCHEMA {
        return Err(failure(DecodeErrorClass::UnsupportedVersion));
    }
    let modules = read_list(&p[1], |v| {
        let a = tuple::<2>(v)?;
        Ok(ProjectModule {
            identity: read_module(&a[0])?,
            imports: read_list(&a[1], owned_text)?,
        })
    })?;
    let declarations = read_list(&p[2], read_declaration)?;

    let vocabulary_records = read_list(&p[3], read_vocabulary_record)?;

    let vocabularies = read_list(&p[4], |v| {
        let a = tuple::<3>(v)?;
        Ok(ProjectPublicVocabulary::new(
            owned_text(&a[0])?,
            owned_text(&a[1])?,
            read_list(&a[2], owned_text)?,
        ))
    })?;
    let fingerprint = SemanticDigest::from_raw_bytes(digest(&p[5])?);
    let sources = read_list(&p[6], |v| {
        let a = tuple::<4>(v)?;
        Ok(ProjectSource {
            module: owned_text(&a[0])?,
            source_id: owned_text(&a[1])?,
            digest: SourceContentDigest::from_raw_bytes(digest(&a[2])?),
            byte_len: unsigned(&a[3])?,
        })
    })?;
    let source_maps = read_list(&p[7], |v| {
        let a = tuple::<2>(v)?;
        Ok(ProjectSourceMap {
            declaration: read_symbol(&a[0])?,
            location: read_location(&a[1])?,
        })
    })?;
    let provenance = read_list(&p[8], |v| {
        let a = tuple::<4>(v)?;
        Ok(ProjectProvenance {
            from: read_symbol(&a[0])?,
            to: read_symbol(&a[1])?,
            kind: read_edge(unsigned(&a[2])?)?,
            location: read_location(&a[3])?,
        })
    })?;
    let l = tuple::<6>(&p[9])?;
    let limits = ProjectLimits {
        modules: unsigned(&l[0])?,
        declarations: unsigned(&l[1])?,
        import_edges: unsigned(&l[2])?,
        nodes: unsigned(&l[3])?,
        text_bytes: unsigned(&l[4])?,
        artifact_bytes: unsigned(&l[5])?,
    };
    let r = tuple::<7>(&p[10])?;
    let resources = ProjectResourceFacts {
        source_units: unsigned(&r[0])?,
        source_bytes: unsigned(&r[1])?,
        vocabulary_units: unsigned(&r[2])?,
        vocabulary_bytes: unsigned(&r[3])?,
        declarations: unsigned(&r[4])?,
        import_edges: unsigned(&r[5])?,
        value_nodes: unsigned(&r[6])?,
    };
    let vocabulary_sources = read_list(&p[11], |v| {
        let a = tuple::<4>(v)?;
        Ok(ProjectVocabularySource {
            identity: owned_text(&a[0])?,
            version: owned_text(&a[1])?,
            digest: VocabularyContentDigest::from_raw_bytes(digest(&a[2])?),
            byte_len: unsigned(&a[3])?,
        })
    })?;
    let ir = ProjectIr {
        schema: PROJECT_IR_SCHEMA.to_owned(),
        modules,
        declarations,
        vocabulary_records,
        public_interface: ProjectInterface::from_parts_with_vocabularies(
            vocabularies,
            Vec::new(),
            Vec::new(),
            fingerprint,
        ),
        sources,
        source_maps,
        provenance,
        limits,
        resources,
        vocabulary_sources,
    };
    Ok(ir)
}

/// Projects complete tuples; the public encoder only calls this with validated IR.
fn write_ir(
    ir: &ProjectIr,
    cancellation: Option<&CancellationToken>,
) -> Result<Vec<u8>, EncodingError> {
    let mut w = cancellation.map_or_else(CborWriter::new, CborWriter::with_cancellation);
    w.array(12)?;
    w.text(&ir.schema)?;
    write_list(&mut w, &ir.modules, |w, m| {
        w.array(2)?;
        write_module(w, &m.identity)?;
        write_list(w, &m.imports, |w, s| w.text(s))
    })?;
    write_list(&mut w, &ir.declarations, |w, d| {
        w.array(5)?;
        write_symbol(w, &d.identity)?;
        w.boolean(d.public)?;
        write_signature(w, &d.signature)?;
        if let Some(v) = &d.value {
            write_value(w, v, 0)?;
        } else {
            w.null()?;
        }
        write_fields(w, &d.defaults, 0)
    })?;
    write_list(&mut w, &ir.vocabulary_records, |w, r| {
        w.array(5)?;
        w.text(&r.identity)?;
        w.text(&r.version)?;
        w.text(&r.name)?;
        w.boolean(r.public)?;
        write_list(w, &r.fields, |w, (n, t)| {
            w.array(2)?;
            w.text(n)?;
            write_type(w, t, 0)
        })
    })?;
    write_list(&mut w, ir.public_interface.vocabularies(), |w, v| {
        w.array(3)?;
        w.text(v.identity())?;
        w.text(v.version())?;
        write_list(w, v.public_types(), |w, s| w.text(s))
    })?;
    w.bytes(&ir.public_interface.fingerprint().as_bytes())?;
    write_companions(&mut w, ir)?;
    w.finish()
}

#[cfg(test)]
#[path = "../tests/project_codec/mod.rs"]
mod tests;
