// SPDX-License-Identifier: Apache-2.0

//! Closed sixteen-position composition transport; decoding never invokes the compiler.

use crate::{
    DecodeError as D, DecodeErrorClass as C, DecodeLimits, EncodingError as E,
    cbor::CborWriter,
    decoder::{CborValue, LocatedValue, parse_canonical_section as parse_section},
    project::{
        array, boolean, digest, edge_tag, failure, owned_text, read_edge, read_list, read_location,
        read_module, read_symbol, read_type, schema, signed, text, tuple, unsigned, write_list,
        write_location, write_module, write_numbers, write_symbol, write_type,
    },
};
use neutral_core::allocation::Shared as Arc;
use neutral_core::{
    ByteSpan, CancellationToken, SemanticDigest, SourceContentDigest, VocabularyContentDigest,
};
use neutral_ir::{
    ExactNumber, ModuleSymbolIdentity, VocabularyIdentity,
    composition::{
        ClosedReference, CompositionAlternative, CompositionBody as B, CompositionBundle,
        CompositionDefinition, CompositionDependency, CompositionField, CompositionValue as V,
        FieldPresence as Presence, FieldRestrictions, ValueOriginKind as K, ValuePathSegment as P,
        profile,
        project::{
            CompositionAttribution as A, CompositionDeclaration, CompositionOrigin,
            CompositionPolicy, CompositionProjectIr as Ir, CompositionResourceFacts,
            CompositionSignature as S,
        },
    },
    project::{
        PROJECT_MAX_DEPTH, ProjectLimits, ProjectModule, ProjectProvenance, ProjectResourceFacts,
        ProjectSource, ProjectSourceMap, ProjectVocabularySource,
    },
};
use neutral_reader::composition::{CompositionReadError, ValidatedCompositionProject};
use neutral_vocabulary::composition::CompositionLimits;

