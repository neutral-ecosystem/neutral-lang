<!-- SPDX-License-Identifier: Apache-2.0 -->

# File-backed language pipeline

Real current-profile `.neu` inputs exercise capture, imports, parsing, semantics,
complete expression-project IR, `/3` encoding, independent reader validation,
source spans, deterministic ordering, and syntax-cache reuse/recovery. Semantic
assertions come from the compiler and reader contracts; wire round trips are not
used as semantic oracles. Fixtures remain mutable regression coverage, separate
from the frozen conformance corpus.

The accepted corpus has literal expectations for scalar/record/list/default
values, Unicode, inert locations, vocabulary aliases and canonical reference
targets. `negative.rs` registers each rejected fixture separately, requiring
capture success and the exact compilation diagnostic code. Reported locations
must belong to captured bytes and stay in bounds; the cross-module private
reference case additionally pins the exact consuming declaration span.

`cross_module.rs` checks valid import SCCs independently of rejected value cycles,
diamond imports, public closure, private target/type rejection and alias/order
invariance. It checks independently decoded values and owners, not just equality
with a producer that could contain the same mistake. The reference-privacy
fixtures use public nominal types to isolate private target rejection from an
unrelated invalid reference-type error.

Use `cargo xtask test all` for the complete repository, or `cargo xtask test
integration` for registered integration cases. For this entire module only, run
`cargo test --package neutral-test-suite source_pipeline::`. The executable
case builds the reader-only probe through Cargo's JSON artifact discovery
and inspects artifacts produced from these sources in an isolated temporary root.
No language runtime exists: execution here means the probe executable.

`system.rs` also runs actual CLI validation, formatting and compilation through
the current profile. Failed compilation cannot replace a previously valid
artifact. Detailed expression, bounds, evaluator, hostile decoder, and consumer
coverage remains in the adjacent `expression_pipeline`, `transport` and
owning compiler/reader tests. See the [test inventory](../README.md) for the
separation between executable modules and historical records.
