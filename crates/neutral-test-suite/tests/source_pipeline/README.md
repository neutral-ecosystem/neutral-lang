<!-- SPDX-License-Identifier: Apache-2.0 -->

# File-backed language pipeline

Real `.neu` inputs exercise capture, imports/SCCs, parsing, semantics, complete
IR, encoding, independent reader validation, public views, identities, and probe
JSON. Expected values and rejection phases are asserted by the Rust runner, not
inferred from round trips. Fixtures are regression coverage, not replacements for
the frozen conformance corpus.

Run `cargo test --package neutral-test-suite source_pipeline::`. The executable
case builds the reader-only probe offline through Cargo's JSON artifact discovery
and inspects artifacts produced from these sources in an isolated temporary root.
No language runtime exists: execution here means the probe executable.

`system.rs` also runs actual CLI validation, formatting and compilation for the
available source profile, and verifies project-file capture without activating an
unavailable compilation profile. Failed compilation cannot replace a previously
valid artifact. Root, nested, list and nullable record-reuse regressions require
complete inherited field provenance and reject missing, duplicate or forged paths.
