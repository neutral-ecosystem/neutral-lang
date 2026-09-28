<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 3.4 graph validation

Status: validation gate complete; Stage 3.3 integration recorded separately

Date: 2026-09-28  
Target gate: `v0.3.4`

The cross-package suite executes all 13 pinned Stage 3 captured-project
fixtures against the three reviewed outcome-oracle tables. It checks accepted
module, edge, and dependency-first SCC order; rejected diagnostic codes and
applicable module/target/alias fields; configured limit values; and absence of
a published graph after rejection. Fixture and oracle test assets are exact
copies of the active portable files (verified byte-for-byte) so tests continue
to run after `portable/` is archived.

Each case is rebuilt after every possible cyclic source-order rotation followed
by reversal. Eight scoped workers then run 16 graph builds each against the
reordered captured projects and compare complete graph/failure results with
the baseline. Dedicated tests cover exact/one-over independent graph limits,
four-diagnostic canonical ordering, diagnostic overflow as explicit
`NEU-MOD-007`, failure recovery, and duplicate-target/alias ambiguity
precedence. A separate exclusion audit rejects wildcard, relative, path, URL,
re-export, implicit-alias, and absent-target forms without acquisition; a
second module header is rejected at the inherited capture boundary.

## Verification

```text
cargo test -p neutral-test-suite --lib stage3::
cargo clippy -p neutral-test-suite --all-targets -- -D warnings
cargo xtask fixtures check
cargo xtask portable verify
cargo xtask ci pr
git diff --check
```

The composed PR workflow passed at `test-results/workflows/ci/pr/run-2-6`.
The fixture hash check reported zero manifest or freeze updates. The frozen
module graph contract remains byte-identical to its registered digest; its
historical status line is intentionally not edited to track implementation.

Stage 3.3 public graph access, typed source locations, and edited/clean
equivalence are recorded in [Stage 3.3 evidence](stage3-3-public-integration.md).