/// Encodes independently validated complete data using the frozen successor tuple grammar.
///
/// # Errors
/// Returns bounds, allocation or cancellation failure without partial output.
pub fn encode_composition_project(
    project: &ValidatedCompositionProject,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, E> {
    let ir = project.complete_ir();
    let mut w = CborWriter::with_cancellation(cancel);
    write_ir(&mut w, ir)?;
    let payload = w.finish()?;
    parse_section(&payload, profile::MAGIC.len(), DecodeLimits::hard(), cancel).map_err(|e| {
        if e.class() == C::Cancelled {
            E::Cancelled
        } else {
            E::EncodedSizeLimit
        }
    })?;
    let mut output = profile::MAGIC.to_vec();
    output
        .try_reserve(payload.len())
        .map_err(|_| E::EncodedSizeLimit)?;
    output.extend(payload);
    if output.len() as u64 > ir.limits.artifact_bytes {
        return Err(E::EncodedSizeLimit);
    }
    if cancel.is_cancelled() {
        return Err(E::Cancelled);
    }
    Ok(output)
}

/// Decodes the explicit successor frame, then independently checks all semantic and companion facts.
///
/// # Errors
/// Rejects unknown/duplicate/noncanonical tags, hostile lengths, stale facts,
/// dangling/private targets or cancellation. No old-profile fallback occurs.
pub fn decode_composition_project(
    bytes: &[u8],
    wire: DecodeLimits,
    project: ProjectLimits,
    composition: CompositionLimits,
    cancel: &CancellationToken,
) -> Result<ValidatedCompositionProject, D> {
    if cancel.is_cancelled() {
        return Err(failure(C::Cancelled));
    }
    if bytes.len() > wire.maximum_artifact_bytes() || bytes.len() as u64 > project.artifact_bytes {
        return Err(failure(C::EncodedSizeLimit));
    }
    let payload = bytes
        .strip_prefix(&profile::MAGIC)
        .ok_or_else(|| failure(C::MalformedFrame))?;
    if payload.len() > wire.maximum_section_bytes() {
        return Err(failure(C::EncodedSizeLimit));
    }
    let parsed = parse_section(payload, profile::MAGIC.len(), wire, cancel)?;
    let ir = read_ir(&parsed)?;
    if bytes.len() as u64 > ir.limits.artifact_bytes {
        return Err(failure(C::EncodedSizeLimit));
    }
    ValidatedCompositionProject::from_ir(
        Arc::try_new(ir).map_err(|_| failure(C::EncodedSizeLimit))?,
        project,
        composition,
        cancel,
    )
    .map_err(|e| {
        failure(match e {
            CompositionReadError::Cancelled => C::Cancelled,
            CompositionReadError::Limit => C::EncodedSizeLimit,
            CompositionReadError::Schema => C::UnsupportedVersion,
            CompositionReadError::Companion => C::InvalidProvenance,
            _ => C::InvalidLogicalIr,
        })
    })
}

/// Shares the value grammar while keeping non-null default references unconstructible.
trait Reference: Sized {
    /// Writes a stable target symbol, never its value.
    fn write(&self, w: &mut CborWriter) -> Result<(), E>;
    /// Reads a target only where this value category permits references.
    fn read(v: &LocatedValue) -> Result<Self, D>;
}
impl Reference for ModuleSymbolIdentity {
    /// Retains exact logical owner identity.
    fn write(&self, w: &mut CborWriter) -> Result<(), E> {
        write_symbol(w, self)
    }
    /// Defers visibility/invariant typing to the independent reader.
    fn read(v: &LocatedValue) -> Result<Self, D> {
        read_symbol(v)
    }
}
impl Reference for ClosedReference {
    /// A closed default cannot reach this branch.
    fn write(&self, _: &mut CborWriter) -> Result<(), E> {
        match *self {}
    }
    /// Rejects non-null references before constructing a closed value.
    fn read(v: &LocatedValue) -> Result<Self, D> {
        Err(schema(v))
    }
}

/// Writes normalized complete value meaning, preserving omission inside record fields only.
fn write_value<R: Reference>(w: &mut CborWriter, v: &V<R>, depth: usize) -> Result<(), E> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(E::EncodedSizeLimit);
    }
    match v {
        V::Null => {
            w.array(1)?;
            w.text("null")
        }
        V::Number(n) => {
            w.array(4)?;
            w.text("num")?;
            w.boolean(n.is_negative())?;
            w.text(n.coefficient())?;
            w.signed(n.scale())
        }
        V::Bool(b) => {
            w.array(2)?;
            w.text("bool")?;
            w.boolean(*b)
        }
        V::String(s) | V::Url(s) | V::Path(s) => {
            w.array(2)?;
            w.text(match v {
                V::String(_) => "string",
                V::Url(_) => "url",
                _ => "path",
            })?;
            w.text(s)
        }
        V::Reference(r) => {
            w.array(2)?;
            w.text("Ref")?;
            r.write(w)
        }
        V::List(values) => {
            w.array(2)?;
            w.text("List")?;
            write_list(w, values, |w, v| write_value(w, v, depth + 1))
        }
        V::Record(fields) => {
            w.array(2)?;
            w.text("record")?;
            write_list(w, fields, |w, (n, v)| {
                w.array(2)?;
                w.text(n)?;
                if let Some(v) = v {
                    write_value(w, v, depth + 1)
                } else {
                    w.array(1)?;
                    w.text("omitted")
                }
            })
        }
        V::Variant { tag, payload } => {
            w.array(3)?;
            w.text("variant")?;
            w.text(tag)?;
            write_value(w, payload, depth + 1)
        }
    }
}

/// Reads a bounded value tuple, rejecting wrong arity and illegal absence states.
fn read_value<R: Reference>(v: &LocatedValue, depth: usize) -> Result<V<R>, D> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(failure(C::EncodedSizeLimit));
    }
    let a = array(v)?;
    let tag = text(a.first().ok_or_else(|| schema(v))?)?;
    Ok(match (tag, a.len()) {
        ("null", 1) => V::Null,
        ("num", 4) => V::Number(
            ExactNumber::from_normalized_parts(
                boolean(&a[1])?,
                text(&a[2])?,
                signed(&a[3])?,
                crate::constants::MAXIMUM_EXACT_NUMBER_DIGITS as u64,
                crate::constants::MAXIMUM_TEXT_BYTES as u64,
            )
            .map_err(|_| schema(v))?,
        ),
        ("bool", 2) => V::Bool(boolean(&a[1])?),
        ("string", 2) => V::String(owned_text(&a[1])?),
        ("url", 2) => V::Url(owned_text(&a[1])?),
        ("path", 2) => V::Path(owned_text(&a[1])?),
        ("Ref", 2) => V::Reference(R::read(&a[1])?),
        ("List", 2) => V::List(read_list(&a[1], |v| read_value(v, depth + 1))?),
        ("record", 2) => V::Record(read_list(&a[1], |v| {
            let f = tuple::<2>(v)?;
            let omitted = matches!(&f[1].value,CborValue::Array(a) if a.len()==1 && text(&a[0]).ok()==Some("omitted"));
            Ok((
                owned_text(&f[0])?,
                if omitted {
                    None
                } else {
                    Some(read_value(&f[1], depth + 1)?)
                },
            ))
        })?),
        ("variant", 3) => V::Variant {
            tag: owned_text(&a[1])?,
            payload: neutral_core::allocation::boxed(read_value(&a[2], depth + 1)?)
                .map_err(|_| failure(C::EncodedSizeLimit))?,
        },
        _ => return Err(schema(v)),
    })
}

