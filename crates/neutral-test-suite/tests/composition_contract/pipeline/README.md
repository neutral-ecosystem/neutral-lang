<!-- SPDX-License-Identifier: Apache-2.0 -->

# Retired `/2` project pipeline tests

The former pipeline registered `/2` complete-project capture, compilation,
encoding, reader, probe, and identity assertions. Those test modules are no
longer active. Frozen oracle inputs and the raw schema-2 vocabulary/catalogue
tests remain in the parent `composition_contract` suite.

Current regressions replace the retired end-to-end coverage:

| Retired coverage | Current named regression |
| --- | --- |
| Source scalars, arithmetic, nullable values, lists, records/defaults, reuse, and reference cycles | `neutral-test-suite/tests/expression_pipeline/mod.rs::integration_expression_source_wire_static_values_match_literal_results`; `neutral-compiler/tests/expression_source/boundaries.rs::nested_record_defaults_and_origins_preserve_null_and_composite_values`, `nullable_lists_and_inert_url_path_literals_keep_exact_values`; `neutral-compiler/tests/expression_source/mod.rs::ordinary_reuse_cycles_imperative_and_chained_comparisons_reject` |
| Source graph/import selection and exact-byte provenance | `neutral-test-suite/tests/source_pipeline/mod.rs::property_current_sources_capture_compile_and_round_trip`, `property_source_order_does_not_change_artifact`, `current_semantics_and_closed_imports_reject_invalid_sources` |
| Reader public views, references, and private-origin redaction | `neutral-reader/tests/composition.rs::reader_composition_materializes_public_closed_values_independently`; `neutral-reader/tests/composition_scope.rs::composition_scope_reader_private_roots_are_redacted`; `neutral-test-suite/tests/expression_pipeline/projection.rs::integration_expression_decoded_projection_redacts_private_occurrences` |
| Vocabulary interpretation and revision controls | `neutral-vocabulary/tests/composition.rs::composition_complete_bundle_retains_contracts_and_defaults`; `neutral-test-suite/tests/expression_pipeline/inspection.rs::integration_expression_probe_preserves_vocabulary_interpretation_contracts`; `neutral-test-suite/tests/expression_pipeline/cache.rs::integration_expression_cache_refreshes_vocabulary_defaults_and_restrictions` |
| Consumer negotiation, selector paths, demand, reports, and failure recovery | `neutral-test-suite/tests/expression_pipeline/consumer.rs::integration_expression_reports_preserve_selected_demand_and_generation_identity`, `integration_expression_output_roots_validate_before_demand_and_recover`, `integration_restriction_debt_survives_until_value_supply`; `neutral-reader/tests/expression/binding_reports.rs::consumer_binding_and_report_faults_recover_without_revoking_accepted_reports` |
| Identity, derivation, and cache invariants | `neutral-ir/tests/expression_identity.rs`; `neutral-test-suite/tests/expression_pipeline/cache.rs::integration_expression_cache_clean_warm_changed_units_and_source_evidence_agree`; `neutral-test-suite/tests/expression_pipeline/consumer.rs::integration_cached_expression_artifact_has_literal_reports_and_immutable_identity` |
| Bounds, cancellation, allocation recovery, hostile frames, and deterministic encoding | `neutral-test-suite/tests/expression_pipeline/bounds.rs::integration_expression_transport_positive_retention_controls_are_independent`; `neutral-compiler/tests/expression_source/fault_sweeps.rs::expression_source_allocation_and_cancellation_sweeps_preserve_request_reuse`; `neutral-test-suite/tests/transport/hostile_decoder.rs::security_current_decoder_rejects_old_magic_and_truncation`, `fuzz_smoke_current_artifact_byte_mutations_fail_or_decode_completely`; `neutral-test-suite/tests/transport/wire.rs::property_current_expression_encoding_is_deterministic` |
| Standalone current probe and source formatting | `neutral-test-suite/tests/source_pipeline/system.rs::system_cli_current_source_artifact_and_probe`; `neutral-test-suite/tests/expression_pipeline/inspection.rs::integration_expression_probe_is_independent_of_roots_and_external_bindings`; `neutral-compiler/tests/expression_source/current_formatter.rs::property_current_expression_source_formatting_is_idempotent_and_recompilable` |

The replaced `/2` suites are not evidence that the `/3` pipeline passes; only
the current tests above provide development evidence.
