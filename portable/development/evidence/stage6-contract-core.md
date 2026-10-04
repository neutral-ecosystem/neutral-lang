<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 6 contract and core boundary

Reviewed: 2026-10-04. Scope: contract/fixture and core implementation gates only.

The [accepted contract](../../specs/contracts/PROJECT-IR.md) is pinned in
[freeze.toml](../../specs/contracts/freeze.toml). Five reviewed complete,
private-only, public-view, private-view, and malformed-artifact cases are pinned
with their oracle in the [manifest](../../conformance/manifest.toml).
Crate-owned copies remain runnable after the portable plan is archived.

## Implemented boundary

- `compile_project` publishes every supplied module and declaration only after
  graph, visibility, contextual value, and closed-default validation. Reuse is
  materialized; reference targets remain typed identities. Invalid unused
  defaults and disconnected private units are not skipped.
- Complete IR retains canonical locked vocabulary schemas, all private content,
  original-byte root maps and occurrence dependency provenance, source/vocabulary
  evidence, explicit limits, and independently recomputable resource counts.
- `ValidatedProject::from_ir` independently rejects malformed values, schemas,
  missing maps/type edges, forged source ownership, resource mismatches, embedded
  type cycles, and bounds/cancellation violations. Its owning crate has no
  compiler dependency. Producer and reader share the bounded schema traversal
  in `neutral-ir`; export types are bounded before cloning.
- `derive_view` returns only selected public roots and their required public
  type/value/reference/vocabulary closure. References inherited through private
  value reuse retain their public target. Empty selection and private-only
  projects remain complete; source/private identities and raw provenance never
  appear in the view. Capture order, host IDs, and concurrent compilation do
  not change complete logical meaning.

## Reproducible checks

| Command | Outcome |
| --- | --- |
| `cargo xtask fixtures sync` | 49 fixture/oracle pairs verified; ten new digest fields synchronized |
| `cargo test --package neutral-test-suite project_ir` | 15 project contract/core regressions pass |
| `cargo test --package neutral-reader --test project` | Four independent reader/schema/producer-bound regressions pass |
| `cargo xtask ci pr` | Full workspace checks, strict lint, tests, smoke, independent probe build, and documentation pass |

Raw toolchain and command logs remain generated under `test-results/workflows`;
they are not copied into per-command Markdown records.

## Deliberate remaining gates

This is typed in-process IR and a versioned Rust result boundary, not a new
external project encoding. Standalone reader-only project probing, encoded
round-trip/hostile decoding, serialization/clean-incremental equivalence, and
the final public envelope review remain Stage 6 integration/validation work.
Canonical project/captured/derivation identities are owned by Stage 7; no
provisional project hash is introduced here. Root/edge maps do not claim
fine-grained serialized value-node provenance. The single-file CLI does not
silently enable an incomplete v1 profile. No release/version promotion is made.