/// Writes an explicit nullable transport slot, not a semantic null value.
fn write_option<T>(
    w: &mut CborWriter,
    v: Option<&T>,
    item: impl FnOnce(&mut CborWriter, &T) -> Result<(), E>,
) -> Result<(), E> {
    match v {
        Some(v) => item(w, v),
        None => w.null(),
    }
}
/// Reads an explicit nullable transport slot without coercion.
fn read_option<T>(
    v: &LocatedValue,
    item: impl FnOnce(&LocatedValue) -> Result<T, D>,
) -> Result<Option<T>, D> {
    if matches!(v.value, CborValue::Null) {
        Ok(None)
    } else {
        item(v).map(Some)
    }
}
/// Writes the closed field restriction tuple in frozen order.
fn write_restrictions(w: &mut CborWriter, r: &FieldRestrictions) -> Result<(), E> {
    w.array(5)?;
    write_list(w, r.choices.as_deref().unwrap_or(&[]), |w, v| {
        write_value(w, v, 0)
    })?;
    for n in [&r.minimum, &r.maximum] {
        write_option(w, n.as_ref(), |w, n| {
            w.array(4)?;
            w.text(neutral_ir::language::NUM)?;
            w.boolean(n.is_negative())?;
            w.text(n.coefficient())?;
            w.signed(n.scale())
        })?;
    }
    for n in [&r.min_length, &r.max_length] {
        write_option(w, n.as_ref(), |w, n| w.unsigned(*n))?;
    }
    Ok(())
}
/// Reads normalized restrictions; semantic compatibility remains reader-owned.
fn read_restrictions(v: &LocatedValue) -> Result<FieldRestrictions, D> {
    let a = tuple::<5>(v)?;
    let choices = read_list(&a[0], |v| read_value(v, 0))?;
    let number = |v: &LocatedValue| match read_value::<ClosedReference>(v, 0)? {
        V::Number(n) => Ok(n),
        _ => Err(schema(v)),
    };
    Ok(FieldRestrictions {
        choices: if choices.is_empty() {
            None
        } else {
            Some(choices)
        },
        minimum: read_option(&a[1], number)?,
        maximum: read_option(&a[2], number)?,
        min_length: read_option(&a[3], unsigned)?,
        max_length: read_option(&a[4], unsigned)?,
    })
}
/// Writes complete nominal bodies, including dormant defaults and all alternative types.
fn write_body(w: &mut CborWriter, b: &B) -> Result<(), E> {
    w.array(2)?;
    match b {
        B::Record(fields) => {
            w.text("record")?;
            write_list(w, fields, |w, f| {
                w.array(5)?;
                w.text(&f.name)?;
                write_type(w, &f.ty, 0)?;
                w.text(match f.presence {
                    Presence::Required => "required",
                    Presence::Optional => "optional",
                    Presence::Defaulted => "defaulted",
                })?;
                write_restrictions(w, &f.restrictions)?;
                if let Some(v) = &f.default {
                    w.array(2)?;
                    w.boolean(true)?;
                    write_value(w, v, 0)
                } else {
                    w.array(1)?;
                    w.boolean(false)
                }
            })
        }
        B::Variant(alternatives) => {
            w.text("variant")?;
            write_list(w, alternatives, |w, a| {
                w.array(2)?;
                w.text(&a.tag)?;
                write_type(w, &a.ty, 0)
            })
        }
    }
}
/// Reads one exact record/variant signature with no fallback to the old body grammar.
fn read_body(v: &LocatedValue) -> Result<B, D> {
    let a = tuple::<2>(v)?;
    match text(&a[0])? {
        "record" => Ok(B::Record(read_list(&a[1], |v| {
            let f = tuple::<5>(v)?;
            let d = array(&f[4])?;
            let default = match d {
                [flag] if !boolean(flag)? => None,
                [flag, value] if boolean(flag)? => Some(read_value(value, 0)?),
                _ => return Err(schema(v)),
            };
            Ok(CompositionField {
                name: owned_text(&f[0])?,
                ty: read_type(&f[1], 0)?,
                presence: match text(&f[2])? {
                    "required" => Presence::Required,
                    "optional" => Presence::Optional,
                    "defaulted" => Presence::Defaulted,
                    _ => return Err(schema(v)),
                },
                restrictions: read_restrictions(&f[3])?,
                default,
            })
        })?)),
        "variant" => Ok(B::Variant(read_list(&a[1], |v| {
            let a = tuple::<2>(v)?;
            Ok(CompositionAlternative {
                tag: owned_text(&a[0])?,
                ty: read_type(&a[1], 0)?,
            })
        })?)),
        _ => Err(schema(v)),
    }
}

