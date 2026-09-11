<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral quality system

This directory contains durable quality governance and reviewed conclusions.
It is deliberately separate from generated measurements: coverage reports,
fuzz corpora, mutation output, benchmark samples, and profiler captures remain
under ignored `test-results/`.

## Structure

| Path | Responsibility |
| --- | --- |
| `manifest.toml` | Machine-readable inventory and lifecycle state for every durable quality document |
| `policy/` | Stable quality criteria and standards-tailoring decisions |
| `reviews/` | Human-reviewed security, dependency, isolation, product, and residual-risk assessments |
| `evidence/<release>/` | Immutable summaries that support one released version |

Policy defines what must be demonstrated. Reviews interpret source and dynamic
results. Versioned evidence records what was actually demonstrated for a
release. A generated result is never promoted merely by copying raw output; its
method, scope, outcome, owner, and limitations must be reviewed first.

`cargo xtask check` validates that every non-README Markdown document is listed
once in `manifest.toml`, remains in its declared category, exists, and carries
the Apache-2.0 SPDX marker.
