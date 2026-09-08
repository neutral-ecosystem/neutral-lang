<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 distribution assembly

Review date: 2026-09-08. Result: pass for Stage 10 Step 8.

## Candidate and selected scope

`cargo xtask package` assembled the selected source-tag and GitHub-binary scope
from clean `main` revision
`b1353acc6d93c796795492d9e96976ed96f4f7fa`. crates.io publication is not
selected, so Cargo package/upload work is intentionally not applicable.

The exact ignored assembly is under
`test-results/release/package/v0.1.0/b1353acc6d93c796795492d9e96976ed96f4f7fa/x86_64-unknown-linux-gnu/`.
The single supported target is `x86_64-unknown-linux-gnu`.

## Declared artifacts

`release-manifest.json` records every selected payload filename, SHA-256,
Apache-2.0 license, producer version `0.1.0`, source commit, and distribution
channel. `SHA256SUMS` covers both binaries, the deterministic source archive,
SBOM/dependency lock, provenance, installation guide, license, README, package
summary, and release manifest. The checksum list is the conventional sole
self-exclusion.

The assembly contains:

- `neutral-cli` and `neutral-probe` GitHub binary assets;
- `neutral-lang-v0.1.0-source.tar`, produced by `git archive` from the exact
  candidate commit for the selected source-tag channel;
- `SBOM-Cargo.lock`, the exact release dependency inventory;
- `provenance.json`, identifying builder command, commit, version, target,
  compiler, and lock digest;
- `INSTALL.md`, `LICENSE`, `README.md`, and `package-summary.json`; and
- `release-manifest.json` plus `SHA256SUMS`.

The manifest records the v0 limitations and deferred scope: one supported Linux
x86_64 target, no runtime/application semantics, no additional host targets,
and no crates.io publication.

## Packaged-form verification

`sha256sum --check SHA256SUMS` passed for every listed file. Every manifest
artifact digest was independently recalculated and matched. Regenerating the
source archive with the same prefix and exact commit produced byte-identical
tar bytes. Its 589 tracked entries contain no `target/`, `test-results/`, fuzz
artifacts/coverage, mutation, profiler, or other transient result output.

The complete package was copied to isolated directory
`/tmp/neutral-step8-consumer.TF1eGm`. From there, its own checksum list passed,
the packaged CLI validated and compiled the source-archive minimal fixture, and
the packaged standalone probe inspected the new artifact successfully. This
verifies the shipped bytes rather than workspace binaries.

No tag, GitHub release, upload, or publication was performed by Step 8.
