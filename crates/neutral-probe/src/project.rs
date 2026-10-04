// SPDX-License-Identifier: Apache-2.0

//! Reader-only complete-project inspection with a redacted public projection.

use crate::inspection_schema::{Field, FieldValue, render_fields_json};
use neutral_core::CancellationToken;
use neutral_encoding::{DecodeError, DecodeLimits, project::decode_project};
use neutral_reader::{ProjectLimits, ProjectReadError, ProjectView, ValidatedProject, ViewRequest};
use std::collections::BTreeMap;

/// Atomically classified project inspection failure, with no partial summary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectProbeError {
    /// External byte/schema/reader validation failed.
    Decode(DecodeError),
    /// Post-validation root selection failed.
    View(ProjectReadError),
}

/// Complete-project counts and a selected public view, never private provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectProbeSummary {
    /// Complete module count, not a list of private module names.
    pub modules: u64,
    /// Complete declaration count, independent of root selection.
    pub declarations: u64,
    /// Public-only interpretive dependency closure and materialized values.
    pub view: ProjectView,
    /// Complete public-interface fingerprint, not a complete project identity.
    pub interface_fingerprint: String,
    /// Complete IR contract identification.
    pub schema: String,
}

/// Inspects external project bytes without compiler linkage or host acquisition.
///
/// An absent selection means all public exports; an explicit empty selection
/// means an empty view. Complete IR is validated regardless of selected roots.
///
/// # Errors
/// Rejects malformed bytes or invalid/private/duplicate roots atomically.
pub fn inspect_project_encoded(
    bytes: &[u8],
    wire_limits: DecodeLimits,
    project_limits: ProjectLimits,
    roots: Option<&[String]>,
    cancellation: &CancellationToken,
) -> Result<ProjectProbeSummary, ProjectProbeError> {
    let project = decode_project(bytes, wire_limits, project_limits, cancellation)
        .map_err(ProjectProbeError::Decode)?;
    let ir = project.complete_ir();
    let roots = if let Some(names) = roots {
        if names.len() > ir.public_interface.exports().len() {
            return Err(ProjectProbeError::View(ProjectReadError::Limit));
        }
        let index = ir
            .public_interface
            .exports()
            .iter()
            .map(|export| (symbol_name(export.identity()), export.identity()))
            .collect::<BTreeMap<_, _>>();
        names
            .iter()
            .map(|name| {
                if cancellation.is_cancelled() {
                    return Err(ProjectProbeError::View(ProjectReadError::Cancelled));
                }
                index
                    .get(name)
                    .map(|identity| (*identity).clone())
                    .ok_or(ProjectProbeError::View(ProjectReadError::View))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        ir.public_interface
            .exports()
            .iter()
            .map(|export| export.identity().clone())
            .collect()
    };
    let view = project
        .derive_view(
            &ViewRequest {
                schema: neutral_reader::PROJECT_VIEW_SCHEMA.to_owned(),
                roots,
            },
            cancellation,
        )
        .map_err(ProjectProbeError::View)?;
    Ok(summarize_project(&project, view))
}

/// Returns a bounded trusted projection; source IDs, spans and private roots are absent.
fn summarize_project(project: &ValidatedProject, view: ProjectView) -> ProjectProbeSummary {
    let ir = project.complete_ir();
    ProjectProbeSummary {
        modules: ir.resources.source_units,
        declarations: ir.resources.declarations,
        view,
        interface_fingerprint: ir.public_interface.fingerprint().to_string(),
        schema: ir.schema.clone(),
    }
}

/// Formats one public module-symbol identity for root selection and inspection.
fn symbol_name(symbol: &neutral_reader::ModuleSymbolIdentity) -> String {
    format!(
        "{}::{}",
        symbol.module().module_name(),
        symbol.declaration_name()
    )
}

/// Renders indented JSON with the same escaping rules as document inspection.
#[must_use]
pub fn render_project_summary_json(summary: &ProjectProbeSummary) -> String {
    let roots = summary
        .view
        .roots()
        .iter()
        .map(symbol_name)
        .collect::<Vec<_>>();
    let exports = summary
        .view
        .exports()
        .iter()
        .map(|export| symbol_name(export.identity()))
        .collect::<Vec<_>>();
    let values = summary
        .view
        .values()
        .iter()
        .map(|(symbol, value)| format!("{} = {value:?}", symbol_name(symbol)))
        .collect::<Vec<_>>();
    let vocabularies = summary
        .view
        .vocabulary_records()
        .iter()
        .map(|record| format!("{}@{}::{}", record.identity, record.version, record.name))
        .collect::<Vec<_>>();
    let modules = summary.modules.to_string();
    let declarations = summary.declarations.to_string();
    render_fields_json(&[
        Field {
            json_key: "schema",
            text_prefix: "schema",
            value: FieldValue::Text(&summary.schema),
        },
        Field {
            json_key: "view_schema",
            text_prefix: "view",
            value: FieldValue::Text(summary.view.schema()),
        },
        Field {
            json_key: "modules",
            text_prefix: "modules",
            value: FieldValue::Text(&modules),
        },
        Field {
            json_key: "declarations",
            text_prefix: "declarations",
            value: FieldValue::Text(&declarations),
        },
        Field {
            json_key: "interface_fingerprint",
            text_prefix: "interface",
            value: FieldValue::Text(&summary.interface_fingerprint),
        },
        Field {
            json_key: "roots",
            text_prefix: "root",
            value: FieldValue::TextList(&roots),
        },
        Field {
            json_key: "exports",
            text_prefix: "export",
            value: FieldValue::TextList(&exports),
        },
        Field {
            json_key: "values",
            text_prefix: "value",
            value: FieldValue::TextList(&values),
        },
        Field {
            json_key: "vocabulary_types",
            text_prefix: "vocabulary",
            value: FieldValue::TextList(&vocabularies),
        },
    ])
}
