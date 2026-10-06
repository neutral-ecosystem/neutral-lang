<!-- SPDX-License-Identifier: Apache-2.0 -->

# Captured composition bundles

`bundle.json` is the runtime-owned copy of the composition example. It exercises
exact lock validation, restricted defaults, optional nullability, list/variant
typing and reference type closure. Tests never depend on an active or archived
portable-plan directory. Hostile/boundary cases are derived explicitly by the
owning integration tests and checked against literal error classifications.
The positive/negative/boundary/migration directories additionally contain literal
registered catalogue inputs. `composition_values.rs` checks their frozen byte
digests and outcomes, then supplied-value restrictions and origin separation.
