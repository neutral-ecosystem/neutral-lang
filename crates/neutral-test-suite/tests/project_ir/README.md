<!-- SPDX-License-Identifier: Apache-2.0 -->

# Historical project IR records

The `/1` project fixtures and oracle in this directory are preserved as
historical development evidence; they are not registered as executable tests.
Current complete artifacts use expression-project `/3` and the active tests
below:

| Current behavior | Executable regression |
| --- | --- |
| Source values, lists, records, and defaults | `neutral-test-suite/tests/expression_pipeline/mod.rs`: `integration_expression_source_wire_static_values_match_literal_results`; `neutral-compiler/tests/expression_source/boundaries.rs`: `nested_record_defaults_and_origins_preserve_null_and_composite_values` |
| Public views, root selection, and private-origin redaction | `neutral-test-suite/tests/expression_pipeline/inspection.rs`: `integration_expression_probe_is_independent_of_roots_and_external_bindings`; `neutral-test-suite/tests/expression_pipeline/projection.rs`: `integration_expression_decoded_projection_redacts_private_occurrences` |
| Hostile artifact validation and truncation | `neutral-test-suite/tests/transport/hostile_decoder.rs`: `security_current_decoder_rejects_old_magic_and_truncation`, `fuzz_smoke_current_artifact_byte_mutations_fail_or_decode_completely` |
| Deterministic current wire | `neutral-test-suite/tests/transport/wire.rs`: `integration_current_source_roundtrips_through_independent_reader` |

Historical files are not a decoder fallback or an active compatibility path.
Run `cargo test -p neutral-test-suite` for cross-package tests, or
`cargo xtask test all` for the complete test graph and doctests.
