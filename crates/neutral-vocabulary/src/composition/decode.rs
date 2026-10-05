// SPDX-License-Identifier: Apache-2.0

//! Closed member sets and bounded decoding for the explicitly selected schema.

use super::{
    Budget, CapturedCompositionBundle, CompositionError as E, ENCODING_VERSION, REQUIRED_FEATURE,
    SCHEMA_VERSION, charge, check_count,
};
use crate::{
    JsonValue as J, ProjectVocabularyType, VocabularyError, json, schema, validation as v,
};
use neutral_ir::{
    ExactNumber, VocabularyIdentity,
    composition::{
        ClosedValue as V, CompositionAlternative, CompositionBody, CompositionBundle,
        CompositionDefinition, CompositionDependency, CompositionField, FieldPresence,
        FieldRestrictions,
    },
    project_interface::ProjectPublicType as T,
};
use std::collections::BTreeSet;

/// Decodes one exact lock into a raw contract; global closure/default validation follows.
pub(super) fn bundle(
    input: CapturedCompositionBundle<'_>,
    budget: &mut Budget<'_>,
) -> Result<CompositionBundle, E> {
    if input.lock.schema_version() == crate::PROJECT_VOCABULARY_SCHEMA_VERSION {
        return legacy_bundle(input, budget);
    }
    if input.lock.schema_version() != SCHEMA_VERSION {
        return Err(VocabularyError::UnsupportedSchemaVersion.into());
    }
    if input.bytes.len() as u64 > budget.limits.json.bundle_bytes() {
        return Err(VocabularyError::BundleByteLimitExceeded.into());
    }
    if input.bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(VocabularyError::BomForbidden.into());
    }
    let text = std::str::from_utf8(input.bytes).map_err(|_| VocabularyError::InvalidUtf8)?;
    let parsed = json::parse_cancellable(text, budget.limits.json, budget.cancellation).map_err(
        |error| {
            if error == VocabularyError::Cancelled {
                E::Cancelled
            } else {
                E::from(error)
            }
        },
    )?;
    budget.step(1)?;
    let root = v::object(&parsed)?;
    v::exact_members(
        root,
        &[
            "format",
            "encoding_version",
            "schema_version",
            "identity",
            "version",
            "required_features",
            "dependencies",
            "types",
        ],
    )?;
    if v::string(root, "format")? != schema::BUNDLE_FORMAT {
        return Err(VocabularyError::UnsupportedFormat.into());
    }
    if v::string(root, "encoding_version")? != ENCODING_VERSION
        || input.lock.encoding_version() != ENCODING_VERSION
    {
        return Err(VocabularyError::UnsupportedEncodingVersion.into());
    }
    if v::string(root, "schema_version")? != SCHEMA_VERSION {
        return Err(VocabularyError::UnsupportedSchemaVersion.into());
    }
    if v::string(root, "identity")? != input.lock.identity()
        || v::string(root, "version")? != input.lock.version()
    {
        return Err(VocabularyError::LockMismatch.into());
    }
    let features = v::array(v::member(root, "required_features")?)?;
    check_count(features.len(), budget.limits.json.features())?;
    if features != [J::String(REQUIRED_FEATURE.to_owned())] {
        return Err(VocabularyError::UnknownRequiredFeature.into());
    }
    if input.lock.required_features() != [REQUIRED_FEATURE] {
        return Err(VocabularyError::LockMismatch.into());
    }
    let dependencies = dependencies(
        v::member(root, "dependencies")?,
        input.lock.identity(),
        budget,
    )?;
    let definitions = definitions(v::member(root, "types")?, input.lock, budget)?;
    Ok(CompositionBundle {
        identity: identity(input),
        dependencies,
        definitions,
    })
}

/// Decodes the exact declared dependency set before allocating its graph.
fn dependencies(
    raw: &J,
    owner: &str,
    budget: &mut Budget<'_>,
) -> Result<Vec<CompositionDependency>, E> {
    let raw_dependencies = v::array(raw)?;
    check_count(
        raw_dependencies.len(),
        budget.limits.dependencies_per_bundle,
    )?;
    charge(
        &mut budget.edges,
        raw_dependencies.len() as u64,
        budget.limits.dependency_edges,
    )?;
    let mut dependencies = Vec::new();
    let mut seen = BTreeSet::new();
    for raw in raw_dependencies {
        budget.step(1)?;
        let dependency = v::object(raw)?;
        v::exact_members(dependency, &["identity", "version"])?;
        let identity = v::string(dependency, "identity")?;
        let version = v::string(dependency, "version")?;
        budget.key(identity.len() + version.len(), raw_dependencies.len())?;
        if !schema::is_upper_name(identity)
            || !schema::is_exact_release_version(version)
            || identity == owner
            || !seen.insert(identity)
        {
            return Err(E::InvalidDependency);
        }
        dependencies.push(CompositionDependency {
            identity: identity.to_owned(),
            version: version.to_owned(),
        });
    }
    dependencies.sort();
    Ok(dependencies)
}

