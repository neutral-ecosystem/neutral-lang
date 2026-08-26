# Neutral v0 development environment and automation

Status: required implementation operations contract

This document owns repository tooling, reproducible environments, task
automation, CI profiles, and generated evidence locations. It must not influence
Neutral language semantics.

## Environment layers

Bootstrap is deliberately split to avoid requiring `cargo xtask` before Rust
and Cargo exist.

### Layer 0: host bootstrap

Thin `scripts/bootstrap.sh` and `scripts/bootstrap.ps1` scripts:

1. detect supported host/architecture;
2. verify required shell/PowerShell, TLS, certificate, archive, and checksum
   facilities;
3. verify or install the exact approved Rust toolchain using an explicit user
   action;
4. verify download checksums/signatures when supplied by the publisher;
5. refuse unsupported/unpinned versions unless an explicit development override
   is recorded; and
6. invoke `cargo xtask bootstrap` only after Cargo is available.

The scripts contain no compiler/test policy beyond reaching the Rust task
runner. They never use `curl | sh`, elevate privilege silently, write outside
documented tool/cache roots, or modify shell startup files without consent.

### Layer 1: workspace bootstrap

`cargo xtask bootstrap` verifies or installs approved Cargo tools, configures
local ignored result/cache directories, and emits an environment manifest. It
does not install Rust itself.

### Layer 2: development container

`.devcontainer/Dockerfile` and `devcontainer.json` provide an optional pinned
environment:

- base image pinned by immutable digest;
- package versions or snapshot repository identified;
- non-root user;
- no host Docker socket, credentials, SSH agent, or secrets by default;
- network unavailable during test execution by default;
- writable repository plus explicit cache/target/result volumes only; and
- same `cargo xtask` entry points as host/CI.

The container is a convenience and reproducibility aid, not the sole supported
environment or semantic authority.

## Version and host policy

Record separately:

- exact pinned development/CI Rust toolchain;
- minimum supported Rust version (MSRV);
- release toolchain;
- supported/tested/experimental/unsupported host triples;
- primary system-test host;
- finite host/filesystem matrix;
- benchmark runner identity and operating policy; and
- approved versions of formatter, linter, coverage, fuzz, mutation, benchmark,
  dependency, link, and documentation tools.

CI runs the pinned toolchain and MSRV as separate jobs. “Stable” without an exact
version/date is not a reproducible pin.

## Environment variables

`.env.example` documents optional harness variables only:

```text
NEUTRAL_TEST_PROFILE=pr
NEUTRAL_TEST_RESULTS=test-results
NEUTRAL_TEST_SEED=...
NEUTRAL_TEST_RETAIN_FAILED_ROOTS=false
NEUTRAL_BENCH_BASELINE=...
NEUTRAL_LOG=...
```

Rules:

- No variable changes lexing, layout, parsing, semantics, logical IR,
  fingerprinting, diagnostic ordering, or reader validity.
- Project variables use the `NEUTRAL_` prefix.
- Tests never repurpose `HOME` or other standard host variables.
- Secrets never appear in `.env.example`, environment manifests, diagnostics,
  logs, snapshots, or test artifacts.
- CLI rendering tests pin locale/timezone, while semantic tests vary them to
  prove independence.
- Every concurrent job gets unique temporary, target, cache, and result roots.

## Dependency acquisition and offline execution

- Commit `Cargo.lock` because the workspace ships binaries and test tooling.
- Pin Git dependencies to immutable revisions; prefer registry releases with
  checksums.
- Review build scripts, proc macros, native dependencies, default features,
  licenses, advisories, and source origins.
- Separate dependency acquisition from build/test execution.
- Release CI uses a prevalidated cache or vendored/snapshotted dependency set and
  runs build/tests with network denied.
- Cache keys include lockfile, toolchain, target, profile, and relevant config
  digests.
- A cache hit cannot bypass integrity checks or quality gates.
- Generate an SBOM/dependency manifest for release artifacts.

## Repository automation package

`xtask` is a non-published workspace package outside production dependency
graphs. Configure `.cargo/config.toml` so `cargo xtask ...` expands to the
workspace task package.

Stable commands:

```text
cargo xtask bootstrap
cargo xtask environment verify|manifest
cargo xtask format [--write]
cargo xtask lint
cargo xtask build --profile dev|test|release
cargo xtask test smoke|unit|integration|system|conformance|property|security|all
cargo xtask test performance --profile pr|nightly|release
cargo xtask fuzz smoke|campaign
cargo xtask coverage
cargo xtask mutate
cargo xtask golden check|update
cargo xtask quality report
cargo xtask ci stage1|pr|nightly|release
cargo xtask clean-results
```

Command rules:

- local and CI automation call the same commands;
- invalid or empty suite selection fails;
- active suite minimum counts come from the stage/test manifest;
- no automatic retry changes a failed required result to pass;
- check commands do not modify tracked files;
- golden update is explicit and lists every changed oracle;
- every aggregate command emits a machine-readable summary;
- interruption marks the run incomplete;
- verbose mode prints exact nonsecret commands/configuration;
- paths are resolved/canonicalized within approved roots; and
- cleanup rejects root, parent, unresolved, symlink-escaped, or non-result paths.

## Generated evidence layout

```text
test-results/<run-id>/
├── environment.json
├── task-summary.json
├── junit/
├── coverage/
├── mutation/
├── fuzz/
├── benchmarks/
├── static-analysis/
├── dependency/
├── logs/
└── quality-report/
```

`run-id` is unique and does not carry semantic identity. Reports identify commit,
tree cleanliness, toolchain, target, profile, fixture-manifest digest, limits,
seed, host image, and task status. Sensitive input excerpts and credentials are
excluded.

## CI profiles

### Stage 1

Runs only active Stage 1 checks:

- environment verification;
- workspace metadata/build/docs;
- formatting/lint/dependency boundaries;
- automation unit tests;
- package shell smoke; and
- probe dependency allowlist.

No compiler/conformance/performance placeholder is intentionally failed.

### Pull request

- environment and repository coherence;
- formatting/lint/docs/dependencies;
- active smoke/unit/integration/system/conformance suites;
- bounded property/security/fuzz smoke;
- coverage on affected production code; and
- informational performance smoke or gross-complexity guard.

### Nightly

- full declared host/MSRV matrix;
- extended property/fuzz/security;
- performance baseline comparison;
- stress/soak;
- mutation analysis;
- dependency/advisory refresh; and
- complete quality report.

### Release

- clean protected release candidate;
- prevalidated offline dependencies;
- full supported matrix and all active release suites;
- controlled performance and resource profiles;
- retained immutable evidence;
- package/SBOM/license verification; and
- publish only after independent release approval.

Untrusted pull-request code never runs with write-capable release credentials.
Workflow permissions are least privilege and reviewed for PR, branch, schedule,
and release events.

## Stage 1 acceptance

- [ ] Layer 0 works on every supported bootstrap host.
- [ ] Layer 1 validates all pinned workspace tools.
- [ ] The optional development container passes the same Stage 1 gate.
- [ ] Environment manifests contain no secrets and identify all relevant tools.
- [ ] Host, container, and CI call identical `xtask` tasks after bootstrap.
- [ ] Concurrent jobs cannot share mutable roots accidentally.
- [ ] Network-denied Stage 1 build/tests pass with prepared dependencies.
- [ ] Running Stage 1 CI leaves tracked files unchanged.
- [ ] A deliberate task/test/dependency-boundary failure propagates nonzero.
- [ ] Generated evidence stays beneath the configured result root.
