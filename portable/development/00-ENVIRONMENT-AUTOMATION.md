# Neutral v0 development environment and automation

Status: required implementation operations contract

This document owns repository tooling, reproducible environments, task
automation, CI profiles, and generated evidence locations. It must not influence
Neutral language semantics.

## Environment layers

Bootstrap is deliberately split to avoid requiring `cargo xtask` before Rust
and Cargo exist.

### Layer 0: host bootstrap

Thin `scripts/linux/bootstrap.sh` and `scripts/win/bootstrap.ps1` scripts:

1. detect supported host/architecture;
2. verify required shell/PowerShell, TLS, certificate, archive, and checksum
   facilities;
3. verify the approved Rust toolchain and direct the user to an explicit
   installation action when it is missing;
4. verify download checksums/signatures when supplied by the publisher;
5. refuse unsupported/unpinned versions unless an explicit development override
   is recorded; and
6. invoke `cargo xtask bootstrap` only after Cargo is available.

The scripts contain no compiler/test policy beyond reaching the Rust task
runner. They never use `curl | sh`, elevate privilege silently, write outside
documented tool/cache roots, or modify shell startup files without consent.

The host scripts resolve tool executables through the optional
`NEUTRAL_CARGO_COMMAND` and `NEUTRAL_RUSTC_COMMAND` variables. They default to
`cargo` and `rustc`, respectively, and invoke the resolved executable directly
(without shell evaluation), so toolchain wrappers can be selected explicitly.

### Layer 1: workspace bootstrap

`cargo xtask bootstrap` verifies the core repository files and selected Rust
toolchain, configures local ignored result directories, and emits an environment
manifest. It does not install Rust or external Cargo tools.

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

- rolling stable development/CI Rust channel;
- minimum supported Rust version (MSRV);
- release toolchain;
- supported/tested/experimental/unsupported host triples;
- primary system-test host;
- finite host/filesystem matrix;
- benchmark runner identity and operating policy; and
- approved versions of formatter, linter, coverage, fuzz, mutation, benchmark,
  dependency, link, and documentation tools.

Development, CI, release qualification, and the convenience container follow
the latest stable Rust channel. The exact resolved `rustc` and Cargo versions
must be recorded in each environment/evidence manifest because a rolling
channel is intentionally not a reproducible version pin. MSRV remains an exact,
separate compatibility contract and does not select the development compiler.

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

- Commit root `Cargo.lock` as the sole release dependency lock because the
  workspace ships binaries. `fuzz/Cargo.lock` is an explicitly isolated,
  non-release cargo-fuzz tool lock and cannot enter shipped package closures.
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
- `config/dependency-sources.toml` owns allowed lockfiles and source classes;
  `cargo xtask version check` validates the release lock offline and rejects
  Git sources, missing registry checksums, or stale workspace package entries.

## Repository automation package

`xtask` is a non-published workspace package outside production dependency
graphs. Configure `.cargo/config.toml` so `cargo xtask ...` expands to the
workspace task package.

Stable commands:

```text
cargo xtask bootstrap
cargo xtask environment verify|manifest
cargo xtask fmt [--write]
cargo xtask lint
cargo xtask check
cargo xtask build --profile dev|release
cargo docs
cargo xtask test smoke|unit|integration|system|conformance|property|security|all
cargo xtask test performance --profile pr|release|soak
cargo xtask fuzz smoke|campaign
cargo xtask coverage
cargo xtask mutate
cargo xtask quality [--profile pr|release]
cargo xtask validate <artifact>|binaries
cargo xtask package
cargo xtask release prepare
cargo xtask version show|check|prepare <version>
cargo xtask portable verify|snapshot
cargo xtask clean
```

Command rules:

- local and CI automation call the same commands;
- invalid or empty suite selection fails;
- active suite minimum counts come from the durable `current` test profile;
- source files may declare path-based private test modules but may not contain
  inline test bodies;
