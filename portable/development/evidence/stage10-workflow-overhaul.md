<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 stable workflow overhaul

Review date: 2026-09-08. Status: Step 2 implementation complete; final candidate
retagging and later release qualification remain separate Stage 10 work.

## Stable command boundary

`xtask/src/interface.rs` is the single command grammar. It exposes `fmt`,
`lint`, `check`, `test`, `coverage`, `fuzz`, `quality`, `build`, `validate`,
`package`, `release`, `version`, `portable`, and `clean` without deriving names
from an implementation stage. Parser tests reject the removed `format`,
`clean-results`, `ci stage1`, and `ci nightly` shapes.

Test minimums used by normal commands now come from `[current.minimum]`, while
historical stage sections remain evidence. Advancing or removing a planning
stage therefore cannot rename commands or deactivate the implemented suites.

## Composition and ownership

- `fmt`, `lint`, and `check` remain independently runnable. `check` composes a
  locked Cargo check, dependency boundaries, crate-local test layout,
  requirement/fixture traceability, and workflow-document synchronization.
- `quality --profile pr` composes formatting, linting, checking, all ordinary
  tests, binary smoke, deterministic fuzz regressions, and the standalone-probe
  build. The release profile additionally verifies retained expensive-gate
  statuses, builds release artifacts, and validates the released binaries.
- `validate binaries` exercises release CLI/probe entry points and rechecks the
  compiler-free probe boundary. `validate <artifact>` uses the built standalone
  release probe.
- `package` reads the typed explicit release selection, verifies approval,
  evidence, annotated tag target, exact candidate checkout, and clean tree,
  then stages only selected binaries plus `LICENSE` and `README.md` beneath the
  ignored release result root.
- `release prepare` composes release quality, documentation, and package
  assembly. It cannot tag, push, upload, publish, or change frozen contracts.

`config/release.toml` records the selected source-tag plus GitHub-binary scope;
crates.io is false. Its constrained parser rejects missing approval, malformed
full commit IDs, empty distribution scope, unsafe binary names, and empty binary
selection for GitHub assets.

## Platform and CI adapters

The required `scripts/linux/` and `scripts/win/` trees each contain bootstrap,
environment, and release adapters plus a README specifying owner, host, inputs,
outputs, and delegated command. Scripts perform only host checks and direct
`cargo xtask` invocation.

`.github/workflows/ci.yml` invokes `quality --profile pr` and `docs` after
checkout/toolchain setup. `.github/workflows/release.yml` invokes only `release
prepare`. An executable workflow-contract check rejects drift back to duplicated
YAML policy or internal `ci` aliases.

## Executed evidence

- `cargo fmt --all`
- `cargo clippy -p xtask --all-targets --all-features -- -D warnings`
- `cargo test -p xtask --all-targets` — 22 passed
- `cargo xtask check` — dependency, test layout, traceability, and workflow
  contracts passed
- `cargo xtask quality --profile pr` — full ordinary suite and composition
  passed
- `scripts/linux/bootstrap.sh` and `scripts/linux/environment.sh manifest` —
  passed on `x86_64-unknown-linux-gnu` with Rust/Cargo 1.98.1
- `cargo xtask version show`, `version check`, and `portable verify` — passed
- `cargo xtask build --profile release` and `validate binaries` — passed
- `cargo xtask package` — correctly failed closed because the current workflow
  revision is newer than candidate `v0.1.0-rc.1`; it did not assemble or publish
  artifacts from the wrong revision

Generated quality and bootstrap evidence is retained beneath ignored
`test-results/`. Release package assembly remains intentionally blocked until a
new clean reviewed candidate includes this workflow implementation.
