<!-- SPDX-License-Identifier: Apache-2.0 -->

# Source-bound measurement internals

The parent `quality_evidence.rs` owns receipt contracts and evidence discovery,
verification and retention orchestration. `inputs.rs` fingerprints actual inputs;
`measurement.rs` executes tools, captures reports and publishes passing receipts;
`validation.rs` independently checks report hashes and native acceptance metrics.

Storage locking and generation publication remain in `../measurement_store.rs`.
Commands use the parent facade rather than importing these private modules.
Refactoring this boundary must not weaken thresholds, change receipt schemas or
allow reports from different source inputs to be reused.
