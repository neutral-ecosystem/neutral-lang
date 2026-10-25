<!-- SPDX-License-Identifier: Apache-2.0 -->

# Reader test coverage

The former `/1` project authority and `/2` complete-composition authority tests
were retired with those reader APIs. Current `/3` expression authority and raw
composition catalogue coverage remain registered:

| Retired reader assertion | Current named regression |
| --- | --- |
| Independent composition contract/value checks | `tests/composition_scope.rs::composition_scope_reader_closed_value_validation`; `tests/composition.rs::reader_composition_materializes_public_closed_values_independently` |
| Exact schemas and safe reader error classes | `tests/expression/public_projection_coverage.rs::complete_reader_errors_keep_safe_classes_and_frozen_result_schema` |
| Producer/resource bounds, cancellation and atomic view allocation | `tests/expression/consumer_coverage.rs::public_view_allocation_failures_preserve_the_complete_project`; `tests/expression/selection.rs::selection_and_view_limits_are_positive_inclusive_and_shared`; `tests/composition_scope.rs::composition_scope_reader_bounds_and_cancellation` |
| Current module/import closure and selected dependency projection | `tests/expression/consumer_coverage.rs::entry_discovery_selects_exact_public_nominals_and_closes_the_view`; `tests/expression/dependency_regions.rs`; `tests/composition_scope.rs::composition_scope_reader_public_source_contracts` |
| Canonical vocabulary contracts and catalogue ordering | `tests/composition.rs::reader_composition_public_catalogue_preserves_contract_facts`; `tests/composition_scope.rs::composition_scope_reader_public_source_contracts` |
| Public-only entries, edges, origins and private-root redaction | `tests/expression/consumer_coverage.rs::empty_view_is_empty_and_private_roots_never_escape`; `tests/expression/selection.rs::public_reuse_projection_redacts_retained_private_evidence`; `tests/composition_bindings.rs::composition_binding_reader_public_edges_and_contracts`; `tests/composition_scope.rs::composition_scope_reader_private_roots_are_redacted` |
| Former `/2` negotiated consumer authority and reports | `tests/expression/boundary_coverage.rs::consumer_policy_rejects_missing_features_reserved_codes_and_private_contracts`; `tests/expression/boundary_coverage.rs::consumer_reports_guarded_input_demand_and_complete_values`; `tests/expression/binding_reports.rs` |
| Former `/2` entries, selection and closure | `tests/expression/consumer_coverage.rs::entry_discovery_selects_exact_public_nominals_and_closes_the_view`; `tests/expression/consumer_coverage.rs::entry_cardinality_distinguishes_empty_unique_and_ambiguous_matches`; `tests/expression/selection.rs::explicit_entry_order_and_private_rejections_preserve_accepted_selection` |
| Former `/2` allocation preflight | `tests/expression/consumer_coverage.rs::public_view_allocation_failures_preserve_the_complete_project`; current projection limits remain covered by `tests/expression/selection.rs::selection_and_view_limits_are_positive_inclusive_and_shared` |

`profile_contract.rs` and current composition/expression reader coverage stay
registered and runnable with `cargo test -p neutral-reader`.
