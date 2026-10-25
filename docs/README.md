<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral documentation hub

[< Back to Neutral](../README.md)

Use this hub to find task-oriented project documentation. Generate public Rust
API documentation locally with `cargo xtask docs`, or browse the live
[Neutral API documentation website](https://neutral-lang-doc.younesrabeh.workers.dev/).

| Task | Guide |
| --- | --- |
| Set up a workstation and use the development loop | [Development workflow](development.md) |
| Optimize human DX and eliminate manual bookkeeping | [Workflow optimization](workflow-optimization.md) |
| Advance from one version to the next | [Development lifecycle](development-lifecycle.md) |
| Distinguish development cleanup from stable compatibility | [Compatibility policy](compatibility-policy.md) |
| Assess completed v1 against real consumer needs and proposed v2 requirements | [Ecosystem-readiness audit](v2-ecosystem-readiness-audit.md) |
| Design post-v2 packages, the runtime host, dynamic editor and self-hosted CI | [Runtime and package architecture](v2-runtime-and-package-architecture.md) |
| Review a readable TOML-inspired `.neu` surface and optional typed TOML capture | [Source syntax proposal](source-syntax-proposal.md) |
| Run quality, coverage, fuzzing, or performance checks | [Quality and analysis](quality-and-analysis.md) |
| Prepare a version, package, or release | [Release and versioning](release-and-versioning.md) |
| Understand repository directories and generated outputs | [Repository structure](repository-structure.md) |
| Resolve common local failures | [Troubleshooting](troubleshooting.md) |

Reference material remains with its owning subsystem:

- [Crate responsibilities](../crates/README.md)
- [Released conformance bundles](../conformance/README.md)
- [Quality system](../quality/README.md)
- [Platform adapters](../scripts/README.md)
- [Automation internals](../xtask/README.md)
