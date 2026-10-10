<!-- SPDX-License-Identifier: Apache-2.0 -->

# Independent document validation

This private boundary checks untrusted in-process artifacts before the public
reader publishes a `ValidatedDocument`. `document.rs` orders cross-artifact
checks, `schema.rs` validates captured vocabulary contracts, `value.rs` checks
typed values and record schemas, and `provenance.rs` verifies source/value edges.

Keep these checks independent of compiler internals. A constructed IR value or
matching digest does not establish validation authority. Complete project and
composition-profile validators retain their existing separate boundaries.
