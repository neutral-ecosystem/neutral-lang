<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition contract oracle

Runtime-owned copies of reviewed inputs live here so tests do not depend on the
portable plan. `reference.py` is a test-only Python encoder of the frozen /2
transcript grammar. It reuses only unchanged /1 framing primitives from the
independent test oracle, never compiler, codec or production identity code.
`mod.rs` verifies literal bytes/digests, input integrity and adversarial partitions.
Production comparisons use the same immutable expectations. Ordinary tests run
with `cargo xtask test all`; focused runs use `cargo test -p neutral-test-suite
composition_contract`. Python 3 is required; missing tools fail, not skip.

`capture.rs` additionally runs all registered catalogue cases through the explicit
compiler capture boundary, then inspects accepted catalogues through the independent
reader. It checks aliases, transitive locks, restrictions, public reference paths,
optional origins, old-schema adaptation, replay, permutations and concurrency.
The production captured-identity transcript is compared with literal bytes/digest
and exact/one-over framing budgets. Catalogue-only cases get an explicit test
source declaring their reviewed roots; those are not source-compilation tests.
`pipeline.rs` additionally runs the actual successor compiler, complete reader
and binary round trip for accepted source families, boundary cases and legacy-leaf
adaptation. It checks frozen source diagnostic codes, both nominal variant origins,
nested closed defaults, ordinary reuse versus reference cycles, heterogeneous
lists, independent bounds, hostile frames, shuffled captures and concurrency.
The complete production logical transcript is compared byte-for-byte and
digest-for-digest with the frozen second implementation.
`reader_probe.rs` checks public field/alternative/reference closure and cyclic
reference worklists, private reuse redaction, precise source-default/reuse spans,
invalid companion rejection, safe vocabulary owners, selection-invariant complete
identity, and the real compiler-free probe executable on successor artifacts.
`hardening.rs` exercises real successor syntax-cache hits and changed units,
vocabularies/defaults, host IDs, policies, failed generations and concurrent
caller isolation. Complete IR/companions and encoded bytes must equal clean
compilation. It also sweeps every independent composition and wire consumer
control, composition producer policy and JSON/scalar/shape capture controls at
their acceptance threshold and one below/above, and mutates every
artifact byte before independent validation. These sweeps complement the literal
catalogue/dependency boundary expectations; they do not prove all allocation
failure paths; those are exercised separately below.

`allocation.rs` uses the core's non-default, thread-local `allocation-testing`
feature to reject every observed reservation independently in accepted capture
families, the adapted leaf embedding graph, source resolution/SCCs, complete
companions, reader validation, views, logical identity, wire encoding/decoding
and probe inspection. Defaults, references and vocabulary-backed values exercise
additional branches. Warm and changed-unit cache sweeps verify the previous
successful generation survives each failure and subsequent compilation recovers.
Real token cancellation is injected at each observed processing reservation;
eight capture controls are tested independently below/at/above the boundary.
Fixture/request setup happens outside observation. These tests simulate checked
reservation failures, not physical whole-process memory exhaustion.
Replay copying and JSON presentation are also included: every observed growth
failure and reservation-boundary cancellation must reject publication. Replay
preserves exact bytes, locks, features and policies; renderer tests additionally
cover empty views, Unicode and JSON control-character escaping.

`graphs.rs` supplements the immutable literals with transitive vocabulary graph
families and source import chains, diamonds and valid reference-type SCCs. Source
topology is compared with the existing graph API, then independently decoded and
inspected; shuffled captures must produce identical wire bytes.

`identity.rs` compares all five frozen /2 baseline partitions, twelve semantic/
evidence variations and eight rejection vectors against production boundaries.
`identity_facts.rs` constructs raw test models from literal facts without sorting
or normalizing them; these models are framing inputs, not reader authority.
`interface_reference.py` independently projects public declarations/contracts
before using the pinned Python grammar. The original `reference.py`, vectors and
fixture/oracle bytes remain unchanged. Tests additionally cover real default versus
supplied values, omission versus null, decoded origin evidence, every explicit
derivation control, byte/frame boundaries, allocation/cancellation recovery and
root invariance. Missing/private/wrong-profile/duplicate roots cannot acquire
artifact identity through the reader. All 26 registered request cases now reach
their applicable capture/compiler/independent wire boundaries; catalogue-only
cases retain explicit test sources rather than invented literal source outcomes.
