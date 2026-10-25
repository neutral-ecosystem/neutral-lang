<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cross-package current wire regressions

`wire.rs` and `hostile_decoder.rs` exercise current source-to-`/3` encoding,
independent reader validation, deterministic re-encoding, old-magic rejection,
truncation, mutation, and artifact bounds. They use the current `/3` source
fixture builder; frozen `0.1` conformance inputs remain historical records and
are not edited or executed as active development tests.

The former whole-document acceptance modules are retired. Their useful language
coverage remains in current named tests:

| Retired coverage | Current regression |
| --- | --- |
| Scalars, exact numbers, and computed values | `neutral-compiler/tests/expression_source/mod.rs`: `frozen_arithmetic_precedence_and_decimal_results`, `construction_match_and_interpolation_literals` |
| Nullable values, lists, records, and defaults | `neutral-compiler/tests/expression_source/boundaries.rs`: `nullable_lists_and_inert_url_path_literals_keep_exact_values`, `nested_record_defaults_and_origins_preserve_null_and_composite_values`, `nominal_constructors_reject_invalid_fields_and_tags` |
| Reuse, references, and cycles | `neutral-compiler/tests/expression_source/mod.rs`: `ordinary_reuse_cycles_imperative_and_chained_comparisons_reject`; `neutral-test-suite/tests/expression_pipeline/source_qualification.rs`: `integration_source_reuse_survives_wire_without_exposing_private_origins` |
| Vocabulary contracts and restrictions | `neutral-test-suite/tests/expression_pipeline/inspection.rs`: `integration_expression_probe_preserves_vocabulary_interpretation_contracts`; `neutral-test-suite/tests/expression_pipeline/cache.rs`: `integration_expression_cache_refreshes_vocabulary_defaults_and_restrictions` |
| Syntax rejection and source bounds | `neutral-compiler/tests/expression_source/boundaries.rs`: `contextual_parser_rejects_malformed_source`, `source_expression_limits_reject_before_publication`; `neutral-compiler/tests/expression_source/capture.rs`: `expression_capture_rejects_unsupported_selection_before_parsing` |
| Formatting and determinism | `neutral-compiler/tests/expression_source/current_formatter.rs`: `property_current_expression_source_formatting_is_idempotent_and_recompilable`; `neutral-test-suite/tests/source_pipeline/mod.rs`: `property_source_order_does_not_change_artifact` |

Run the focused current tests with `cargo test -p neutral-test-suite transport::wire`
and `cargo test -p neutral-test-suite transport::hostile_decoder`. Use
`cargo xtask test all` for the complete repository test graph and doctests.
