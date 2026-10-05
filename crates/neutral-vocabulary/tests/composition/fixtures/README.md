<!-- SPDX-License-Identifier: Apache-2.0 -->

# Captured composition bundles

`bundle.json` is the runtime-owned copy of the composition example. It exercises
exact lock validation, restricted defaults, optional nullability, list/variant
typing and reference type closure. Tests never depend on an active or archived
portable-plan directory. Hostile/boundary cases are derived explicitly by the
owning integration tests and checked against literal error classifications.
