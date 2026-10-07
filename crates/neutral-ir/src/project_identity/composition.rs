// SPDX-License-Identifier: Apache-2.0

//! Successor logical framing; capture, origins and resource companions are excluded.

use super::{
    CompositionLogicalIdentity, IdentityError as E, IdentityLimits, IdentityTranscript,
    framing::{Writer, ordered},
    logical::{symbol, type_transcript},
    transcript_profile,
};
use crate::{
    composition::{
        CompositionBody as B, CompositionValue as V, FieldPresence, FieldRestrictions, profile,
        project::{CompositionProjectIr, CompositionSignature as S, inspect_composition_ir},
    },
    project::PROJECT_MAX_DEPTH,
    project_interface::ProjectPublicEdgeKind,
};
use neutral_core::{CancellationToken, SemanticDigest, profile::V1_SOURCE_PROFILE};
use std::collections::BTreeSet;

/// Frames complete successor meaning without accepting source evidence as logical content.
///
/// # Errors
/// Rejects malformed canonical order, schema, bounds and cancellation. This is
/// structural framing, not a substitute for independent semantic reader validation.
pub fn canonical_composition_project(
    ir: &CompositionProjectIr,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<CompositionLogicalIdentity>, E> {
    build(
        ir,
        limits,
        cancellation,
        false,
        profile::LOGICAL_DOMAIN,
        CompositionLogicalIdentity,
    )
}

/// Recomputes the successor public interpretive projection, excluding private implementation facts.
///
/// # Errors
/// Rejects malformed structure, bounds or cancellation; semantic public closure
/// must be established by the caller's independent reader before trusting this digest.
pub fn composition_interface(
    ir: &CompositionProjectIr,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<SemanticDigest>, E> {
    build(
        ir,
        limits,
        cancellation,
        true,
        profile::INTERFACE_DOMAIN,
        |digest| digest,
    )
}

/// Selects only versioned framing; package releases and host mappings never choose semantics.
fn build<I>(
    ir: &CompositionProjectIr,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
    public: bool,
    domain: &str,
    wrap: impl FnOnce(SemanticDigest) -> I,
) -> Result<IdentityTranscript<I>, E> {
    if ir.schema != profile::PROJECT_IR_SCHEMA || ir.modules.is_empty() {
        return Err(E::InvalidInput);
    }
    inspect_composition_ir(ir, ir.limits, ir.composition_limits, cancellation).map_err(
        |e| match e {
            crate::composition::project::CompositionShapeError::Cancelled => E::Cancelled,
            crate::composition::project::CompositionShapeError::Limit => E::Limit,
        },
    )?;
    transcript_profile(
        domain,
        profile::IDENTITY_PROFILE,
        limits,
        cancellation,
        |w| {
            ordered(ir.modules.iter().map(|m| &m.identity))?;
            ordered(ir.declarations.iter().map(|d| &d.identity))?;
            ordered(
                ir.vocabularies
                    .iter()
                    .map(|b| (b.identity.identity(), b.identity.version())),
            )?;
            w.leaf("schema", ir.schema.as_bytes())?;
            w.leaf("profile", V1_SOURCE_PROFILE.as_bytes())?;
            modules(w, ir)?;
            declarations(w, ir, public)?;
            vocabulary_types(w, ir, public)?;
            catalogues(w, ir)?;
            dependencies(w, ir)?;
            let visible = ir
                .declarations
                .iter()
                .filter(|d| d.public)
                .map(|d| &d.identity)
                .collect::<BTreeSet<_>>();
            let edges = ir
                .provenance
                .iter()
                .filter(|e| {
                    visible.contains(&e.from)
                        && visible.contains(&e.to)
                        && matches!(
                            e.kind,
                            ProjectPublicEdgeKind::Type
                                | ProjectPublicEdgeKind::ReferenceType
                                | ProjectPublicEdgeKind::Reference
                        )
                })
                .map(|e| (&e.from, e.kind, &e.to))
                .collect::<BTreeSet<_>>();
            w.frame("public-edges", |w| {
                for (from, kind, to) in edges {
                    w.frame("edge", |w| {
                        w.frame("from", |w| symbol(w, from))?;
                        w.frame("to", |w| symbol(w, to))?;
                        w.leaf("kind", kind.spelling().as_bytes())
                    })?;
                }
                Ok(())
            })
        },
        wrap,
    )
}

/// Frames the canonical modules section under the selected projection.
fn modules(w: &mut Writer<'_>, ir: &CompositionProjectIr) -> Result<(), E> {
    w.frame("modules", |w| {
        for m in &ir.modules {
            ordered(&m.imports)?;
            w.frame("module", |w| {
                w.leaf("name", m.identity.module_name().as_bytes())?;
                w.frame("imports", |w| {
                    for i in &m.imports {
                        w.leaf("target", i.as_bytes())?;
                    }
                    Ok(())
                })
            })?;
        }
        Ok(())
    })
}

/// Frames the canonical declarations section under the selected projection.
fn declarations(w: &mut Writer<'_>, ir: &CompositionProjectIr, public: bool) -> Result<(), E> {
    w.frame("declarations", |w| {
        for d in ir.declarations.iter().filter(|d| !public || d.public) {
            w.frame("declaration", |w| {
                symbol(w, &d.identity)?;
                w.leaf("public", &[u8::from(d.public)])?;
                w.frame("signature", |w| match &d.signature {
                    S::Binding(t) => w.frame("binding", |w| type_transcript(w, t, 0)),
                    S::Definition(b) => body(w, b),
                })?;
                w.frame("value", |w| match &d.value {
                    Some(v) => value(w, v, 0),
                    None => w.leaf("absent", &[]),
                })
            })?;
        }
        Ok(())
    })
}

/// Frames the canonical vocabulary types section under the selected projection.
fn vocabulary_types(w: &mut Writer<'_>, ir: &CompositionProjectIr, public: bool) -> Result<(), E> {
    w.frame("vocabulary-types", |w| {
        for bundle in &ir.vocabularies {
            for d in bundle.definitions.iter().filter(|d| !public || d.public) {
                w.frame("definition", |w| {
                    w.leaf("identity", bundle.identity.identity().as_bytes())?;
                    w.leaf("version", bundle.identity.version().as_bytes())?;
                    w.leaf("name", d.name.as_bytes())?;
                    w.leaf("public", &[u8::from(d.public)])?;
                    w.frame("body", |w| body(w, &d.body))
                })?;
            }
        }
        Ok(())
    })
}

/// Frames the canonical catalogues section under the selected projection.
fn catalogues(w: &mut Writer<'_>, ir: &CompositionProjectIr) -> Result<(), E> {
    w.frame("vocabulary-catalogues", |w| {
        for b in &ir.vocabularies {
            w.frame("vocabulary", |w| {
                w.leaf("identity", b.identity.identity().as_bytes())?;
                w.leaf("version", b.identity.version().as_bytes())?;
                w.leaf("schema-version", b.identity.schema_version().as_bytes())?;
                w.frame("features", |w| {
                    for f in b.identity.required_features() {
                        w.leaf("feature", f.as_bytes())?;
                    }
                    Ok(())
                })?;
                w.frame("public-types", |w| {
                    for d in b.definitions.iter().filter(|d| d.public) {
                        w.leaf("name", d.name.as_bytes())?;
                    }
                    Ok(())
                })
            })?;
        }
        Ok(())
    })
}

/// Frames the canonical dependencies section under the selected projection.
fn dependencies(w: &mut Writer<'_>, ir: &CompositionProjectIr) -> Result<(), E> {
    w.frame("vocabulary-dependencies", |w| {
        for b in &ir.vocabularies {
            w.frame("vocabulary", |w| {
                w.leaf("identity", b.identity.identity().as_bytes())?;
                w.leaf("version", b.identity.version().as_bytes())?;
                w.frame("dependencies", |w| {
                    for d in &b.dependencies {
                        w.frame("dependency", |w| {
                            w.leaf("identity", d.identity.as_bytes())?;
                            w.leaf("version", d.version.as_bytes())
                        })?;
                    }
                    Ok(())
                })
            })?;
        }
        Ok(())
    })
}

/// Frames one source/vocabulary body with complete unused contract facts.
fn body(w: &mut Writer<'_>, b: &B) -> Result<(), E> {
    match b {
        B::Record(fields) => {
            ordered(fields.iter().map(|f| &f.name))?;
            w.frame("record", |w| {
                for f in fields {
                    w.frame("field", |w| {
                        w.leaf("name", f.name.as_bytes())?;
                        type_transcript(w, &f.ty, 0)?;
                        w.leaf(
                            "presence",
                            match f.presence {
                                FieldPresence::Required => b"required",
                                FieldPresence::Optional => b"optional",
                                FieldPresence::Defaulted => b"defaulted",
                            },
                        )?;
                        restrictions(w, &f.restrictions)?;
                        w.frame("default", |w| match &f.default {
                            Some(v) => value(w, v, 0),
                            None => w.leaf("absent", &[]),
                        })
                    })?;
                }
                Ok(())
            })
        }
        B::Variant(alternatives) => {
            ordered(alternatives.iter().map(|a| &a.tag))?;
            w.frame("variant", |w| {
                for a in alternatives {
                    w.frame("alternative", |w| {
                        w.leaf("tag", a.tag.as_bytes())?;
                        type_transcript(w, &a.ty, 0)
                    })?;
                }
                Ok(())
            })
        }
    }
}

/// Preserves absence of constraints separately from present finite or numeric bounds.
fn restrictions(w: &mut Writer<'_>, r: &FieldRestrictions) -> Result<(), E> {
    w.frame("restrictions", |w| {
        w.frame("choices", |w| {
            if let Some(values) = &r.choices {
                for v in values {
                    value(w, v, 0)?;
                }
            }
            Ok(())
        })?;
        for (tag, bound) in [("minimum", &r.minimum), ("maximum", &r.maximum)] {
            w.frame(tag, |w| match bound {
                Some(n) => value::<std::convert::Infallible>(w, &V::Number(n.clone()), 0),
                None => w.leaf("absent", &[]),
            })?;
        }
        for (tag, bound) in [("min-length", r.min_length), ("max-length", r.max_length)] {
            w.frame(tag, |w| match bound {
                Some(n) => w.number("length", n),
                None => w.leaf("absent", &[]),
            })?;
        }
        Ok(())
    })
}

/// Lets closed values share framing without permitting non-null default references.
trait Reference {
    /// Frames an identity edge without traversing its target.
    fn write(&self, w: &mut Writer<'_>) -> Result<(), E>;
}
impl Reference for crate::ModuleSymbolIdentity {
    /// Uses the unchanged stable symbol tuple.
    fn write(&self, w: &mut Writer<'_>) -> Result<(), E> {
        symbol(w, self)
    }
}
impl Reference for std::convert::Infallible {
    /// Closed defaults cannot construct this branch.
    fn write(&self, _: &mut Writer<'_>) -> Result<(), E> {
        match *self {}
    }
}

/// Frames materialized values; optional absence is only emitted by a record field.
fn value<R: Reference>(w: &mut Writer<'_>, v: &V<R>, depth: usize) -> Result<(), E> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(E::Limit);
    }
    match v {
        V::Null => w.leaf("null", &[]),
        V::Bool(b) => w.leaf("bool", &[u8::from(*b)]),
        V::Number(n) => w.frame("num", |w| {
            w.leaf("negative", &[u8::from(n.is_negative())])?;
            w.leaf("coefficient", n.coefficient().as_bytes())?;
            w.leaf("scale", &n.scale().to_be_bytes())
        }),
        V::String(s) => w.leaf("string", s.as_bytes()),
        V::Url(s) => w.leaf("url", s.as_bytes()),
        V::Path(s) => w.leaf("path", s.as_bytes()),
        V::Reference(r) => w.frame("Ref", |w| r.write(w)),
        V::List(vs) => w.frame("List", |w| {
            for v in vs {
                value(w, v, depth + 1)?;
            }
            Ok(())
        }),
        V::Record(fields) => {
            ordered(fields.iter().map(|(n, _)| n))?;
            w.frame("record", |w| {
                for (n, v) in fields {
                    w.frame("field", |w| {
                        w.leaf("name", n.as_bytes())?;
                        match v {
                            Some(v) => value(w, v, depth + 1),
                            None => w.leaf("omitted", &[]),
                        }
                    })?;
                }
                Ok(())
            })
        }
        V::Variant { tag, payload } => w.frame("variant", |w| {
            w.leaf("tag", tag.as_bytes())?;
            w.frame("payload", |w| value(w, payload, depth + 1))
        }),
    }
}
