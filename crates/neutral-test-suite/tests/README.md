<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cross-package test modules

This directory owns executable smoke, integration, system, conformance,
property, security, deterministic fuzz-style, and hardening evidence.

`module_graph/` validates the captured module/import graph against its pinned corpus
and runs adversarial graph-order, limit, and exclusion checks.

`project_identity/` owns literal transcript/digest vectors and identity-layer
equivalence, exclusion, context, cancellation, and boundary regressions.
