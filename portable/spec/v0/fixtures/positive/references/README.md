<!-- SPDX-License-Identifier: Apache-2.0 -->

# Positive typed identity-reference fixtures

These fixtures freeze Stage 5.2 behavior for `Ref<T>` and `ref(name)`. They
cover forward targets, references nested in contextual records, recursive
nominal schemas broken only by reference edges, and identity cycles that remain
outside ordinary value-dependency evaluation. References carry document-local
identity only and imply no ownership, containment, ordering, or readiness.
