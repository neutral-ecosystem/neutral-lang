<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition contract oracle

Runtime-owned copies of reviewed inputs live here so tests do not depend on the
portable plan. `reference.py` is a test-only Python encoder of the frozen /2
transcript grammar. It reuses only unchanged /1 framing primitives from the
independent test oracle, never compiler, codec or production identity code.
`mod.rs` verifies literal bytes/digests, input integrity and adversarial partitions.
This checks a frozen design, not implementation availability. Ordinary tests run
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
Derivation/artifact identities, complete hostile/fault/fuzz campaigns and actual
cache execution remain pending; the registered suite is still frozen.
