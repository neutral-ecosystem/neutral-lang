<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete expression wire literals

`static.hex` copies `complete-static-project` from the frozen expression
`vectors.json`; `residual.hex` and `template.hex` copy `complete-guarded-residual`
and `complete-public-template` from `complete-vectors.json`.

The reviewed originals are under `portable/specs/fixtures/expressions/`. These
runtime-owned copies carry no producer implementation claim and must keep the
registered bytes unchanged.
