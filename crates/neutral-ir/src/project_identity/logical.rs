// SPDX-License-Identifier: Apache-2.0

//! Complete typed logical form; source/processing companions never enter this layer.

use super::{
    IdentityError, IdentityLimits, IdentityTranscript, LOGICAL_DOMAIN, LogicalProjectIdentity,
    framing::{Writer, ordered},
    transcript,
};
use crate::{
    ModuleSymbolIdentity, language,
    project::{PROJECT_IR_SCHEMA, PROJECT_MAX_DEPTH, ProjectIr, ProjectValue},
    project_interface::{ProjectPublicSignature, ProjectPublicType},
};
use neutral_core::{CancellationToken, profile::V1_SOURCE_PROFILE};

/// Builds bounded complete logical form for structurally canonical, semantically validated IR.
///
/// # Errors
/// Rejects noncanonical/schema input, byte/node/depth bounds, or cancellation.
/// Independent semantic reader validation remains mandatory before trusting an untrusted producer.
pub fn canonical_logical_project(
    ir: &ProjectIr,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<LogicalProjectIdentity>, IdentityError> {
    transcript(
        LOGICAL_DOMAIN,
        limits,
        cancellation,
        |writer| {
            if ir.schema != PROJECT_IR_SCHEMA || ir.modules.is_empty() {
                return Err(IdentityError::InvalidInput);
            }
            writer.items(ir.modules.len())?;
            writer.items(ir.declarations.len())?;
            writer.items(ir.vocabulary_records.len())?;
            preflight_keys(writer, ir)?;
            let schema_limits = crate::project::ProjectLimits {
                modules: limits.nodes.min(super::MAX_TRANSCRIPT_NODES),
                declarations: limits.nodes.min(super::MAX_TRANSCRIPT_NODES),
                import_edges: limits.nodes.min(super::MAX_TRANSCRIPT_NODES),
                nodes: limits.nodes.min(super::MAX_TRANSCRIPT_NODES),
                text_bytes: limits.bytes.min(super::MAX_TRANSCRIPT_BYTES),
                artifact_bytes: limits.bytes.min(super::MAX_TRANSCRIPT_BYTES),
            };
            if !ir.within_schema_limits(schema_limits) {
                return Err(IdentityError::Limit);
            }
            ordered(ir.modules.iter().map(|module| &module.identity))?;
            ordered(ir.declarations.iter().map(|decl| &decl.identity))?;
            ordered(
                ir.vocabulary_records
                    .iter()
                    .map(|record| (&record.identity, &record.version, &record.name)),
            )?;
            writer.leaf("schema", ir.schema.as_bytes())?;
            writer.leaf("profile", V1_SOURCE_PROFILE.as_bytes())?;
            writer.frame("modules", |writer| {
                for module in &ir.modules {
                    if module.identity.language_behavior_version() != V1_SOURCE_PROFILE {
                        return Err(IdentityError::InvalidInput);
                    }
                    writer.items(module.imports.len())?;
                    ordered(&module.imports)?;
                    writer.frame("module", |writer| {
                        writer.leaf("name", module.identity.module_name().as_bytes())?;
                        writer.frame("imports", |writer| {
                            for import in &module.imports {
                                writer.leaf("target", import.as_bytes())?;
                            }
                            Ok(())
                        })
                    })?;
                }
                Ok(())
            })?;
            declarations(writer, ir)?;
            vocabularies(writer, ir)?;
            public_edges(writer, ir)
        },
        LogicalProjectIdentity,
    )
}

/// Caps aggregate identity-key comparison work before comparing canonical root sequences.
fn preflight_keys(writer: &mut Writer<'_>, ir: &ProjectIr) -> Result<(), IdentityError> {
    let mut imports = 0_usize;
    for module in &ir.modules {
        writer.text(module.identity.module_name())?;
        imports = imports
            .checked_add(module.imports.len())
            .ok_or(IdentityError::Limit)?;
        writer.items(imports)?;
        for target in &module.imports {
            writer.text(target)?;
        }
    }
    for declaration in &ir.declarations {
        writer.text(declaration.identity.module().language_behavior_version())?;
        writer.text(declaration.identity.module().module_name())?;
        writer.text(declaration.identity.declaration_name())?;
    }
    for record in &ir.vocabulary_records {
        writer.text(&record.identity)?;
        writer.text(&record.version)?;
        writer.text(&record.name)?;
    }
    Ok(())
}

/// Frames stable full module-symbol identity without graph-local labels.
pub(super) fn symbol(
    writer: &mut Writer<'_>,
    identity: &ModuleSymbolIdentity,
) -> Result<(), IdentityError> {
    writer.leaf(
        "profile",
        identity.module().language_behavior_version().as_bytes(),
    )?;
    writer.leaf("module", identity.module().module_name().as_bytes())?;
    writer.leaf("declaration", identity.declaration_name().as_bytes())
}

/// Frames complete public/private declarations and all unused closed defaults.
fn declarations(writer: &mut Writer<'_>, ir: &ProjectIr) -> Result<(), IdentityError> {
    writer.frame("declarations", |writer| {
        for decl in &ir.declarations {
            writer.frame("declaration", |writer| {
                symbol(writer, &decl.identity)?;
                writer.leaf("public", &[u8::from(decl.public)])?;
                signature(writer, &decl.signature)?;
                writer.frame("value", |writer| match &decl.value {
                    Some(value) => value_transcript(writer, value, 0),
                    None => writer.leaf("absent", &[]),
                })?;
                writer.frame("defaults", |writer| fields(writer, &decl.defaults, 0))
            })?;
        }
        Ok(())
    })
}

/// Frames the full locked interpretation schemas and public nominal catalogues.
fn vocabularies(writer: &mut Writer<'_>, ir: &ProjectIr) -> Result<(), IdentityError> {
    writer.frame("vocabulary-records", |writer| {
        for record in &ir.vocabulary_records {
            writer.frame("record", |writer| {
                writer.leaf("identity", record.identity.as_bytes())?;
                writer.leaf("version", record.version.as_bytes())?;
                writer.leaf("name", record.name.as_bytes())?;
                writer.leaf("public", &[u8::from(record.public)])?;
                writer.items(record.fields.len())?;
                for (name, _) in &record.fields {
                    writer.text(name)?;
                }
                ordered(record.fields.iter().map(|(name, _)| name))?;
                writer.frame("fields", |writer| {
                    for (name, ty) in &record.fields {
                        writer.frame("field", |writer| {
                            writer.leaf("name", name.as_bytes())?;
                            type_transcript(writer, ty, 0)
                        })?;
                    }
                    Ok(())
                })
            })?;
        }
        Ok(())
    })?;
    writer.items(ir.public_interface.vocabularies().len())?;
    for vocabulary in ir.public_interface.vocabularies() {
        writer.text(vocabulary.identity())?;
        writer.text(vocabulary.version())?;
    }
    ordered(
        ir.public_interface
            .vocabularies()
            .iter()
            .map(|vocabulary| (vocabulary.identity(), vocabulary.version())),
    )?;
    writer.frame("vocabulary-catalogues", |writer| {
        for vocabulary in ir.public_interface.vocabularies() {
            writer.frame("vocabulary", |writer| {
                writer.leaf("identity", vocabulary.identity().as_bytes())?;
                writer.leaf("version", vocabulary.version().as_bytes())?;
                writer.items(vocabulary.public_types().len())?;
                for name in vocabulary.public_types() {
                    writer.text(name)?;
                }
                ordered(vocabulary.public_types())?;
                writer.frame("public-types", |writer| {
                    for name in vocabulary.public_types() {
                        writer.leaf("name", name.as_bytes())?;
                    }
                    Ok(())
                })
            })?;
        }
        Ok(())
    })
}

/// Preserves validated public dependency topology, excluding occurrence locations.
fn public_edges(writer: &mut Writer<'_>, ir: &ProjectIr) -> Result<(), IdentityError> {
    writer.items(ir.public_interface.edges().len())?;
    for edge in ir.public_interface.edges() {
        for identity in [edge.from(), edge.to()] {
            writer.text(identity.module().language_behavior_version())?;
            writer.text(identity.module().module_name())?;
            writer.text(identity.declaration_name())?;
        }
    }
    ordered(
        ir.public_interface
            .edges()
            .iter()
            .map(|edge| (edge.from(), edge.kind(), edge.to())),
    )?;
    writer.frame("public-edges", |writer| {
        for edge in ir.public_interface.edges() {
            writer.frame("edge", |writer| {
                writer.frame("from", |writer| symbol(writer, edge.from()))?;
                writer.frame("to", |writer| symbol(writer, edge.to()))?;
                writer.leaf("kind", edge.kind().spelling().as_bytes())
            })?;
        }
        Ok(())
    })
}

/// Frames distinct binding and record signatures with canonical field order.
fn signature(
    writer: &mut Writer<'_>,
    signature: &ProjectPublicSignature,
) -> Result<(), IdentityError> {
    writer.frame("signature", |writer| match signature {
        ProjectPublicSignature::Binding(ty) => {
            writer.frame("binding", |writer| type_transcript(writer, ty, 0))
        }
        ProjectPublicSignature::Record(fields) => {
            writer.items(fields.len())?;
            for field in fields {
                writer.text(field.name())?;
            }
            ordered(
                fields
                    .iter()
                    .map(crate::project_interface::ProjectPublicField::name),
            )?;
            writer.frame("record", |writer| {
                for field in fields {
                    writer.frame("field", |writer| {
                        writer.leaf("name", field.name().as_bytes())?;
                        type_transcript(writer, field.ty(), 0)
                    })?;
                }
                Ok(())
            })
        }
    })
}

/// Frames typed meaning without source aliases or recursive reference expansion.
fn type_transcript(
    writer: &mut Writer<'_>,
    ty: &ProjectPublicType,
    depth: usize,
) -> Result<(), IdentityError> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(IdentityError::Limit);
    }
    match ty {
        ProjectPublicType::Num => writer.leaf(language::NUM, &[]),
        ProjectPublicType::String => writer.leaf(language::STRING, &[]),
        ProjectPublicType::Bool => writer.leaf(language::BOOL, &[]),
        ProjectPublicType::Url => writer.leaf(language::URL, &[]),
        ProjectPublicType::Path => writer.leaf(language::PATH, &[]),
        ProjectPublicType::Nominal(identity) => {
            writer.frame("nominal", |writer| symbol(writer, identity))
        }
        ProjectPublicType::VocabularyNominal {
            identity,
            version,
            name,
        } => writer.frame("vocabulary", |writer| {
            writer.leaf("identity", identity.as_bytes())?;
            writer.leaf("version", version.as_bytes())?;
            writer.leaf("name", name.as_bytes())
        }),
        ProjectPublicType::List(inner) => writer.frame(language::LIST, |writer| {
            type_transcript(writer, inner, depth + 1)
        }),
        ProjectPublicType::Ref(inner) => writer.frame(language::REF_TYPE, |writer| {
            type_transcript(writer, inner, depth + 1)
        }),
        ProjectPublicType::Nullable(inner) => writer.frame("nullable", |writer| {
            type_transcript(writer, inner, depth + 1)
        }),
    }
}

