<!-- SPDX-License-Identifier: Apache-2.0 -->

# xtask

`xtask` is Neutral's repository automation package. It provides the stable
`cargo xtask` commands for bootstrap, boundary checks, testing, CI, generated
evidence, and safe cleanup.

During Stage 9, `cargo xtask coverage`, `cargo xtask mutate`, and
`cargo xtask fuzz campaign` invoke the real external Cargo tools with targets,
budgets, and thresholds read from `config/quality-gates.toml`. A missing tool
fails the gate; deterministic fuzz-style smoke tests are not reported as a
coverage-guided campaign.

The `cargo docs` alias runs workspace rustdoc and generates a searchable
workspace landing page from Cargo metadata. Package additions, removals,
descriptions, ownership, versions, visibility, and dependency relationships are
therefore reflected without editing an HTML package list.

It is non-published and outside production dependency graphs. It enforces
repository policy around the compiler rather than implementing Neutral language
behavior, and writes generated evidence only beneath the configured
`test-results/` root.
