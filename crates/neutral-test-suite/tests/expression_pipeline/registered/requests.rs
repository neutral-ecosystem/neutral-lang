// SPDX-License-Identifier: Apache-2.0

//! Exact literal corpus selection and caller controls, independent of runtime defaults.

use super::*;

/// Maps the closed reviewed control spellings once for compilation and explicit evaluation.
pub(super) fn controls(
    case: &J,
    registry: &J,
    overrides: bool,
) -> (ExpressionLimits, ExpansionLimits) {
    let read = |key: &str| {
        if overrides && let Some(n) = case["controls"][key].as_u64() {
            return n;
        }
        registry["defaults"]["controls"][key].as_u64().unwrap()
    };
    (
        ExpressionLimits {
            nodes: read("expression.nodes"),
            syntax_depth: read("expression.syntax-depth"),
            evaluation_depth: read("expression.evaluation-depth"),
            bindings: read("expression.bindings"),
            dependency_edges: read("expression.dependency-edges"),
            work: read("expression.work"),
            retained_bytes: read("expression.retained-bytes"),
            output_bytes: read("expression.output-bytes"),
            coefficient_digits: read("expression.coefficient-digits"),
            absolute_scale: read("expression.absolute-scale"),
            diagnostics: read("expression.diagnostics"),
        },
        ExpansionLimits {
            definitions: read("expansion.definitions"),
            applications: read("expansion.applications"),
            parameters: read("expansion.parameters"),
            axes: read("expansion.axes"),
            axis_items: read("expansion.axis-items"),
            product_tuples: read("expansion.product-tuples"),
            aggregate_tuples: read("expansion.aggregate-tuples"),
            expanded_bindings: read("expansion.expanded-bindings"),
            expanded_nodes: read("expansion.expanded-nodes"),
            depth: read("expansion.depth"),
            work: read("expansion.work"),
            retained_bytes: read("expansion.retained-bytes"),
            output_bytes: read("expansion.output-bytes"),
        },
    )
}

/// Captures explicit test-owned bytes and digest locks without acquiring any source dependency.
pub(super) fn capture(
    case: &J,
    registry: &J,
    evaluator: bool,
) -> Result<CapturedExpressionProject, String> {
    let (expression, expansion) = controls(case, registry, !evaluator);
    let mut paths = vec![case["source"].as_str().unwrap()];
    if let Some(others) = case["other_sources"].as_array() {
        paths.extend(others.iter().map(|p| p.as_str().unwrap()));
    }
    let sources = paths
        .into_iter()
        .map(|path| {
            let source = input(path);
            let module = source
                .lines()
                .find_map(|l| l.strip_prefix("module "))
                .unwrap();
            CapturedSourceInput::new(
                format!("source:{module}"),
                module,
                source.as_bytes().to_vec(),
            )
        })
        .collect();
    let mut vocabularies = Vec::new();
    if let Some(path) = case["vocabulary"].as_str() {
        let bytes = input(path).into_bytes();
        let bundle: J = serde_json::from_slice(&bytes).unwrap();
        let lock = neutral_vocabulary::VocabularyLock::new(
            bundle["identity"].as_str().unwrap(),
            bundle["version"].as_str().unwrap(),
            bundle["encoding_version"].as_str().unwrap(),
            bundle["schema_version"].as_str().unwrap(),
            neutral_core::VocabularyContentDigest::from_bytes(&bytes),
            bundle["required_features"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_owned())
                .collect(),
        )
        .unwrap();
        vocabularies.push(neutral_compiler::CapturedVocabularyInput::new(bytes, lock));
    }
    let mut profiles = SemanticProfiles::try_frozen().unwrap();
    if let Some(remove) = case["remove_feature"].as_str() {
        profiles.required_features.retain(|f| f != remove);
    }
    if let Some(add) = case["add_feature"].as_str() {
        profiles.required_features.push(add.to_owned());
        profiles.required_features.sort();
    }
    let version = case["capture_profile"]
        .as_str()
        .unwrap_or(registry["profiles"]["capture"].as_str().unwrap());
    capture_expression_project(CapturedExpressionProjectRequest::new(
        capture_envelope(sources, vocabularies, version, CancellationToken::new()),
        profiles,
        composition_limits(),
        expression,
        expansion,
    ))
    .map_err(|e| e.code().to_owned())
}
