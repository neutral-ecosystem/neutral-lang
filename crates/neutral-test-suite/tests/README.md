<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cross-package test modules

The non-published `neutral-test-suite` crate owns cross-package integration,
system, conformance, property, security, and deterministic hardening tests. Its
test modules are wired from `src/lib.rs` and remain filterable by module name.

| Module | Current coverage |
| --- | --- |
| `source_pipeline/` | Current source capture, compilation, cache recovery, and CLI behavior. |
| `expression_pipeline/` | Complete `/3` reader, identity, consumer, projection, and transport behavior. |
| `composition_contract/` | Raw composition/vocabulary catalogue capture and reader contracts, plus immutable historical identity vectors. |
| `hardening/` | Fixed-seed source, formatter, vocabulary, `/3` decoder/probe campaigns and bounded metamorphic checks. |
| `suite/` | Current wire round trips and hostile decoder rejection tests. |

Run the focused current pipeline checks with
`cargo test -p neutral-test-suite expression_pipeline` or
`cargo test -p neutral-test-suite source_pipeline`. Run
`cargo test -p neutral-test-suite` for all cross-package tests, or
`cargo xtask test all` for the complete repository suite including doctests.