/// Writes sixteen sections, including explicit empty catalogues and dependencies.
fn write_ir(w: &mut CborWriter, ir: &Ir) -> Result<(), E> {
    w.array(16)?;
    w.text(&ir.schema)?;
    write_list(w, &ir.modules, |w, m| {
        w.array(2)?;
        write_module(w, &m.identity)?;
        write_list(w, &m.imports, |w, s| w.text(s))
    })?;
    write_list(w, &ir.declarations, |w, d| {
        w.array(4)?;
        write_symbol(w, &d.identity)?;
        w.boolean(d.public)?;
        match &d.signature {
            S::Binding(t) => {
                w.array(2)?;
                w.text("binding")?;
                write_type(w, t, 0)?;
            }
            S::Definition(b) => write_body(w, b)?,
        }
        write_option(w, d.value.as_ref(), |w, v| write_value(w, v, 0))
    })?;
    let count = ir.vocabularies.iter().map(|b| b.definitions.len()).sum();
    w.array(count)?;
    for b in &ir.vocabularies {
        for d in &b.definitions {
            w.array(5)?;
            w.text(b.identity.identity())?;
            w.text(b.identity.version())?;
            w.text(&d.name)?;
            w.boolean(d.public)?;
            write_body(w, &d.body)?;
        }
    }
    write_list(w, &ir.vocabularies, |w, b| {
        w.array(5)?;
        w.text(b.identity.identity())?;
        w.text(b.identity.version())?;
        w.text(b.identity.schema_version())?;
        write_list(w, b.identity.required_features(), |w, s| w.text(s))?;
        w.array(b.definitions.iter().filter(|d| d.public).count())?;
        for definition in b.definitions.iter().filter(|d| d.public) {
            w.text(&definition.name)?;
        }
        Ok(())
    })?;
    w.bytes(&ir.interface_digest.as_bytes())?;
    write_companions(w, ir)?;
    write_list(w, &ir.vocabularies, |w, b| {
        w.array(3)?;
        w.text(b.identity.identity())?;
        w.text(b.identity.version())?;
        write_list(w, &b.dependencies, |w, d| {
            w.array(2)?;
            w.text(&d.identity)?;
            w.text(&d.version)
        })
    })?;
    write_list(w, &ir.origins, write_origin)?;
    write_numbers(w, &ir.composition_limits.values())?;
    write_numbers(w, &ir.composition_resources.values())
}