/// Frames materialized typed values, preserving list order and exact rational normalization.
fn value_transcript(
    writer: &mut Writer<'_>,
    value: &ProjectValue,
    depth: usize,
) -> Result<(), IdentityError> {
    if depth > PROJECT_MAX_DEPTH {
        return Err(IdentityError::Limit);
    }
    match value {
        ProjectValue::Null => writer.leaf(language::NULL, &[]),
        ProjectValue::Bool(value) => writer.leaf(language::BOOL, &[u8::from(*value)]),
        ProjectValue::Number(value) => writer.frame(language::NUM, |writer| {
            writer.leaf("negative", &[u8::from(value.is_negative())])?;
            writer.leaf("coefficient", value.coefficient().as_bytes())?;
            writer.leaf("scale", &value.scale().to_be_bytes())
        }),
        ProjectValue::String(value) => writer.leaf(language::STRING, value.as_bytes()),
        ProjectValue::Url(value) => writer.leaf(language::URL, value.as_bytes()),
        ProjectValue::Path(value) => writer.leaf(language::PATH, value.as_bytes()),
        ProjectValue::Reference(identity) => {
            writer.frame(language::REF_TYPE, |writer| symbol(writer, identity))
        }
        ProjectValue::List(values) => {
            writer.items(values.len())?;
            writer.frame(language::LIST, |writer| {
                for value in values {
                    value_transcript(writer, value, depth + 1)?;
                }
                Ok(())
            })
        }
        ProjectValue::Record(values) => {
            writer.frame(language::RECORD, |writer| fields(writer, values, depth + 1))
        }
    }
}

/// Frames sorted unique fields and defaults without depending on source field order.
fn fields(
    writer: &mut Writer<'_>,
    fields: &[(String, ProjectValue)],
    depth: usize,
) -> Result<(), IdentityError> {
    writer.items(fields.len())?;
    for (name, _) in fields {
        writer.text(name)?;
    }
    ordered(fields.iter().map(|(name, _)| name))?;
    for (name, value) in fields {
        writer.frame("field", |writer| {
            writer.leaf("name", name.as_bytes())?;
            value_transcript(writer, value, depth)
        })?;
    }
    Ok(())
}
