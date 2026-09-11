<!-- SPDX-License-Identifier: Apache-2.0 -->

# Coverage-guided fuzzing

This isolated `cargo-fuzz` package owns coverage-guided source, vocabulary,
external IR, formatter, and probe campaigns. It is deliberately outside the
stable production workspace: fuzzing may use a temporary nightly toolchain,
while all release code continues to build with the repository’s selected
stable compiler.

Confirmed failures must be minimized and promoted into deterministic fixtures
or regression tests before they are considered resolved.

## Subsystem ownership

| Target | Owning boundary |
| --- | --- |
| `source` | `neutral-compiler` source capture and frontend |
| `vocabulary` | `neutral-vocabulary` strict bundle decoding |
| `ir` | `neutral-encoding` external artifact decoding |
| `formatter` | `neutral-compiler` reference formatting |
| `probe` | `neutral-probe` reader-only traversal |

Harnesses and seed documentation are tracked. Mutable state under
`fuzz/corpus/` and failures under `fuzz/artifacts/` are ignored and remain
separate from the immutable released fixtures under `conformance/`. A confirmed
finding is minimized first, then retained in the owning crate's deterministic
regression suite when it represents a real defect.
