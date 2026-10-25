<!-- SPDX-License-Identifier: Apache-2.0 -->

# Current source examples

`minimal.neu` is the current single-source CLI example. Compile it with
`cargo run --package neutral-cli -- compile --output target/minimal.nir examples/minimal.neu`,
then inspect it with `cargo run --package neutral-probe -- target/minimal.nir`.

The artifact contains the complete current project. Probe inspection does not
evaluate deferred expressions or execute operations. Historical release fixtures
remain under `conformance/releases/`; they are not current CLI inputs.
