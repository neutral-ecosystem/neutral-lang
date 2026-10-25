<!-- SPDX-License-Identifier: Apache-2.0 -->

# File-backed language pipeline

Real current-profile `.neu` inputs exercise capture, imports, parsing, semantics,
complete expression-project IR, `/3` encoding, independent reader validation,
source spans, deterministic ordering, and syntax-cache reuse/recovery. Semantic
assertions come from the compiler and reader contracts; wire round trips are not
used as semantic oracles. Fixtures remain mutable regression coverage, separate
from the frozen conformance corpus.

Run `cargo test --package neutral-test-suite source_pipeline::`. The executable
case builds the reader-only probe offline through Cargo's JSON artifact discovery
and inspects artifacts produced from these sources in an isolated temporary root.
No language runtime exists: execution here means the probe executable.

`system.rs` also runs actual CLI validation, formatting and compilation through
the current profile. Failed compilation cannot replace a previously valid
artifact. Detailed expression, bounds, evaluator, hostile decoder, and consumer
coverage remains in the adjacent `expression_pipeline`, `transport` and
owning compiler/reader tests. See the [test inventory](../README.md) for the
separation between executable modules and historical records.
