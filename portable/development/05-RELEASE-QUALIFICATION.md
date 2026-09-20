<!-- SPDX-License-Identifier: Apache-2.0 -->

# 05 — Release qualification

Status: accepted operational plan

## Per-stage promotion

`v0.n.4` may become the next stage's `.0` only after the active manifest,
portable contract updates, all inherited and active delta fixtures, identity
vectors, reader/probe checks, limits, deterministic behavior, and migration
notes pass under retained CI evidence.

## v1.0.0 promotion

`v0.9.4` may be released as `v1.0.0` only when all nine stages pass and the
portable package includes the complete v1 delta requirements, contracts,
decisions, fixture bytes, expected oracles, identity vectors, traceability, and
conformance manifest. The release notes must identify the supported source
profiles and state that Flow execution and Editor UX are not Neutral core.

No release may claim conformance from design acceptance, an implementation demo,
or one consumer integration alone.
