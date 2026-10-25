<!-- SPDX-License-Identifier: Apache-2.0 -->

# Historical project identity records

The vectors and independent Python oracle here are immutable `/1` evidence and
are not executed by current development tests. Current identity behavior uses
the complete expression-project `/3` pipeline:

| Current behavior | Executable regression |
| --- | --- |
| Capture transcript bytes, digest, bounds, allocation failure and recovery | `neutral-compiler/tests/expression_source/capture.rs`: `captured_expression_identity_matches_exact_byte_oracle`, `captured_expression_identity_rejects_bounds_and_recovers_after_faults` |
| Complete identity and changed-source cache behavior | `neutral-test-suite/tests/expression_pipeline/cache.rs`: `integration_expression_cache_clean_warm_changed_units_and_source_evidence_agree` |
| Vocabulary/default identity refresh | `neutral-test-suite/tests/expression_pipeline/cache.rs`: `integration_expression_cache_refreshes_vocabulary_defaults_and_restrictions` |
| Consumer generation and binding identity | `neutral-test-suite/tests/expression_pipeline/consumer/generations.rs`: `property_binding_revisions_do_not_rewrite_project_identity`; `neutral-test-suite/tests/expression_pipeline/consumer.rs`: `integration_cached_expression_artifact_has_literal_reports_and_immutable_identity` |
| Public roots and redacted inspection | `neutral-test-suite/tests/expression_pipeline/inspection.rs`: `integration_expression_probe_is_independent_of_roots_and_external_bindings` |

Run `cargo test -p neutral-test-suite expression_pipeline` and
`cargo test -p neutral-compiler expression_source::capture` for current
regressions. Historical `/1` fixtures are not accepted by the current decoder.
