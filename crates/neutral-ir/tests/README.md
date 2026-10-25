<!-- SPDX-License-Identifier: Apache-2.0 -->

# IR tests

Run `cargo test -p neutral-ir` for all current tests and doctests. Raw shared
scalar/type/value tests remain in the library unit tests, composition tests and
interface-boundary tests. `/3` identity transcript bytes are literal fixtures
in `expression_identity.rs` and have not been regenerated.

The old complete-document and `/1`/`/2` complete-project model tests were
retired with their model APIs:

| Retired assertion | Current named coverage |
| --- | --- |
| Meaning versus capture/processing companions | `expression_identity::private_values_and_source_companions_affect_only_the_governed_partitions`; `expression_identity::all_five_partitions_match_complete_frozen_residual_and_template_bytes` |
| Meaning changes from residuals and contract defaults | `expression_identity::frozen_context_and_residual_variations_keep_their_exact_partitions`; `expression_identity::restrictions_and_unused_closed_defaults_are_complete_logical_meaning` |
| Transcript framing bounds, order, cancellation and recovery | `project_identity::framing` unit tests; `expression_identity::all_partitions_enforce_exact_bytes_nodes_and_atomic_fault_cancellation_recovery` |
| Retained composition facts for current raw expression data | `unit/expression_retention_inspection.rs::expression_reference_values_charge_their_identity`; `tests/expression.rs::composition_preflight_visits_closed_choices_and_defaults`; `tests/expression.rs::contract_preflight_rejects_deep_types_before_copy` |

The former graph-local `ElementId` alpha-equivalence and document accessor tests
had no corresponding construct in the current `/3` project model. The shared
`ElementId`, exact scalar, vocabulary contract, nominal type/value and
fingerprint primitives remain covered independently of those retired complete
models.

`neutral-test-support` retains temporary-directory ownership tests. Its old
`empty_composition_ir` builder was removed with the `/2` model; current fixtures
remain with their owning producer, reader, codec or cross-crate tests.