/// Decodes the complete nominal namespace; field and alternative parsing stay separate.
fn definitions(
    raw: &J,
    lock: &crate::VocabularyLock,
    budget: &mut Budget<'_>,
) -> Result<Vec<CompositionDefinition>, E> {
    let raw_types = v::array(raw)?;
    check_count(raw_types.len(), budget.limits.json.types())?;
    charge(
        &mut budget.types,
        raw_types.len() as u64,
        budget.limits.total_types,
    )?;
    let mut definitions = Vec::new();
    let mut names = BTreeSet::new();
    for raw in raw_types {
        budget.step(1)?;
        let fields = v::object(raw)?;
        let kind = v::string(fields, "kind")?;
        let allowed = match kind {
            "record" => &["kind", "name", "public", "fields"][..],
            "variant" => &["kind", "name", "public", "alternatives"][..],
            _ => return Err(E::InvalidContract),
        };
        v::exact_members(fields, allowed)?;
        let name = v::string(fields, "name")?;
        budget.key(name.len(), raw_types.len())?;
        if !schema::is_upper_name(name) || schema::is_protected_name(name) {
            return Err(VocabularyError::InvalidTypeName.into());
        }
        if !names.insert(name) {
            return Err(VocabularyError::DuplicateType.into());
        }
        let body = if kind == "record" {
            record(v::member(fields, "fields")?, lock, budget)?
        } else {
            variant(v::member(fields, "alternatives")?, lock, budget)?
        };
        definitions.push(CompositionDefinition {
            name: name.to_owned(),
            public: v::boolean(fields, "public")?,
            body,
        });
    }
    definitions.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(definitions)
}

/// Copies exact captured facts into the shared raw identity container.
fn identity(input: CapturedCompositionBundle<'_>) -> VocabularyIdentity {
    VocabularyIdentity::new(
        input.lock.identity(),
        input.lock.version(),
        input.lock.schema_version(),
        input.lock.encoding_version(),
        input.lock.content_digest(),
        input.lock.required_features().to_vec(),
    )
}

/// Adapts existing schema leaf contracts without changing their parser or meaning.
fn legacy_bundle(
    input: CapturedCompositionBundle<'_>,
    budget: &mut Budget<'_>,
) -> Result<CompositionBundle, E> {
    let legacy = crate::validate_project_bundle(input.bytes, input.lock, budget.limits.json)?;
    charge(
        &mut budget.types,
        legacy.types().len() as u64,
        budget.limits.total_types,
    )?;
    let mut definitions = Vec::new();
    for ty in legacy.types() {
        budget.step(1)?;
        charge(
            &mut budget.fields,
            ty.fields().len() as u64,
            budget.limits.total_fields,
        )?;
        let fields = ty
            .fields()
            .iter()
            .map(|field| {
                let ty = match field.ty() {
                    ProjectVocabularyType::Num => T::Num,
                    ProjectVocabularyType::String => T::String,
                    ProjectVocabularyType::Bool => T::Bool,
                    ProjectVocabularyType::Url => T::Url,
                    ProjectVocabularyType::Path => T::Path,
                    ProjectVocabularyType::Nominal(name) => T::VocabularyNominal {
                        identity: legacy.identity().to_owned(),
                        version: legacy.version().to_owned(),
                        name: name.clone(),
                    },
                };
                CompositionField {
                    name: field.name().to_owned(),
                    ty,
                    presence: FieldPresence::Required,
                    restrictions: FieldRestrictions::default(),
                    default: None,
                }
            })
            .collect();
        definitions.push(CompositionDefinition {
            name: ty.name().to_owned(),
            public: ty.is_public(),
            body: CompositionBody::Record(fields),
        });
    }
    Ok(CompositionBundle {
        identity: identity(input),
        dependencies: Vec::new(),
        definitions,
    })
}

