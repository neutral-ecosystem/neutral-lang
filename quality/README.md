<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral quality system

This directory contains durable quality governance and reviewed conclusions.
It is deliberately separate from generated measurements: coverage reports,
fuzz corpora, mutation output, benchmark samples, and profiler captures remain
under ignored `test-results/` until verified release measurements are retained
automatically beside the release approval.

## Structure

| Path | Responsibility |
| --- | --- |
| `manifest.toml` | Machine-readable inventory and lifecycle state for every durable quality document |
| `policy/` | Stable quality criteria and standards-tailoring decisions |
| `reviews/` | Human-reviewed security, dependency, isolation, product, and residual-risk assessments |
| `evidence/<release>/` | Maintained summaries that support one released version |

Policy defines what must be demonstrated. Reviews interpret source and dynamic
results. Versioned evidence records what was actually demonstrated for a
release. A generated result is never promoted merely by copying raw output; its
method, scope, outcome, owner, and limitations must be reviewed first.
Frozen language contracts, fixtures, and oracles remain in
[`conformance/`](../conformance/README.md), not in this approval tree. The
package release version and the inherited language-behavior corpus have
different lifecycles.

`cargo xtask check` validates that every authored non-README Markdown document
is listed once in `manifest.toml`, remains in its declared category, exists, and
carries the SPDX marker derived from the root workspace license. The generated
`STATUS.md` is verified separately against release records.

## Managed workflow

```text
cargo xtask quality status
cargo xtask quality evaluate --profile release
cargo xtask quality approve --release v<version>
cargo xtask quality render
cargo xtask quality verify
```

`evaluate` requires a clean checkout and retains a passing evaluation beneath
ignored generated quality results. `approve` records that a clean `main` HEAD
with the matching release version passed release evaluation. It creates one
`record.toml` bound to the commit, evaluation, and gate-configuration digest.
Evidence files remain editable without recomputing a release-wide evidence
hash. Machine measurement snapshots are different: their exact report bytes and
code/test/fixture/configuration input fingerprints are verified. Altering either
invalidates the snapshot; handwritten status labels cannot satisfy these gates.
See [quality commands](../docs/quality-and-analysis.md#release-measurements) for
the measurement sequence. Approval retains these reports automatically, including
for a previously approved version, without changing its historical `record.toml`.

Release preparation accepts later `main` commits descending from the recorded
candidate under the same quality-gate configuration. It reruns the full release
quality profile on the current commit before packaging; generated workflow logs
and package manifests retain that commit's identity.

`render` derives `STATUS.md` from release records. Do not edit `STATUS.md`
manually; `verify` rejects generated-status drift.
