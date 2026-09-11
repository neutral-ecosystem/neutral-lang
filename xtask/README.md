<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral repository automation

`xtask` is the single platform-neutral command and policy layer for Neutral's
developer, CI, quality, documentation, package, and release-preparation flows.
It is a non-published workspace package and must remain outside every production
dependency graph.

## Responsibility in the ecosystem

This package owns stable command parsing, Cargo invocation, repository boundary
and traceability checks, durable test selection, configured quality thresholds,
generated-evidence paths, release-candidate validation, and fail-closed package
assembly. It orchestrates production crates but does not implement Neutral
syntax, semantics, IR, encoding, reading, formatting, or host acquisition.

The root `[workspace.package].version` is the sole package-release version.
`version` commands verify its inheritance and lock coherence, while release
commands derive tag `v<version>` instead of reading a duplicated version.
`portable install <directory>` atomically copies and verifies a reviewed package
without overwriting an existing one. `portable verify` enforces the
active-package boundary, and `portable snapshot` creates an ignored,
SHA-256-addressed archive candidate without network access.

Inputs are tracked repository configuration and explicit command arguments.
Outputs are terminal messages prefixed with a category and ignored generated
files beneath `target/` or `test-results/`. It does not tag, push, upload,
publish, mutate frozen contracts, or write into another repository.

## Structure

| Path | Ownership |
| --- | --- |
| `src/interface.rs` | Stable stage-free command grammar and help |
| `src/release.rs` | Typed fail-closed release-scope parsing |
| `src/constants.rs` | Shared package, path, executable, and output names |
| `src/lib.rs` | Reusable command implementations and repository checks |
| `tests/unit/` | Parser, safety, boundary, release-plan, and helper tests |

Use `cargo xtask --help` for the authoritative command list. The root README
documents contributor usage and migration from removed legacy names. Linux and
Windows scripts are thin adapters that delegate here.

Full coverage-guided fuzzing, LLVM coverage, and mutation analysis require their
documented external Cargo tools. Missing tools fail their command; bounded fuzz
regression tests are never reported as a full fuzz campaign.

`cargo xtask docs` builds workspace rustdoc and generates a searchable landing
page from Cargo metadata. Package additions, removals, descriptions, ownership,
versions, and dependency relationships therefore require no hand-maintained HTML
package list.