/// Decodes fields and independently charges aggregate field work before allocation.
fn record(
    raw: &J,
    lock: &crate::VocabularyLock,
    budget: &mut Budget<'_>,
) -> Result<CompositionBody, E> {
    let raw = v::array(raw)?;
    check_count(raw.len(), budget.limits.json.fields())?;
    charge(
        &mut budget.fields,
        raw.len() as u64,
        budget.limits.total_fields,
    )?;
    let mut fields = Vec::new();
    let mut names = BTreeSet::new();
    for item in raw {
        budget.step(1)?;
        let field = v::object(item)?;
        let presence = match v::string(field, "presence")? {
            "required" => FieldPresence::Required,
            "optional" => FieldPresence::Optional,
            "defaulted" => FieldPresence::Defaulted,
            _ => return Err(E::InvalidContract),
        };
        let members = if presence == FieldPresence::Defaulted {
            &["name", "type", "presence", "restrictions", "default"][..]
        } else {
            &["name", "type", "presence", "restrictions"][..]
        };
        v::exact_members(field, members)?;
        let name = v::string(field, "name")?;
        budget.key(name.len(), raw.len())?;
        if !schema::is_snake_name(name) || schema::is_protected_name(name) {
            return Err(VocabularyError::InvalidFieldName.into());
        }
        if !names.insert(name) {
            return Err(VocabularyError::DuplicateField.into());
        }
        fields.push(CompositionField {
            name: name.to_owned(),
            ty: ty(v::member(field, "type")?, lock, budget, 1)?,
            presence,
            restrictions: restrictions(v::member(field, "restrictions")?, budget)?,
            default: if presence == FieldPresence::Defaulted {
                Some(closed(v::member(field, "default")?, budget, 1)?)
            } else {
                None
            },
        });
    }
    fields.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(CompositionBody::Record(fields))
}

/// Decodes nonempty, uniquely tagged alternatives under separate variant budgets.
fn variant(
    raw: &J,
    lock: &crate::VocabularyLock,
    budget: &mut Budget<'_>,
) -> Result<CompositionBody, E> {
    let raw = v::array(raw)?;
    if raw.is_empty() {
        return Err(E::InvalidContract);
    }
    check_count(raw.len(), budget.limits.alternatives_per_type)?;
    charge(
        &mut budget.alternatives,
        raw.len() as u64,
        budget.limits.total_alternatives,
    )?;
    let mut alternatives = Vec::new();
    let mut tags = BTreeSet::new();
    for item in raw {
        budget.step(1)?;
        let fields = v::object(item)?;
        v::exact_members(fields, &["tag", "type"])?;
        let tag = v::string(fields, "tag")?;
        budget.key(tag.len(), raw.len())?;
        if !schema::is_snake_name(tag) || schema::is_protected_name(tag) || !tags.insert(tag) {
            return Err(E::InvalidContract);
        }
        alternatives.push(CompositionAlternative {
            tag: tag.to_owned(),
            ty: ty(v::member(fields, "type")?, lock, budget, 1)?,
        });
    }
    alternatives.sort_by(|a, b| a.tag.cmp(&b.tag));
    Ok(CompositionBody::Variant(alternatives))
}

/// Resolves bounded type objects to the existing alias-independent nominal/list/ref model.
fn ty(raw: &J, lock: &crate::VocabularyLock, budget: &mut Budget<'_>, depth: u64) -> Result<T, E> {
    budget.depth(depth, budget.limits.type_depth)?;
    let fields = v::object(raw)?;
    let kind = v::string(fields, "kind")?;
    match kind {
        "num" | "string" | "bool" | "url" | "path" => {
            v::exact_members(fields, &["kind"])?;
            Ok(match kind {
                "num" => T::Num,
                "string" => T::String,
                "bool" => T::Bool,
                "url" => T::Url,
                _ => T::Path,
            })
        }
        "nominal" | "external" => {
            let (identity, version) = if kind == "nominal" {
                v::exact_members(fields, &["kind", "name"])?;
                (lock.identity(), lock.version())
            } else {
                v::exact_members(fields, &["kind", "identity", "version", "name"])?;
                (
                    v::string(fields, "identity")?,
                    v::string(fields, "version")?,
                )
            };
            let name = v::string(fields, "name")?;
            if !schema::is_upper_name(name)
                || schema::is_protected_name(name)
                || !schema::is_upper_name(identity)
                || !schema::is_exact_release_version(version)
                || (kind == "external" && identity == lock.identity())
            {
                return Err(E::InvalidContract);
            }
            Ok(T::VocabularyNominal {
                identity: identity.to_owned(),
                version: version.to_owned(),
                name: name.to_owned(),
            })
        }
        "list" | "nullable" | "ref" => {
            let member = match kind {
                "list" => "element",
                "nullable" => "inner",
                _ => "target",
            };
            v::exact_members(fields, &["kind", member])?;
            let inner = ty(v::member(fields, member)?, lock, budget, depth + 1)?;
            match kind {
                "list" => Ok(T::List(Box::new(inner))),
                "nullable" if !matches!(inner, T::Nullable(_)) => Ok(T::Nullable(Box::new(inner))),
                "ref" if matches!(inner, T::VocabularyNominal { .. }) => {
                    Ok(T::Ref(Box::new(inner)))
                }
                _ => Err(E::InvalidContract),
            }
        }
        _ => Err(E::InvalidContract),
    }
}

