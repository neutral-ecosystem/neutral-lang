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
boundary and approved pure-expression/finite-composition extension
(V1-CONS-001..016). Both must be schema-reviewed, registered in the
conformance manifest, implemented, and independently validated before promotion.
Previously retained Stage 7 identity evidence does not complete these additions.
The final gate requires their full compatibility, limits, identity, privacy,
and reader/authoring evidence, without depending on another project or executor.
The extension requires frozen static/deferred semantics, independent pure
interpretation, bounded parameter/matrix expansion, and explicit unsupported
feature rejection. Existing source exclusions must be amended through reviewed
profile activation, never inferred from the approved development scope alone.

Stage 8 promotion also requires the ordered selector/phase, expression/identity,
finite-composition, and authoring slices to be complete. Unknown future values
retain explicit consumer validation obligations; successful compilation is not
proof that such values already satisfy vocabulary restrictions. Logical identity
tracks normalized static values/expanded meaning and canonical deferred
expressions, not source spelling or external bound values.

Stage 9 is qualification-only: accept feature-complete inputs, fix defects, and
finish migration/compatibility work. Missing features or changed semantics return
to Stage 8 review. Any fix invalidates affected evidence and requires fresh final
integration plus all measurements bound to changed inputs before approval.

No release may claim conformance from design acceptance, an implementation demo,
or one consumer integration alone.
