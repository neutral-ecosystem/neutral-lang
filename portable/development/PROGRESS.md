# Neutral v0 development progress

Status: implementation foundation in progress.

## Current focus

- Stage: Mandatory contract-freeze gate
- Status: Stage 1 complete; contract-freeze review in progress, not approved
- Last updated: 2026-08-26

## Next actions

- [ ] Resolve the draft freeze manifest blockers and complete the mandatory
      contract-freeze gate before compiler behavior.

## Blockers

None recorded.

## Completed log

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
