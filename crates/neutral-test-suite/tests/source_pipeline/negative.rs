// SPDX-License-Identifier: Apache-2.0

//! Individually classified rejected programs from the current source corpus.

use super::*;

fn rejects(name: &str, units: &[(&str, &str)], expected_code: &str) {
    let captured = capture_result(units)
        .unwrap_or_else(|failure| panic!("{name}: capture failed with {}", failure.code()));
    let failure = compile_expression_project(&captured, &CancellationToken::new())
        .expect_err(&format!("{name}: invalid source compiled"));
    assert_eq!(failure.code, expected_code, "{name}");

    if let Some(location) = failure.location {
        let source = captured
            .sources()
            .iter()
            .find(|source| source.digest() == location.source())
            .unwrap_or_else(|| panic!("{name}: diagnostic digest is not a captured source"));
        let span = location.span();
        assert!(span.start() < span.end(), "{name}: empty diagnostic span");
        assert!(
            span.end() <= source.bytes().len() as u64,
            "{name}: span exceeds exact source bytes"
        );
    }
}

macro_rules! negative_case {
    ($test:ident, $name:literal, [$($unit:expr),+ $(,)?], $code:literal) => {
        #[test]
        fn $test() {
            rejects($name, &[$($unit),+], $code);
        }
    };
}

negative_case!(
    integration_rejects_cache_invalid,
    "cache-invalid",
    [
        source!("negative", "cache-invalid", "app::consumer"),
        source!("positive", "shared", "app::shared")
    ],
    "NEU-EXP-003"
);
negative_case!(
    integration_rejects_cycle_broken,
    "cycle-broken",
    [
        source!("negative", "cycle-broken", "cycle::left"),
        source!("positive", "cycle-right", "cycle::right")
    ],
    "NEU-EXP-007"
);
negative_case!(
    integration_rejects_default_type,
    "default-type",
    [source!("negative", "default-type", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_duplicate_field,
    "duplicate-field",
    [source!("negative", "duplicate-field", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_duplicate_final_field,
    "duplicate-final-field",
    [source!("negative", "duplicate-final-field", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_duplicate_public,
    "duplicate-public",
    [source!("negative", "duplicate-public", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_function_call,
    "function",
    [source!("negative", "function", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_indirect_private_reference,
    "indirect-private-reference",
    [source!("negative", "indirect-private-reference", "api")],
    "NEU-EXP-003"
);
negative_case!(
    integration_rejects_list_element,
    "list-element",
    [source!("negative", "list-element", "api")],
    "NEU-EXP-004"
);
negative_case!(
    integration_rejects_missing_field_separator,
    "missing-field-separator",
    [source!("negative", "missing-field-separator", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_missing_field,
    "missing-field",
    [source!("negative", "missing-field", "api")],
    "NEU-EXP-004"
);
negative_case!(
    integration_rejects_missing_import,
    "missing-import",
    [source!("negative", "missing-import", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_nominal_mismatch,
    "nominal-mismatch",
    [source!("negative", "nominal-mismatch", "api")],
    "NEU-EXP-004"
);
negative_case!(
    integration_rejects_nonnullable_null,
    "nonnullable-null",
    [source!("negative", "nonnullable-null", "api")],
    "NEU-EXP-004"
);
negative_case!(
    integration_rejects_private_import,
    "private-import",
    [
        source!("negative", "private-import", "app::consumer"),
        source!("positive", "shared", "app::shared")
    ],
    "NEU-EXP-003"
);
negative_case!(
    integration_rejects_private_reference,
    "private-reference",
    [source!("negative", "private-reference", "api")],
    "NEU-EXP-003"
);
negative_case!(
    integration_rejects_private_type,
    "private-type",
    [source!("negative", "private-type", "api")],
    "NEU-COM-010"
);
negative_case!(
    integration_rejects_record_value_cycle,
    "record-value-cycle",
    [source!("negative", "record-value-cycle", "api")],
    "NEU-COM-012"
);
negative_case!(
    integration_rejects_reference_type,
    "reference-type",
    [source!("negative", "reference-type", "api")],
    "NEU-EXP-004"
);
negative_case!(
    integration_rejects_relative_import,
    "relative-import",
    [source!("negative", "relative-import", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_self_import,
    "self-import",
    [source!("negative", "self-import", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_trailing_token,
    "trailing-token",
    [source!("negative", "trailing-token", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_type_mismatch,
    "type-mismatch",
    [source!("negative", "type-mismatch", "api")],
    "NEU-EXP-004"
);
negative_case!(
    integration_rejects_unknown_field,
    "unknown-field",
    [source!("negative", "unknown-field", "api")],
    "NEU-EXP-004"
);
negative_case!(
    integration_rejects_unknown_name,
    "unknown-name",
    [source!("negative", "unknown-name", "api")],
    "NEU-EXP-003"
);
negative_case!(
    integration_rejects_unterminated_string,
    "unterminated-string",
    [source!("negative", "unterminated-string", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_url_import,
    "url-import",
    [source!("negative", "url-import", "api")],
    "NEU-EXP-002"
);
negative_case!(
    integration_rejects_value_cycle,
    "value-cycle",
    [source!("negative", "value-cycle", "api")],
    "NEU-EXP-007"
);
negative_case!(
    integration_rejects_wildcard_import,
    "wildcard-import",
    [source!("negative", "wildcard-import", "api")],
    "NEU-EXP-002"
);