/// Writes the unchanged base project companions in their frozen six-slot order.
fn write_companions(w: &mut CborWriter, ir: &Ir) -> Result<(), E> {
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

/// Writes an origin separately from materialized meaning and logical identity.
fn write_origin(w: &mut CborWriter, o: &CompositionOrigin) -> Result<(), E> {
    w.array(4)?;
    write_symbol(w, &o.binding)?;
    write_list(w, &o.path, |w, p| match p {
        P::Field(n) => {
            w.array(2)?;
            w.text("field")?;
            w.text(n)
        }
        P::Element(n) => {
            w.array(2)?;
            w.text("element")?;
            w.unsigned(*n)
        }
        P::Payload => {
            w.array(1)?;
            w.text("payload")
        }
    })?;
    w.text(match o.kind {
        K::Supplied => "supplied",
        K::ExplicitNull => "explicit-null",
        K::OmittedOptional => "omitted-optional",
        K::Defaulted => "defaulted",
    })?;
    write_option(w, o.attribution.as_ref(), |w, a| match a {
        A::Source(location) => {
            w.array(2)?;
            w.text("source")?;
            write_location(w, *location)
        }
        A::Vocabulary {
            identity,
            version,
            type_name,
            field_name,
            span,
        } => {
            w.array(6)?;
            w.text("vocabulary")?;
            w.text(identity)?;
            w.text(version)?;
            w.text(type_name)?;
            w.text(field_name)?;
            write_option(w, span.as_ref(), |w, s| {
                write_numbers(w, &[s.start(), s.end()])
            })
        }
    })
}

/// Reads exact origin paths and explicit evidence availability; reader checks ownership.
fn read_origin(v: &LocatedValue) -> Result<CompositionOrigin, D> {
    let a = tuple::<4>(v)?;
    Ok(CompositionOrigin {
        binding: read_symbol(&a[0])?,
        path: read_list(&a[1], |v| {
            let p = array(v)?;
            match p {
                [tag] if text(tag)? == "payload" => Ok(P::Payload),
                [tag, n] if text(tag)? == "field" => Ok(P::Field(owned_text(n)?)),
                [tag, n] if text(tag)? == "element" => Ok(P::Element(unsigned(n)?)),
                _ => Err(schema(v)),
            }
        })?,
        kind: match text(&a[2])? {
            "supplied" => K::Supplied,
            "explicit-null" => K::ExplicitNull,
            "omitted-optional" => K::OmittedOptional,
            "defaulted" => K::Defaulted,
            _ => return Err(schema(v)),
        },
        attribution: read_option(&a[3], |v| {
            let p = array(v)?;
            match p {
                [tag, l] if text(tag)? == "source" => Ok(A::Source(read_location(l)?)),
                [tag, identity, revision, type_name, field_name, span]
                    if text(tag)? == "vocabulary" =>
                {
                    Ok(A::Vocabulary {
                        identity: owned_text(identity)?,
                        version: owned_text(revision)?,
                        type_name: owned_text(type_name)?,
                        field_name: owned_text(field_name)?,
                        span: read_option(span, |v| {
                            let a = tuple::<2>(v)?;
                            ByteSpan::new(unsigned(&a[0])?, unsigned(&a[1])?).map_err(|_| schema(v))
                        })?,
                    })
                }
                _ => Err(schema(v)),
            }
        })?,
    })
}

/// Joins all exact catalogue owners, public names, definitions and dependency records without sorting untrusted input.
fn read_vocabularies(
    p: &[LocatedValue; 16],
    vocabulary_sources: &[ProjectVocabularySource],
) -> Result<Vec<CompositionBundle>, D> {
    // Wire definitions are already ordered by exact owner/name. Group once in
    // linear order instead of allocating a tree or cloning ownership keys.
    let raw_definitions = array(&p[3])?;
    let mut definitions: Vec<((&str, &str), Vec<CompositionDefinition>)> = Vec::new();
    definitions
        .try_reserve_exact(raw_definitions.len())
        .map_err(|_| failure(C::EncodedSizeLimit))?;
    let mut previous = None;
    for d in raw_definitions {
        let a = tuple::<5>(d)?;
        let owner = (text(&a[0])?, text(&a[1])?);
        let name = owned_text(&a[2])?;
        let key = (owner, text(&a[2])?);
        if previous.as_ref().is_some_and(|p| p >= &key) {
            return Err(schema(d));
        }
        previous = Some(key);
        if definitions.last().is_none_or(|last| last.0 != owner) {
            definitions.push((owner, Vec::new()));
        }
        let group = &mut definitions.last_mut().ok_or_else(|| schema(d))?.1;
        group
            .try_reserve(1)
            .map_err(|_| failure(C::EncodedSizeLimit))?;
        group.push(CompositionDefinition {
            name,
            public: boolean(&a[3])?,
            body: read_body(&a[4])?,
        });
    }
    let dependencies = read_list(&p[12], |v| {
        let a = tuple::<3>(v)?;
        Ok((
            owned_text(&a[0])?,
            owned_text(&a[1])?,
            read_list(&a[2], |v| {
                let d = tuple::<2>(v)?;
                Ok(CompositionDependency {
                    identity: owned_text(&d[0])?,
                    version: owned_text(&d[1])?,
                })
            })?,
        ))
    })?;
    let catalogues = array(&p[4])?;
    if catalogues.len() != dependencies.len() || catalogues.len() != vocabulary_sources.len() {
        return Err(schema(&p[4]));
    }
    let mut definitions = definitions.into_iter().peekable();
    let mut vocabularies = Vec::new();
    vocabularies
        .try_reserve_exact(catalogues.len())
        .map_err(|_| failure(C::EncodedSizeLimit))?;
    for ((c, dep), source) in catalogues.iter().zip(dependencies).zip(vocabulary_sources) {
        let a = tuple::<5>(c)?;
        let identity = owned_text(&a[0])?;
        let version = owned_text(&a[1])?;
        if (identity.as_str(), version.as_str()) != (dep.0.as_str(), dep.1.as_str())
            || identity != source.identity
            || version != source.version
        {
            return Err(schema(c));
        }
        let owner = (identity.as_str(), version.as_str());
        let defs = if definitions.peek().is_some_and(|entry| entry.0 == owner) {
            definitions.next().ok_or_else(|| schema(c))?.1
        } else {
            Vec::new()
        };
        let public = read_list(&a[4], owned_text)?;
        if !defs
            .iter()
            .filter(|d| d.public)
            .map(|d| &d.name)
            .eq(public.iter())
        {
            return Err(schema(c));
        }
        vocabularies.push(CompositionBundle {
            identity: VocabularyIdentity::new(
                identity,
                version,
                owned_text(&a[2])?,
                neutral_core::allocation::text(neutral_vocabulary::composition::ENCODING_VERSION)
                    .map_err(|_| failure(C::EncodedSizeLimit))?,
                source.digest,
                read_list(&a[3], owned_text)?,
            ),
            dependencies: dep.2,
            definitions: defs,
        });
    }
    if definitions.next().is_some() {
        return Err(schema(&p[4]));
    }
    Ok(vocabularies)
}

/// Reads an exact unsigned control/fact tuple without trusting advertised semantics.
fn read_numbers<const N: usize>(value: &LocatedValue) -> Result<[u64; N], D> {
    let mut numbers = [0; N];
    for (out, item) in numbers.iter_mut().zip(tuple::<N>(value)?) {
        *out = unsigned(item)?;
    }
    Ok(numbers)
}

/// Restores complete resolved producer facts without publishing any validation authority.
fn read_ir(v: &LocatedValue) -> Result<Ir, D> {
    let p = tuple::<16>(v)?;
    if text(&p[0])? != profile::PROJECT_IR_SCHEMA {
        return Err(failure(C::UnsupportedVersion));
    }
    let modules = read_list(&p[1], |v| {
        let a = tuple::<2>(v)?;
        Ok(ProjectModule {
            identity: read_module(&a[0])?,
            imports: read_list(&a[1], owned_text)?,
        })
    })?;
    let declarations = read_list(&p[2], |v| {
        let a = tuple::<4>(v)?;
        let s = tuple::<2>(&a[2])?;
        Ok(CompositionDeclaration {
            identity: read_symbol(&a[0])?,
            public: boolean(&a[1])?,
            signature: if text(&s[0])? == "binding" {
                S::Binding(read_type(&s[1], 0)?)
            } else {
                S::Definition(read_body(&a[2])?)
            },
            value: read_option(&a[3], |v| read_value(v, 0))?,
        })
    })?;
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
    let limit_tuple = tuple::<6>(&p[9])?;
    let limits = ProjectLimits {
        modules: unsigned(&limit_tuple[0])?,
        declarations: unsigned(&limit_tuple[1])?,
        import_edges: unsigned(&limit_tuple[2])?,
        nodes: unsigned(&limit_tuple[3])?,
        text_bytes: unsigned(&limit_tuple[4])?,
        artifact_bytes: unsigned(&limit_tuple[5])?,
    };
    let resource_tuple = tuple::<7>(&p[10])?;
    let resources = ProjectResourceFacts {
        source_units: unsigned(&resource_tuple[0])?,
        source_bytes: unsigned(&resource_tuple[1])?,
        vocabulary_units: unsigned(&resource_tuple[2])?,
        vocabulary_bytes: unsigned(&resource_tuple[3])?,
        declarations: unsigned(&resource_tuple[4])?,
        import_edges: unsigned(&resource_tuple[5])?,
        value_nodes: unsigned(&resource_tuple[6])?,
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
    let vocabularies = read_vocabularies(p, &vocabulary_sources)?;
    Ok(Ir {
        schema: neutral_core::allocation::text(profile::PROJECT_IR_SCHEMA)
            .map_err(|_| failure(C::EncodedSizeLimit))?,
        modules,
        declarations,
        vocabularies,
        interface_digest: SemanticDigest::from_raw_bytes(digest(&p[5])?),
        sources,
        source_maps,
        provenance,
        limits,
        resources,
        vocabulary_sources,
        origins: read_list(&p[13], read_origin)?,
        composition_limits: CompositionPolicy::from_values(read_numbers(&p[14])?),
        composition_resources: CompositionResourceFacts::from_values(read_numbers(&p[15])?),
    })
}
