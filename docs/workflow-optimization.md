<!-- SPDX-License-Identifier: Apache-2.0 -->

# Workflow Optimization Guide

This document outlines the workflow optimizations implemented to reduce cognitive friction, eliminate manual bookkeeping, and establish an ergonomic, 100% native Rust developer experience in `neutral-lang`.

---

## 1. Background & Audit: The Human DX Problem

In earlier iterations of this codebase, day-to-day development felt slow, redundant, and excessively bureaucratic:

1. **The Recursive Fixture Hash Burden**:
   Modifying or adding a test fixture in `portable/specs/fixtures/` required:
   - Calculating the SHA-256 digest of the `.neu` or `.toml` fixture.
   - Calculating the SHA-256 digest of the corresponding oracle file.
   - Editing `portable/conformance/manifest.toml` to insert both digests.
   - Calculating the SHA-256 digest of `manifest.toml` itself.
   - Editing `portable/specs/contracts/freeze.toml` to update `manifest_sha256`.
   - Recalculating review and contract hashes if any related files were touched.
   A single mismatched hex character caused `cargo xtask check` or `cargo xtask dev` to fail closed, blocking local progress with cryptic mismatch errors.

2. **Fractured, Non-Native Commands**:
   Developers were expected to run platform-specific shell scripts (`./scripts/linux/bootstrap.sh` or `.\scripts\win\bootstrap.ps1`) that merely wrapped Cargo commands. This added unnecessary mental overhead, bypassed standard Rust developer muscle memory, and created platform friction.

3. **Monolithic & Redundant Validation**:
   The primary development loop (`cargo xtask dev`) conflated everyday compiler hacking with deep CI release gating: running multiple full Cargo passes in series, rebuilding API documentation on every run, re-filtering test targets, and verifying non-code repository governance.

---

## 2. Solution 1: Automated Fixture Synchronization (`cargo sync-fixtures`)

To completely eliminate the fixture bookkeeping burden, we introduced a native Rust fixture synchronization engine located in [`xtask/src/fixtures.rs`](../xtask/src/fixtures.rs).

### How It Works
When you create or update a test fixture or oracle:
```sh
cargo sync-fixtures
```
This single native command:
- Reads `portable/conformance/manifest.toml` and locates every registered `fixture` and `oracle`.
- Recalculates their exact SHA-256 digests.
- Updates `fixture_sha256` and `oracle_sha256` in-place inside `manifest.toml`.
- Re-hashes `portable/conformance/manifest.toml` and automatically updates `manifest_sha256` in `portable/specs/contracts/freeze.toml`.
- Re-hashes any updated contract files declared in `freeze.toml` (such as `fixture-oracle-review.toml` and `CAPTURE-REQUEST.md`).
- Scans `portable/specs/fixtures/` on disk and warns if any fixture file is missing from the manifest.

### Check / Dry-Run Mode
To verify fixture and contract digests without modifying files (ideal for CI):
```sh
cargo check-fixtures
# or: cargo xtask fixtures check
```

---

## 3. Solution 2: Eliminating Non-Native Commands (100% Native Cargo)

We have "killed" developer reliance on shell scripts (`scripts/linux/*.sh` and `scripts/win/*.ps1`). The repository now natively uses Cargo aliases configured in [`.cargo/config.toml`](../.cargo/config.toml):

| Old Non-Native Command | Modern Native Cargo Command | Purpose |
| ---------------------- | --------------------------- | ------- |
| `./scripts/linux/bootstrap.sh` | `cargo bootstrap` | Verify local Rust toolchain & initialize environment |
| `cargo xtask dev` | `cargo dev` | Full local formatting, linting, tests, and smoke passes |
| *(Manual SHA-256 calculations)* | `cargo sync-fixtures` | Automatically synchronize fixture & freeze digests |
| *(Manual verification)* | `cargo check-fixtures` | Verify fixture digests without modifying files |
| `cargo xtask check` | `cargo check-repo` | Run repository structure, traceability, and boundary checks |
| `cargo xtask quality` | `cargo quality` | Run complete release quality gate composition |

> **Note**: The legacy scripts in `scripts/` remain as thin, backward-compatible adapters for automated host diagnostics, but human contributors should exclusively use standard `cargo` commands.

---

## 4. The Human-Centric 3-Tier Workflow

Developers should not be subjected to full release-gate auditing while iterating on code. The recommended day-to-day workflow is structured into three tiers:

```
┌────────────────────────────────────────────────────────┐
│ Tier 1: Fast Inner Loop (< 1s)                         │
│ • cargo check                                          │
│ • cargo test -p <crate_name>                           │
│ Fast compiler diagnostics and targeted crate tests.    │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│ Tier 2: Pre-Commit Iteration (< 5s)                    │
│ • cargo sync-fixtures (if fixtures/contracts changed)  │
│ • cargo dev                                            │
│ Automated hash sync, workspace formatting, unit tests. │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│ Tier 3: Quality & CI Gate                              │
│ • cargo quality                                        │
│ • cargo xtask ci pr                                    │
│ Full coverage, fuzz regressions, and audit gates.      │
└────────────────────────────────────────────────────────┘
```

---

## 5. Principles for Ongoing Human Maintainability

1. **Tools Serve Humans, Not the Reverse**:
   If an administrative requirement (like SHA-256 verification or manifest tracking) can be automated by software, automate it. Developers should focus on syntax, semantics, IR, and compilation logic.

2. **Standard Rust Ergonomics First**:
   Always prefer native `cargo <command>` idioms over external bash/PowerShell scripts or custom wrapper executables.

3. **Fail-Helpful, Not Fail-Closed**:
   When checks fail, output actionable remedies (e.g. *"Run `cargo sync-fixtures` to resolve digest mismatches"*) rather than generic failure notices.
