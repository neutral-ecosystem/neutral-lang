<!-- SPDX-License-Identifier: Apache-2.0 -->

# Coverage-guided fuzzing

This isolated `cargo-fuzz` package owns coverage-guided source, vocabulary,
external IR, formatter, and probe campaigns. It is deliberately outside the
stable production workspace: fuzzing may use a temporary nightly toolchain,
while all release code continues to build with the repository’s selected
stable compiler.

Confirmed failures must be minimized and promoted into deterministic fixtures
or regression tests before they are considered resolved.

## What belongs in Git

Keep `Cargo.toml`, `Cargo.lock`, harness source, responsibility READMEs, and
reviewed seeds under `seeds/`. The isolated lockfile is intentional dependency
input, not build output. Do not commit target binaries, mutable corpus entries
(including loose files directly under `corpus/`), coverage output, or raw crash,
timeout, out-of-memory, and leak findings. Keep local findings until they are
triaged and minimized; promote real defects to named deterministic tests.
`cargo xtask check` rejects ignored files that were force-added to Git.

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

All targets exercise current boundaries: the vocabulary target checks captured
bundle schemas; the other targets drive complete expression projects. Shared
`expression_support` registers the [runtime-owned seeds](seeds/README.md), bounds
source capture and generates valid complete frames for deep wire mutations.
Source compilation must independently validate and round-trip. Successful wire
decoding is followed by bounded public-view/guarded-dependency inspection and
explicit consumer binding requests; the probe selects a real public nominal
contract rather than mistaking a binding name for a type. Consumer outcomes cover
selected/skipped outputs, supplied/missing/unavailable/failed states, malformed
roots, duplicate/unknown keys and subsequent-request recovery. No target adds an
executor or implicit input lookup.

The mutation scope in `config/quality-gates.toml` includes current shared typing,
guarded evaluation, consumer binding policy, restrictions, decoding, dependency
accounting and expression logical-identity framing. Registration is not a passing
campaign: changed harnesses, tests or scope require new source-bound receipts.
Transitive graph coverage and allocator-fault injection remain the responsibility
of the deterministic tests and separate review.

The campaign command supplies tracked seeds from `fuzz/seeds/vocabulary/`
alongside its ignored mutable corpus; expression sources are registered directly
in the shared harness. Run `cargo xtask fuzz smoke` for deterministic coverage of
these harness paths. Select nightly for `cargo xtask fuzz campaign` to run the
configured per-target budget. An untraced runner is needed for LeakSanitizer; a
ptraced sandbox's leak-checker shutdown failure is an environment failure, not
passing campaign evidence. If a system `ccache` wrapper points at a read-only
cache, set `CCACHE_DISABLE=1` for that run.
