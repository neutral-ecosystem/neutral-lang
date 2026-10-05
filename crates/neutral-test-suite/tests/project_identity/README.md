<!-- SPDX-License-Identifier: Apache-2.0 -->

# Project identity conformance

This suite owns executable literal transcript/SHA-256 vectors, complete logical
meaning, layer exclusions, bounded construction, cancellation, and artifact
selection/format separation. `vectors.json` is the execution copy of the portable
release asset; it remains usable after the portable plan is archived.
`reference.py` independently implements all four frozen identity layers using
Python's standard library, explicit big-endian framing, and SHA-256. Rust tests
project typed input facts into its data-only request; they never supply Rust
transcripts as expectations. Both implementations must match every retained
literal vector, accepted corpus case, and adversarial identity vector, including
exact byte/node boundaries, tuple collision paths, and canonical root selections.

`integration.rs` checks public reader facts, standalone reader-only probe root
invariance, and actual caller-owned syntax cache execution versus clean builds.
It observes parsed/reused units, changes private source bytes and source IDs,
revalidates changed vocabulary and controls, checks failed-run recovery, and
compares complete artifacts under concurrent schedules. Compiler-owned tests
add misplaced-entry and simulated digest-collision rejection.

Run `cargo test --package neutral-test-suite project_identity`. Python 3 is
required (`python3`, or `python` on hosts using that name); set
`NEUTRAL_IDENTITY_REFERENCE_PYTHON` to override interpreter discovery. Missing
Python fails conformance rather than silently skipping independent review.
