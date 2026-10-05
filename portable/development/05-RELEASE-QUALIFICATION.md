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
profiles and state that consumer execution and authoring-client UX are not Neutral core.

`v0.8.0` additionally requires the composite vocabulary extension gate
(V1-VOC-005..011); `v0.9.0` additionally requires the consumer data/authoring
boundary (V1-CONS-001..010). Both must be schema-reviewed, registered in the
conformance manifest, implemented, and independently validated before promotion.
Previously retained Stage 7 identity evidence does not complete these additions.
The final gate requires their full compatibility, limits, identity, privacy,
and reader/authoring evidence, without depending on another project or executor.

No release may claim conformance from design acceptance, an implementation demo,
or one consumer integration alone.
