# Neutral v0 development progress

Status: Stage 2 fixture/oracle activation in progress.

## Current focus

- Stage: Stage 2, Step 1
- Status: contract freeze approved; Stage 2 Step 2 complete
- Last updated: 2026-08-26

## Next actions

- [ ] Continue with Stage 2, Step 3: minimal frontend slice.

## Blockers

None recorded.

## Completed log

- [*] 2026-08-26: Configured continuous integration for every push to `main`
  and release qualification for every pushed tag; both workflows retain manual
  dispatch.
- [*] 2026-08-26: Upgraded workflow repository checkout steps from v4 to
  `actions/checkout@v6` for the current credential-handling implementation.
- [*] 2026-08-26: Renamed the main-branch workflow from `stage1.yml` to
  `ci.yml`; it remains the continuous-integration workflow.

- [*] 2026-08-26: Moved automation names and output-category prefixes into the
  dedicated `xtask/src/constants.rs` module, including `[info]`, `[error]`,
  `[warn]`, and `[manifest]` linkage.
- [*] 2026-08-26: Removed duplicated CLI/probe package-name output literals by
  deriving names from Cargo package metadata while retaining category prefixes.

- [*] 2026-08-26: Centralized workspace package and tool command names in the
  `xtask` constants namespace, replacing repeated command literals (including
  `NEUTRAL_COMPILER`), and linked host bootstrap scripts to safe
  `NEUTRAL_CARGO_COMMAND`/`NEUTRAL_RUSTC_COMMAND` overrides.

- [*] 2026-08-26: Completed Stage 2, Step 2. Added typed exact SHA-256 source
  identity, checked spans and line/column derivation, deterministic diagnostics
  and limits, cancellation/result classes, immutable capture, and the I/O-free
  compilation boundary. The SHA-256 dependency and its transitive closure are
  explicitly reviewed by automation policy.
- [*] 2026-08-26: Approved the v0 contract freeze with the repository owner,
  promoted the governing specifications and author guide, assigned the `0.1.0`
  contract family, and completed Stage 2, Step 1 with three frozen source cases
  and complete per-case oracles.
- [*] 2026-08-26: Classified all known contract-freeze questions in a blocking
  ledger and linked it from the freeze manifest and development entry point.
  Stage 2 remains blocked until every blocking entry is accepted and closed.
- [*] 2026-08-26: Added a responsibility and ecosystem README to every
  workspace package, including the non-production `xtask` package.
- [*] 2026-08-26: Added a review-candidate fixture/oracle registry for all 14
  current source fixtures. It locks source SHA-256 values and required oracle
  shapes while explicitly recording that no oracle is yet approved or immutable.
- [*] 2026-08-26: Organized generated evidence beneath `test-results/` by
  bootstrap, CI profile/stage, suite, and analysis category; CI runs now write
  to `test-results/ci/<profile>/run-<process-id>-<sequence>/`.
- [*] 2026-08-26: Started the mandatory contract-freeze gate with a draft
  manifest that hashes each governing source and records the unresolved approval,
  versioning, fixture/oracle, and review blockers. It does not authorize Stage 2.
- [*] 2026-08-26: Corrected the dev-container Apache-2.0 header to a JSONC
  comment so it is not interpreted as an unsupported configuration property.
- [*] 2026-08-26: Removed the time-based nightly workflow schedule; the nightly
  profile now runs only on pushes to `main` or manual dispatch.
- [*] 2026-08-26: Moved host bootstrap scripts to `scripts/linux/` and
  `scripts/win/`.
- [*] 2026-08-26: Stage 1, Step 1 completed. Created the 11-package virtual Rust
  workspace with explicit ownership, non-published support packages, pinned
  toolchain and quality configuration, documented behavior-free shells, and a
  committed lockfile. `cargo metadata`, formatting, workspace check, strict
  Clippy, tests, and documentation passed.
- [*] 2026-08-26: Stage 1, Step 2 completed. Added `cargo xtask boundary check`
  to enforce direct package dependencies, the pure compiler closure, and the
  standalone probe allowlist. Negative tests prove forbidden compiler and probe
  edges are rejected; the workspace audit remains green.
- [*] 2026-08-26: Added a workspace-enforced Rust documentation rule for every
  function, including private helpers and test functions; documented all current
  function definitions.
- [*] 2026-08-26: Stage 1, Step 3 and Stage 1 validation completed. Added
  bootstrap scripts, the pinned development container, active-suite and planned
  conformance configuration, workflow shells, and the full `cargo xtask`
  automation interface. `cargo xtask ci stage1` passed locally and in the
  network-disabled non-root development container.
- [*] 2026-08-26: Standardized current CLI, probe, bootstrap, and automation
  output as `[category] message`, including `[info]`, `[error]`, and
  `[manifest]` payloads.

## Working rule

Keep this file small. Update it when the active step, blocker, or completed
validation changes. Completion requires the validation evidence named by the
relevant stage or slice in `IMPLEMENTATION-STAGES.md`.
