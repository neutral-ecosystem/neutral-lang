<!-- SPDX-License-Identifier: Apache-2.0 -->

# Raw expression tests

[`mod.rs`](mod.rs) owns common constructors and shape-preservation examples.
[`inspection.rs`](inspection.rs) checks allocation-free preflight and independent
exact/one-over limits. [`allocation.rs`](allocation.rs) sweeps fallible copying and
cancellation, including deep literal recovery. [`project.rs`](project.rs) checks
input/definition distinctions, exact profile selection and nominal-body preflight.

Run `cargo nextest run --config-file config/nextest.toml --package neutral-ir
--test expression`. These tests do not establish source compilation, semantic
typing, complete decoding or successor profile availability.
