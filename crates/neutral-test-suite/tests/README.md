<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cross-package test modules

This directory owns executable smoke, integration, system, conformance,
property, security, deterministic fuzz-style, and Stage 9 hardening evidence.

`stage3/` validates the captured module/import graph against its pinned corpus
and runs adversarial graph-order, limit, and exclusion checks.
