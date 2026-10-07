<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition validation boundary

This module validates explicitly selected new vocabulary contracts without host
acquisition or compiler linkage. Decoding owns closed member sets and budgets;
closure validation owns canonical dependencies, nominal/public closure and
cycles; value validation owns closed defaults and declarative restrictions.
Supplied-value materialization reuses that validator and publishes materialized
meaning separately from safe occurrence origins; references cannot be invented
by a closed value. Independent value-node/work/depth budgets limit default expansion.
The shared raw data model belongs to `neutral-ir`, not a compiler-private AST.

Resolved bindings use that same structural value model with an exact module-symbol
reference parameter. Closed defaults have an uninhabited reference parameter, so
they cannot smuggle binding references into a contract. `bindings` validates the
whole supplied binding index, invariant reference types, public target closure,
default/restriction materialization and actual reference occurrence paths. Work
and value visits are cumulative across bindings, not reset for each reference.
Previously accepted contracts and unused defaults are rechecked against stricter
semantic policy. Captured-byte/digest checks remain the capture boundary's job.
These resolved bindings are not source compilation or complete project artifacts.

`model` independently revalidates canonical catalogues reconstructed from IR.
It shares closure and materialization rules, checks explicit legacy-leaf adaptation
and rejects noncanonical defaults/choices before the complete reader accepts them.
The successor compiler feeds resolved source contracts and bindings through these
same APIs; the module remains independent of compiler syntax and host services.

`copy` reserves occurrence-path and field-name storage fallibly before copying.
Private reservation callbacks permit deterministic failure-path tests without a
global allocator. Materialization uses fallible boxed/recursive copies and ordered
indexes; graph worklists reserve before growth and charge index shifts to work.
Successor validated scopes use core `Shared::try_new`, not infallible `Arc::new`.
The JSON parser tests interruption at each reservation checkpoint. This remains
partial pipeline hardening. Schema retention now uses request-local fallible
string/vector/box/membership helpers, with checks before growth and fault injection
through the same private checkpoints. Legacy schema parsing and remaining producer,
consumer and companion paths still require allocation review.

This boundary is separate from the frozen project `1.0` validator. A successful
catalogue is not a compiled project, encoded artifact, or release approval.
