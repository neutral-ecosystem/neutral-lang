<!-- SPDX-License-Identifier: Apache-2.0 -->

# Repository structure

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

| Path | Responsibility |
| --- | --- |
| `assets/` | Shared diagrams and official visual identity assets |
| `config/` | Machine-readable repository policy |
| `conformance/` | Immutable per-release contracts, fixtures, and oracles |
| `crates/` | Rust implementation, binaries, and crate-owned tests |
| `docs/` | Maintained task-oriented documentation hub and guides |
| `fuzz/` | Fuzz harnesses and subsystem definitions |
| `quality/` | Quality policy, reviews, approvals, and release evidence |
| `scripts/` | Thin Linux and Windows host adapters |
| `xtask/` | Stable commands and shared automation policy |
| `target/` | Ignored Cargo and local Rustdoc output |
| `test-results/` | Ignored generated test, analysis, quality, and release evidence |

The executable ownership inventory is
[`config/repository-layout.toml`](../config/repository-layout.toml). Generated
output policy is defined in
[`config/generated-outputs.toml`](../config/generated-outputs.toml).

The API website is generated under ignored `target/doc/` and published at the
[Neutral API documentation website](https://neutral-lang-doc.younesrabeh.workers.dev/).

## Generated output and cleanup

Use `cargo xtask clean` to remove generated repository evidence. Cleanup is
restricted to the validated relative result root and cannot target the
repository root, a parent or absolute directory, released conformance data,
tracked quality policy, or an installed portable plan.

## Portable development plans

`portable/` contains the reviewed future-version plan currently active for
development. Install, verify, or archive plans through `cargo xtask portable`;
a plan is a development input and never changes an already released
conformance bundle.
