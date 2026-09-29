<!-- SPDX-License-Identifier: Apache-2.0 -->

# Repository policy configuration

This directory owns machine-readable development, host, test, quality,
dependency, generated-output, repository-layout, encoding, and release-scope
policy. `conformance.toml` selects the immutable inherited language corpus
independently of the Cargo package version. `xtask` validates these inputs;
production language behavior is governed
only by the frozen portable contracts. Generated reports never belong here.

`automation.toml` owns the local Cargo/Rust compiler command defaults and the
generated-results root. Optional per-invocation environment overrides are
documented in the [developer command guide](../docs/workflow-optimization.md).
Repository layout, quality thresholds, and the active conformance corpus stay
in their own policy files rather than being duplicated in xtask constants.
