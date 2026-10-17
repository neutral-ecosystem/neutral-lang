// SPDX-License-Identifier: Apache-2.0

//! Positive complete-artifact boundaries must fail below real counts, not merely reject zero.

use super::*;
use neutral_compiler::CapturedVocabularyInput;
use neutral_core::VocabularyContentDigest;
use neutral_vocabulary::{VocabularyLock, composition};
use serde_json::{Value, json};

/// Builds a diamond of explicit inert captured bundles, including finite choices.
fn vocabularies() -> Vec<CapturedVocabularyInput> {
    let mut domain: Value = serde_json::from_slice(include_bytes!("fixtures/domain.json")).unwrap();
    domain["dependencies"] =
        json!([{"identity":"Base","version":"1.0.0"},{"identity":"Common","version":"1.0.0"}]);
    domain["types"].as_array_mut().unwrap().push(json!({
        "kind":"record", "name":"Options", "public":true,
        "fields":[{"name":"mode","type":{"kind":"string"},"presence":"defaulted",
            "restrictions":{"choices":[{"kind":"string","value":"safe"},{"kind":"string","value":"fast"}]},
            "default":{"kind":"string","value":"safe"}},
            {"name":"base","type":{"kind":"external","identity":"Base","version":"1.0.0","name":"Marker"},"presence":"optional","restrictions":{}},
            {"name":"common","type":{"kind":"external","identity":"Common","version":"1.0.0","name":"Marker"},"presence":"optional","restrictions":{}}]
    }));
    let bundle = |identity, dependencies| {
        json!({
            "format":"neutral-vocabulary-bundle", "encoding_version":composition::ENCODING_VERSION,
            "schema_version":composition::SCHEMA_VERSION, "identity":identity, "version":"1.0.0",
            "required_features":[composition::REQUIRED_FEATURE], "dependencies":dependencies,
            "types":[{"kind":"record","name":"Marker","public":true,"fields":[]}]
        })
    };
    let mut base = bundle("Base", json!([{"identity":"Common","version":"1.0.0"}]));
    base["types"][0]["fields"] = json!([{"name":"common","type":{"kind":"external","identity":"Common","version":"1.0.0","name":"Marker"},"presence":"optional","restrictions":{}}]);
    [domain, base, bundle("Common", json!([]))]
        .into_iter()
        .map(|bundle| {
            let bytes = serde_json::to_vec_pretty(&bundle).unwrap();
            let lock = VocabularyLock::new(
                bundle["identity"].as_str().unwrap(),
                "1.0.0",
                composition::ENCODING_VERSION,
                composition::SCHEMA_VERSION,
                VocabularyContentDigest::from_bytes(&bytes),
                vec![composition::REQUIRED_FEATURE.to_owned()],
            )
            .unwrap();
            CapturedVocabularyInput::new(bytes, lock)
        })
        .collect()
}

/// Uses multiple real modules/templates/parameters so positive below-bound controls cannot pass.
fn artifact() -> (Vec<u8>, ValidatedExpressionProject) {
    let sources = [
        ("pipeline", include_bytes!("fixtures/bounds.neu").as_slice()),
        (
            "pipeline::shared",
            include_bytes!("fixtures/shared.neu").as_slice(),
        ),
        (
            "pipeline::other",
            include_bytes!("fixtures/other.neu").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(module, source)| {
        CapturedSourceInput::new(format!("source:{module}"), module, source.to_vec())
    })
    .collect();
    pipeline_from_capture(&capture_sources(sources, vocabularies()))
}

/// Covers all controls whose positive retained counts can be reconstructed from a complete artifact.
#[test]
fn integration_expression_transport_positive_retention_controls_are_independent() {
    type Control = fn(&mut SemanticLimits) -> &mut u64;
    macro_rules! controls {
        ($($family:ident { $($field:ident),* $(,)? }),* $(,)?) => {
            &[$($( (concat!(stringify!($family), ".", stringify!($field)),
                (|p: &mut SemanticLimits| &mut p.$family.$field) as Control), )*)*]
        };
    }
    let controls: &[(&str, Control)] = controls!(
        project {
            modules,
            declarations,
            import_edges,
            nodes,
            text_bytes,
            artifact_bytes
        },
        composition {
            bundles,
            captured_bytes,
            dependencies_per_bundle,
            dependency_edges,
            dependency_depth,
            total_types,
            total_fields,
            alternatives_per_type,
            total_alternatives,
            choices_per_field,
            total_choices,
            type_depth,
            value_depth,
            value_nodes,
            work
        },
        expression {
            nodes,
            syntax_depth,
            evaluation_depth,
            bindings,
            dependency_edges,
            work,
            retained_bytes,
            coefficient_digits,
            absolute_scale
        },
        expansion {
            definitions,
            parameters,
            axes,
            axis_items,
            work,
            retained_bytes,
            output_bytes
        },
    );
    // Expansion visitation/depth and evaluator output budgets govern execution, not retained history.
    let (bytes, project) = artifact();
    let cancel = CancellationToken::new();
    for &(name, select) in controls {
        let mut policy = project.model().limits();
        let (mut lower, mut upper) = (1, *select(&mut policy));
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            *select(&mut policy) = middle;
            if decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel).is_ok() {
                upper = middle;
            } else {
                lower = middle + 1;
            }
        }
        assert!(
            upper > 1,
            "{name}: fixture must exercise a positive below-bound rejection"
        );
        *select(&mut policy) = upper;
        let accepted = decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel)
            .unwrap_or_else(|error| panic!("{name} inclusive boundary {upper}: {error:?}"));
        *select(&mut policy) -= 1;
        assert_eq!(
            decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel)
                .unwrap_err()
                .class(),
            neutral_encoding::DecodeErrorClass::EncodedSizeLimit,
            "{name}"
        );
        assert_eq!(accepted.ir(), project.ir());
        *select(&mut policy) = upper;
        assert_eq!(
            decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel)
                .unwrap()
                .ir(),
            project.ir(),
            "{name} recovery"
        );
    }
}