- traceability checks reject missing accepted IDs, unchecked master syntax,
  unregistered normative fixtures/oracles, and missing manifest paths;
- version checks derive the release tag from the root workspace version, keep
  all frozen contract domains separate, and emit review-only transition plans;
- portable checks validate active-series identity, local links, fixture freeze
  digests, manifest ownership, and the archive dependency boundary;
- portable snapshots are atomic, digest-addressed copies under ignored
  `test-results/portable/snapshot/` and never write to the roadmap repository;
- `config/generated-outputs.toml` owns generation, validation, and tracking
  policy for every generated product class;
- `config/repository-layout.toml` owns top-level directory responsibility and
  lifecycle, while `config/test-levels.toml` maps every durable test purpose to
  its independently runnable command;
- coverage is nightly-only and writes browsable HTML plus machine-readable JSON
  beneath ignored `test-results/analysis/coverage/`; its 85% line, 90% function,
  and 80% region thresholds exclude only separately command-tested repository
  automation; a future exclusion requires an explicit reviewed policy change;
- `cargo docs` generates crate rustdoc plus a Cargo-metadata-driven workspace
  index without a fixed package list;
- no automatic retry changes a failed required result to pass;
- check commands do not modify tracked files;
- every aggregate command emits a machine-readable summary;
- interruption marks the run incomplete;
- verbose mode prints exact nonsecret commands/configuration;
- human-readable automation output begins with a bracketed category, such as
  `[info]` or `[error]`; structured payloads use a category followed by the
  payload, such as `[manifest] { ... }`;
- paths are resolved/canonicalized within approved roots; and
- cleanup rejects root, parent, unresolved, symlink-escaped, or non-result paths.

## Generated evidence layout

```text
test-results/
├── bootstrap/
│   └── environment.json
├── ci/
│   ├── pr/
│   │   └── run-<process-id>-<sequence>/
│   │       └── task-summary.json
│   └── release/
├── release/
│   └── package/
├── version/
├── suites/
│   ├── conformance/
│   ├── fuzz/
│   ├── performance/
│   └── security/
└── analysis/
    ├── coverage/
    ├── dependency/
    ├── mutation/
    └── quality-report/
```

Only directories for executed tasks are created. The durable CI profile is part of
the directory path, while `run-<process-id>-<sequence>` is unique and carries no
semantic identity. Reports identify commit, tree cleanliness, toolchain, target,
profile, fixture-manifest digest, limits, seed, host image, and task status.
Sensitive input excerpts and credentials are excluded.

## CI profiles

### Push and pull request

- environment and repository coherence;
- formatting/lint/docs/dependencies;
- smoke/unit/integration/system/conformance/property/security suites;
- bounded property/security/fuzz smoke;
- standalone probe dependency check; and
- generated workspace documentation.

### Release

- clean protected release candidate;
- prevalidated offline dependencies;
- full supported matrix and all release suites;
- controlled performance and resource profiles;
- retained immutable evidence;
- package/SBOM/license verification; and
- publish only after independent release approval.

Untrusted pull-request code never runs with write-capable release credentials.
Continuous integration runs after every push to `main` (and may be manually
dispatched). Release qualification runs after every pushed tag (and may be
manually dispatched). Workflow permissions are least privilege and reviewed for
branch, tag, and manual-dispatch events.

## Stage 1 acceptance

- [x] Layer 0 works on every supported bootstrap host.
- [x] Layer 1 validates all pinned workspace tools.
- [x] The optional development container passes the same Stage 1 gate.
- [x] Environment manifests contain no secrets and identify all relevant tools.
- [x] Host, container, and CI call identical `xtask` tasks after bootstrap.
- [x] Concurrent jobs cannot share mutable roots accidentally.
- [x] Network-denied Stage 1 build/tests pass with prepared dependencies.
- [x] Running Stage 1 CI leaves tracked files unchanged.
- [x] A deliberate task/test/dependency-boundary failure propagates nonzero.
- [x] Generated evidence stays beneath the configured result root.
