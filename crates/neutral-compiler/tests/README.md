<!-- SPDX-License-Identifier: Apache-2.0 -->

# Compiler test ownership

Test bodies live here and are registered through owning source modules. A folder
under `tests/` is not automatically a Cargo test binary; only top-level Rust files
are discovered automatically.

| Active path | Responsibility |
| --- | --- |
| `frontend/` | Raw lexer, source spans and header/import token scanning |
| `expression_source/` | Complete source parsing, semantics, expansion, formatting, cache, capture and fault recovery |
| `graph/` | Import/SCC work, size boundaries and cancellation |
| `nominal/` | Nominal syntax, defaults/types and captured companion facts |
| `support/` | Shared private captured-source fixture, without a separate test binary |
| `profiles.rs` | Public profile and result contracts |
| `project_capture_contract.rs` | Public host byte-envelope builder and replay contracts |

The retired `parser/mod.rs` was not registered and referred to the removed parser.
Its coverage has been mapped to current owners before deletion:

| Retired parser check | Current owner |
| --- | --- |
| Truncated headers/declarations | `expression_source/capture.rs`, `grammar_boundaries.rs` |
| Invalid header/field tokens | `expression_source/capture.rs`, `boundaries.rs` |
| Truncated recursive operands | `expression_source/parser.rs`, `grammar_boundaries.rs` |
| Truncated template/matrix declarations | `grammar_boundaries.rs::incomplete_template_and_matrix_declarations_reject_safely` |
| Empty named primaries | `grammar_boundaries.rs::empty_names_and_missing_assignment_reject_with_syntax_diagnostics` |
| Missing assignment delimiter | Same grammar-boundary check, with valid input-declaration control |
| Truncated production prefixes | `expression_source/parser.rs::recursive_construction_token_prefixes_never_publish_partial_values` checks each token boundary in the listed record/value constructions; raw group/name tests cover their remaining helper boundaries |
| Wrong operand-token categories | `expression_source/parser.rs`, `boundaries.rs::contextual_parser_rejects_malformed_source` |
| Exact keyword spellings | `expression_source/parser.rs::contextual_keywords_keep_exact_original_spellings` |
| Immediate record field limit | `grammar_boundaries.rs::record_fields_accept_exact_limit_and_reject_one_over` |

Historical profile examples under `fixtures/profiles/` retain their original
bytes, but presence there does not register a test. Use `cargo xtask test all`
for complete discovery and `cargo test -p neutral-compiler` for owning tests.
