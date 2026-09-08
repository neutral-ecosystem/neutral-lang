<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 repository and quality overhaul evidence

Evaluation date: 2026-09-08. Owner: maintainer. Scope: Stage 10, Step 4.

## Repository ownership and hygiene

`config/repository-layout.toml` is the executable inventory for every retained
tracked top-level directory. Each entry identifies its README, owner, and
lifecycle. `cargo xtask check` verifies the exact inventory, each workspace
package README, both platform-script READMEs, generated-output ownership, Git
tracking hygiene, crate test layout, portable boundaries, fixture traceability,
and the thin workflow contract.

No generated result, archived portable tree, duplicate experiment, or mutable
fuzz state is tracked. Existing ignored `target/`, `test-results/`, mutation,
corpus, and fuzz-artifact data was preserved; Step 4 performed no automated
deletion. Frozen fixtures and evidence were not moved.

## Durable test levels

`config/test-levels.toml` assigns each durable test purpose to one stable
command and owner. The following commands passed independently:

- `cargo xtask test unit`
- `cargo xtask test smoke`
- `cargo xtask test integration`
- `cargo xtask test system`
- `cargo xtask test conformance`
- `cargo xtask test property`
- `cargo xtask test security`
- `cargo xtask fuzz smoke`
- `cargo xtask test performance --profile pr`
- `cargo xtask test all`

Filtered levels search crate-owned library, binary, and integration-test
targets across the workspace. Production sources contain only path-based test
module declarations; test bodies remain in crate `tests/` directories. The
portable manifest remains the immutable positive/negative feature fixture and
oracle owner.

## Coverage, fuzzing, and warnings

`RUSTUP_TOOLCHAIN=nightly cargo xtask coverage` passed and generated:

- HTML: `test-results/analysis/coverage/html/index.html`
- JSON: `test-results/analysis/coverage/coverage.json`

Measured production coverage is 93.86% lines, 93.66% functions, and 85.16%
regions against unchanged 85%/90%/80% gates. The only exclusion is the
repository-automation `xtask`, which has separate unit and command-level
validation. Its exact regex and rationale are reviewed, declared, and checked
in `config/quality-gates.toml`; callers cannot add an undocumented filter.

All five coverage-guided fuzz harnesses have explicit subsystem owners.
Mutable corpora and findings remain ignored and separate from normative
fixtures; confirmed defects must be minimized and promoted to deterministic
owner-crate regressions.

Current nightly Cargo inferred every conventional package README without the
redundant manifest declarations that emitted `manual_readme` warnings.
`cargo xtask quality --profile release` and `cargo xtask docs` passed with
warnings denied across workspace targets and Rustdoc. Offline Cargo metadata is
also checked for release-relevant warnings.

## Result

Stage 10 Step 4 passes. A clean clone can discover ownership and lifecycle from
the root README and policy inventory, invoke each test level independently,
reproduce the release-quality composition locally, and locate every ignored
generated evidence class without consulting historical stage notes.
