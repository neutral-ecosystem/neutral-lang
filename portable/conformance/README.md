<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 conformance

Status: accepted release-train baseline

[manifest.toml](manifest.toml) is the activation inventory for the inherited
v0 suite and v1 delta suites. Each active v1 case must name exact bytes and an
expected result; no planned inventory entry is passing evidence.

The v1.0.0 gate requires the full inherited v0 corpus, all v1 fixture/oracle
families, reviewed canonical identity vectors, and independent Reader, generic
authoring, and project-neutral consumer-boundary probes. See [the plan](../PLAN.md) and
[testing rules](../development/04-TESTING-CONFORMANCE.md).

The scheduled [consumer-readiness requirements](../specs/contracts/CONSUMER-READINESS.md)
add composite vocabulary fixtures before `v0.8.0` and consumer-data/authoring
fixtures before `v0.9.0`. They are not active manifest entries or passing evidence
until their schemas, literal outcomes, and identity vectors have been reviewed.
Core acceptance and consumer interpretation failures must have separate oracles;
no backend execution or external product implementation is required.
