<!-- SPDX-License-Identifier: Apache-2.0 -->

# Historical public semantics records

The `/1` `ProjectInterface` and probe fixtures in this directory remain
historical development evidence and are no longer executable tests. Current
capture and public-reader behavior is exercised through expression-project
`/3`:

| Current behavior | Executable regression |
| --- | --- |
| Capture and closed imports | `neutral-test-suite/tests/composition_contract/capture.rs`: `integration_composition_capture_diamond_and_independent_public_reader` |
| Typed public views and references | `neutral-test-suite/tests/expression_pipeline/projection.rs`: `integration_expression_decoded_projection_keeps_every_future_obligation` |
| Vocabulary interpretation | `neutral-test-suite/tests/expression_pipeline/inspection.rs`: `integration_expression_probe_preserves_vocabulary_interpretation_contracts` |
| Private-origin redaction | `neutral-test-suite/tests/expression_pipeline/projection.rs`: `integration_expression_decoded_projection_redacts_private_occurrences`; `neutral-test-suite/tests/expression_pipeline/source_qualification.rs`: `integration_source_reuse_survives_wire_without_exposing_private_origins` |
| Compiler-free probe behavior | `neutral-test-suite/tests/expression_pipeline/inspection.rs`: `integration_expression_probe_is_independent_of_roots_and_external_bindings` |

These current tests do not reactivate the historical source profile or
project-interface wire format. Run `cargo test -p neutral-test-suite
expression_pipeline` for the current reader boundary.
