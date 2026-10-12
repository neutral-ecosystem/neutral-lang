<!-- SPDX-License-Identifier: Apache-2.0 -->

# File-backed consumer graphs

These 31 cases own project-neutral graph/map/condition/output/capability/privacy
examples under the [example convention](../../../contracts/EXAMPLE-CONSUMER.md).
`bundle.json` is an exact captured schema-2.0 vocabulary; `.neu` files are source
inputs, including four deliberately core-invalid files. `cases.json` records
locks, pins, request contexts and exact/one-over consumer controls.

The cycle, duplicate map-key, unsupported capability and unavailable-output
families are core-valid data, not compiler errors. Compare them with the literal
[consumer reports](../../../../conformance/oracles/consumer/graphs.json) only after
independent reader validation. Malformed request JSON intentionally includes an
unknown-member file and a duplicate-key file; do not reformat away the latter.

Registration is frozen-only, not an implemented consumer or required-active pass.
Core compilation/reader checks do not execute operations or resolve handles.
The full Consumer/1 envelope binds independently validated project identity;
these literal report projections never invent a digest or claim source hashes
are logical identities. See the convention for merge rules, privacy, control
precedence and the remaining implementation evidence.