/// Parses a strict, bounded Neutral numeric literal rather than a host float.
fn number(text: &str, budget: &mut Budget<'_>) -> Result<ExactNumber, E> {
    budget.step(text.len() as u64)?;
    ExactNumber::from_source(
        text,
        budget.limits.json.numeric_digits,
        budget.limits.json.numeric_scale,
    )
    .map_err(|error| {
        if error == neutral_ir::IrError::ExactNumberLimitExceeded {
            E::Limit
        } else {
            E::InvalidContract
        }
    })
}

/// Decodes canonical unsigned length strings without overflow or ambiguous spellings.
fn length(raw: &J) -> Result<u64, E> {
    let J::String(text) = raw else {
        return Err(E::InvalidRestrictions);
    };
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(E::InvalidRestrictions);
    }
    text.parse().map_err(|_| E::InvalidRestrictions)
}

/// Decodes optional restriction members; contextual compatibility is checked later.
fn restrictions(raw: &J, budget: &mut Budget<'_>) -> Result<FieldRestrictions, E> {
    let fields = v::object(raw)?;
    v::members(
        fields,
        &["choices", "minimum", "maximum", "min_length", "max_length"],
    )?;
    let mut result = FieldRestrictions::default();
    for (name, raw) in fields {
        budget.step(1)?;
        match name.as_str() {
            "choices" => {
                let raw = v::array(raw)?;
                if raw.is_empty() {
                    return Err(E::InvalidRestrictions);
                }
                check_count(raw.len(), budget.limits.choices_per_field)?;
                charge(
                    &mut budget.choices,
                    raw.len() as u64,
                    budget.limits.total_choices,
                )?;
                result.choices = Some(
                    raw.iter()
                        .map(|raw| closed(raw, budget, 1))
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            "minimum" | "maximum" => {
                let J::String(text) = raw else {
                    return Err(E::InvalidRestrictions);
                };
                let value = number(text, budget)?;
                if name == "minimum" {
                    result.minimum = Some(value);
                } else {
                    result.maximum = Some(value);
                }
            }
            "min_length" => result.min_length = Some(length(raw)?),
            "max_length" => result.max_length = Some(length(raw)?),
            _ => unreachable!("member set was checked"),
        }
    }
    Ok(result)
}

/// Decodes closed values only; reference/alias/expression tags have no accepted branch.
fn closed(raw: &J, budget: &mut Budget<'_>, depth: u64) -> Result<V, E> {
    budget.depth(depth, budget.limits.value_depth)?;
    let fields = v::object(raw)?;
    let kind = v::string(fields, "kind")?;
    match kind {
        "num" | "string" | "bool" | "url" | "path" => {
            v::exact_members(fields, &["kind", "value"])?;
            Ok(match kind {
                "num" => V::Number(number(v::string(fields, "value")?, budget)?),
                "bool" => V::Bool(v::boolean(fields, "value")?),
                "string" => V::String(v::string(fields, "value")?.to_owned()),
                "url" => V::Url(v::string(fields, "value")?.to_owned()),
                _ => V::Path(v::string(fields, "value")?.to_owned()),
            })
        }
        "null" => {
            v::exact_members(fields, &["kind"])?;
            Ok(V::Null)
        }
        "list" => {
            v::exact_members(fields, &["kind", "items"])?;
            let raw = v::array(v::member(fields, "items")?)?;
            check_count(raw.len(), budget.limits.json.array_items())?;
            Ok(V::List(
                raw.iter()
                    .map(|raw| closed(raw, budget, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        "record" => {
            v::exact_members(fields, &["kind", "fields"])?;
            let raw = v::array(v::member(fields, "fields")?)?;
            check_count(raw.len(), budget.limits.json.fields())?;
            let mut seen = BTreeSet::new();
            let mut values = Vec::new();
            let field_count = raw.len();
            for raw in raw {
                budget.step(1)?;
                let fields = v::object(raw)?;
                v::exact_members(fields, &["name", "value"])?;
                let name = v::string(fields, "name")?;
                budget.key(name.len(), field_count)?;
                if !seen.insert(name) {
                    return Err(E::InvalidDefault);
                }
                values.push((
                    name.to_owned(),
                    Some(closed(v::member(fields, "value")?, budget, depth + 1)?),
                ));
            }
            values.sort_by(|a, b| a.0.cmp(&b.0));
            Ok(V::Record(values))
        }
        "variant" => {
            v::exact_members(fields, &["kind", "tag", "payload"])?;
            Ok(V::Variant {
                tag: v::string(fields, "tag")?.to_owned(),
                payload: Box::new(closed(v::member(fields, "payload")?, budget, depth + 1)?),
            })
        }
        _ => Err(E::InvalidDefault),
    }
}
