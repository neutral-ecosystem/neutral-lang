<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 1 profile-foundation validation

Status: validated; `v0.2.0` promotion not performed

Date: 2026-09-20  
Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`  
Package release during validation: `0.1.0`

## Contract and fixture gate

- The exact `0.1` and `1.0` profile spellings, unavailable-profile behavior,
  migration rule, exclusions, shared default limits, and `NEU-PRO` diagnostic
  family are frozen in the source contract and shared core profile module.
- The active manifest registers positive, negative, boundary, lookalike, and
  exclusion cases. The freeze manifest pins the reviewed manifest and
  fixture/oracle-review file by SHA-256.
- Executable compiler fixtures are byte-identical copies owned by the compiler
  test package. This preserves the rule that the repository continues to work
  when the active portable planning tree is archived or replaced.

## Implementation and public boundary

- `neutral-core` owns profile selection, availability, capabilities, diagnostic
  family, and shared default-limit constants.
- The compiler dispatches exact recognized profiles before interpreting source.
  `0.1` retains its frozen behavior, exact `1.0` fails with `NEU-PRO-001`, and
  lookalikes retain `NEU-SYN-002`.
- `neutral-reader` exposes the shared catalogue without depending on the
  compiler. `neutral-cli profiles` renders that reader-visible catalogue using
  the stable `[info]` output convention.
- The dependency-boundary audit passed. No public API exposes compiler-private
  lexer, parser, syntax, or semantic model types.

## Migration and exclusions

Existing `neu "0.1"` source requires no migration and is never reinterpreted.
The `1.0` header is discoverable but unavailable; functions, effects,
acquisition, and product semantics all fail at the profile gate without partial
IR. Later stages may activate only their explicitly frozen v1 capabilities.

## Retained validation

The following commands passed with no skipped required Stage 1 case:

```text
cargo xtask portable verify
cargo xtask check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo xtask test conformance
cargo xtask ci pr
cargo run --quiet --package neutral-cli -- profiles
```

The final CI composition passed at
`test-results/workflows/ci/pr/run-2-8`. It covered the inherited v0.1 suite,
Stage 1 profile fixtures, structural limits, dependency policy, documentation,
fuzz regressions, repeated execution, and concurrent dispatch.

## Promotion decision

Stage 1 implementation is validated. Package version changes, release
qualification, and publication of `v0.2.0` are deliberately deferred; they
require a separate owner-approved release action.
