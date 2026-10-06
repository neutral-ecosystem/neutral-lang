<!-- SPDX-License-Identifier: Apache-2.0 -->

# Vocabulary tests

This directory owns strict captured-bundle parsing, schema, compatibility,
limit, and hostile-input tests.

`contracts.rs` includes old-project-schema compatibility regressions for
composite type objects, variants, presence/default/restriction members,
dependency envelopes, and feature-lock assertions. These protect existing
behavior; they do not claim support for the proposed composition extension.

`composition.rs` exercises the separate explicit-schema catalogue boundary:
typed defaults/restrictions, variant/public closure, exact dependencies,
independent limits, cancellation, compatibility leaves and concurrent requests.
It does not stand in for compiler/IR/wire/probe end-to-end conformance.
`composition_values.rs` checks byte-pinned literal catalogue cases and shared
closed supplied-value/default materialization, origins, exact/one-over limits,
ordering and concurrency. Invalid supplied values cannot bypass bundle restrictions.
