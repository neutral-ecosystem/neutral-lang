<!-- SPDX-License-Identifier: Apache-2.0 -->

# 06 — Validation ledger

Status: active release-train ledger

## Rules

The checklist is the completion authority; this ledger is the status view. A
stage is **not started**, **in progress**, **validated**, or **released**. It
can be marked validated only when all of its checklist items are checked and
the matching conformance suites are active and passing. A stage is released
only when its `.4 -> next .0` promotion evidence is retained.

| Stage | Transition | Current status | Validation required to advance |
| --- | --- | --- | --- |
| 1 | `v0.1.0 -> v0.2.0` | not started | profile matrix, inherited v0 corpus, v1 exclusion/audit evidence |
| 2 | `v0.2.0 -> v0.3.0` | not started | capture request/closure/limits/no-I/O fixtures |
| 3 | `v0.3.0 -> v0.4.0` | not started | module/import/SCC/determinism fixtures |
| 4 | `v0.4.0 -> v0.5.0` | not started | visibility/public closure/cross-module semantic fixtures |
| 5 | `v0.5.0 -> v0.6.0` | not started | exact vocabulary lock and inert-location fixtures |
| 6 | `v0.6.0 -> v0.7.0` | not started | project IR/reader/view public probe |
| 7 | `v0.7.0 -> v0.8.0` | not started | reviewed transcript and identity vectors |
| 8 | `v0.8.0 -> v0.9.0` | not started | dynamic catalogue and generic Editor no-op probe |
| 9 | `v0.9.0 -> v1.0.0` | not started | full manifest, all probes, hardening, and release review |

## Per-release validation stack

For every `v0.n.1`, `.2`, `.3`, `.4`, record:

- [ ] exact implementation revision and toolchain;
- [ ] inherited v0.1 suite result;
- [ ] active v1 manifest suites and fixture/oracle result;
- [ ] formatter, linter, unit, integration, property/fuzz, and limit result;
- [ ] public reader/authoring/consumer probe result applicable to the release;
- [ ] deterministic repeated/concurrent result and identity-vector result;
- [ ] reviewed migration note and any deliberately deferred checklist items.

The final `v1.0.0` promotion additionally requires every row above plus all
items in [the v1 checklist](../specs/contracts/v1-checklist.md).
