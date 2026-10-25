// SPDX-License-Identifier: Apache-2.0

//! Shared composition-contract framing used by current expression identity.

use super::super::{
    IdentityError as E,
    framing::{Writer, ordered},
};
use crate::{
    ModuleSymbolIdentity,
    composition::{
        CompositionBody as B, CompositionBundle, CompositionValue as V, FieldPresence,
        FieldRestrictions,
    },
    language,
    project::PROJECT_MAX_DEPTH,
    project_interface::ProjectPublicType,
};
use neutral_core::profile::V1_SOURCE_PROFILE;

/// Frames logical modules in canonical order.
pub(super) fn modules(
    w: &mut Writer<'_>,
    modules: &[crate::project::ProjectModule],
) -> Result<(), E> {
    w.frame("modules", |w| {
        for m in modules {
            if m.identity.language_behavior_version() != V1_SOURCE_PROFILE
                || m.identity.module_name().is_empty()
            {
                return Err(E::InvalidInput);
            }
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

/// Frames the canonical vocabulary types section under the selected projection.
pub(super) fn vocabulary_types(
    w: &mut Writer<'_>,
    vocabularies: &[CompositionBundle],
    public: bool,
) -> Result<(), E> {
    w.frame("vocabulary-types", |w| {
        for bundle in vocabularies {
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
pub(super) fn catalogues(w: &mut Writer<'_>, vocabularies: &[CompositionBundle]) -> Result<(), E> {
    w.frame("vocabulary-catalogues", |w| {
        for b in vocabularies {
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
pub(super) fn dependencies(
    w: &mut Writer<'_>,
    vocabularies: &[CompositionBundle],
) -> Result<(), E> {
    w.frame("vocabulary-dependencies", |w| {
        for b in vocabularies {
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
pub(super) fn body(w: &mut Writer<'_>, b: &B) -> Result<(), E> {
    match b {
        B::Record(fields) => {
            ordered(fields.iter().map(|f| &f.name))?;
            w.frame("record", |w| {
                for f in fields {
                    if (f.presence == FieldPresence::Defaulted) != f.default.is_some() {
                        return Err(E::InvalidInput);
                    }
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
            if alternatives.is_empty() || alternatives.iter().any(|a| a.tag.is_empty()) {
                return Err(E::InvalidInput);
            }
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
    if let Some(choices) = &r.choices {
        w.items(choices.len())?;
        for choice in choices {
            w.check()?;
            match choice {
                V::Number(n) => w.text(n.coefficient())?,
                V::String(s) | V::Url(s) | V::Path(s) => w.text(s)?,
                V::Bool(_) => {}
                _ => return Err(E::InvalidInput),
            }
        }
        for pair in choices.windows(2) {
            if crate::composition::compare_composition_scalars(&pair[0], &pair[1])
                != Some(std::cmp::Ordering::Less)
            {
                return Err(E::InvalidInput);
            }
        }
    }
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
                Some(n) => number(w, n),
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

/// Checks shared wrapper invariants before using the unchanged `/3` type framing grammar.
pub(super) fn type_transcript(
    w: &mut Writer<'_>,
    ty: &ProjectPublicType,
    depth: usize,
) -> Result<(), E> {
    let mut inner = ty;
    let mut current_depth = depth;
    loop {
        w.check()?;
        if current_depth > PROJECT_MAX_DEPTH {
            return Err(E::Limit);
        }
        match inner {
            ProjectPublicType::Ref(target)
                if !matches!(
                    target.as_ref(),
                    ProjectPublicType::Nominal(_) | ProjectPublicType::VocabularyNominal { .. }
                ) =>
            {
                return Err(E::InvalidInput);
            }
            ProjectPublicType::Nullable(target)
                if matches!(target.as_ref(), ProjectPublicType::Nullable(_)) =>
            {
                return Err(E::InvalidInput);
            }
            ProjectPublicType::List(target)
            | ProjectPublicType::Ref(target)
            | ProjectPublicType::Nullable(target) => {
                inner = target;
                current_depth += 1;
            }
            _ => break,
        }
    }
    frozen_type_transcript(w, ty, depth)
}

/// Frames a stable source module-symbol without graph-local IDs.
pub(super) fn symbol(w: &mut Writer<'_>, identity: &ModuleSymbolIdentity) -> Result<(), E> {
    w.leaf(
        "profile",
        identity.module().language_behavior_version().as_bytes(),
    )?;
    w.leaf("module", identity.module().module_name().as_bytes())?;
    w.leaf("declaration", identity.declaration_name().as_bytes())
}

/// Frames the shared exact nominal type grammar used in `/3` identity.
fn frozen_type_transcript(
    w: &mut Writer<'_>,
    ty: &ProjectPublicType,
    depth: usize,
) -> Result<(), E> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(E::Limit);
    }
    match ty {
        ProjectPublicType::Num => w.leaf(language::NUM, &[]),
        ProjectPublicType::String => w.leaf(language::STRING, &[]),
        ProjectPublicType::Bool => w.leaf(language::BOOL, &[]),
        ProjectPublicType::Url => w.leaf(language::URL, &[]),
        ProjectPublicType::Path => w.leaf(language::PATH, &[]),
        ProjectPublicType::Nominal(identity) => w.frame("nominal", |w| symbol(w, identity)),
        ProjectPublicType::VocabularyNominal {
            identity,
            version,
            name,
        } => w.frame("vocabulary", |w| {
            w.leaf("identity", identity.as_bytes())?;
            w.leaf("version", version.as_bytes())?;
            w.leaf("name", name.as_bytes())
        }),
        ProjectPublicType::List(inner) => w.frame(language::LIST, |w| {
            frozen_type_transcript(w, inner, depth + 1)
        }),
        ProjectPublicType::Ref(inner) => w.frame(language::REF_TYPE, |w| {
            frozen_type_transcript(w, inner, depth + 1)
        }),
        ProjectPublicType::Nullable(inner) => {
            w.frame("nullable", |w| frozen_type_transcript(w, inner, depth + 1))
        }
    }
}

/// Frames borrowed exact numbers without constructing an allocating temporary value.
fn number(w: &mut Writer<'_>, n: &crate::ExactNumber) -> Result<(), E> {
    w.frame("num", |w| {
        w.leaf("negative", &[u8::from(n.is_negative())])?;
        w.leaf("coefficient", n.coefficient().as_bytes())?;
        w.leaf("scale", &n.scale().to_be_bytes())
    })
}

/// Lets closed values share framing without permitting non-null default references.
pub(super) trait Reference {
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
pub(super) fn value<R: Reference>(w: &mut Writer<'_>, v: &V<R>, depth: usize) -> Result<(), E> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(E::Limit);
    }
    match v {
        V::Null => w.leaf("null", &[]),
        V::Bool(b) => w.leaf("bool", &[u8::from(*b)]),
        V::Number(n) => number(w, n),
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
