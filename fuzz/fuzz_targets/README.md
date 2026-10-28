<!-- SPDX-License-Identifier: Apache-2.0 -->

# Fuzz targets

Each target drives an untrusted-input boundary with finite production controls.
Source and formatter targets capture the current expression-project contract;
IR and probe targets decode the current complete expression artifact; vocabulary
fuzzing checks current captured bundle schemas. `expression_support.rs` owns shared bounded capture, the explicit seed registry,
full-frame mutation, public inspection and runtime-generated `/3` wire seeds.
`consumer_support.rs` explicitly validates/evaluates borrowed data-only binding
requests under the registered consumer seed's public contracts. It never runs an
operation, resolves a credential or acquires an input. The compiler-independent
probe itself remains inspection-only; the fuzz harness separately calls the
explicit consumer boundary. Stable owning tests execute both support modules.
