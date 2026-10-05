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

The vocabulary target exercises both the released bundle decoder and the v1
project-bundle decoder. The campaign command supplies tracked v1 seeds from
`fuzz/seeds/vocabulary/` alongside the ignored mutable corpus. Run
`cargo xtask fuzz smoke` for deterministic mutations, or select a nightly
toolchain for `cargo xtask fuzz campaign` to run the configured per-target
budget. An untraced runner is needed for LeakSanitizer; in a ptraced sandbox,
set `LSAN_OPTIONS=detect_leaks=0` for the address-sanitized campaign and record
that leak checking was unavailable there. If a system `ccache` wrapper points
at a read-only cache, set `CCACHE_DISABLE=1` for that run.
