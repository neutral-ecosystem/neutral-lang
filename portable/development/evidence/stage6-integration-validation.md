<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 6 — encoded public integration and validation

Review date: 2026-10-05. Scope: public integration/validation, not promotion.

## Implemented boundary

- Separate complete `NIR-PROJECT-CBOR/1` codec reuses the existing restricted-CBOR
  writer/parser and independent complete reader. Legacy document framing is
  unchanged. Closed tuples retain all private validation content and companions;
  public indices are derived once from declarations/provenance and checked
  against the declared interface fingerprint.
- Captured artifact bytes are now a distinct producer limit, intersected with
  consumer and hard wire ceilings. The writer polls cancellation before emitted
  primitives; the probe bounds file reads before decoding. Original source IDs
  obey reader text bounds.
- Standalone probe recognizes both formats without compiler linkage. Repeated
  public roots select a post-compilation view; counts remain complete. JSON/text
  renderers share escaping rules. Private identities, source IDs, spans, defaults,
  and raw provenance are absent from the public projection.
- Compiler round-trips preserve exact complete/private/disconnected IR,
  contextual values/defaults, reference edges, inert locations, vocabularies,
  resource/derivation facts, and original source/provenance companions.
- Root permutations/empty selections leave complete IR and encoded bytes
  unchanged. Public nominal/reference and transitive vocabulary schemas remain
  in the selected interpretive closure; private schema names are excluded.
- Immutable replay and changed-unit reconstruction match clean construction.
  Shuffled input order and concurrent compilation remain deterministic.

## Envelope and hostile-input review

Capture still uses its closed capture request/failure contract. Compile and
reader/view results retain `neutral.project-result/1`; transport failures retain
bounded decoder classes/codes/offsets. Probe failures contain decode or view
classification, no authoritative partial summary, and no hostile decoded text.
The executable prints errors only on stderr and exits unsuccessfully. JSON
stdout has no log prefixes. A valid source digest is evidence, not proof that
an arbitrary producer faithfully compiled source bytes unavailable to the reader.

Tests cover every truncated prefix, trailing bytes, wrong tuple cardinalities,
unknown schemas/tags, giant container lengths, duplicate identities/fields,
dangling/self imports, dangling provenance, forged maps/resources/fingerprints,
contextual mismatches, independent wire ceilings, captured output exact/one-over,
deep nesting, source-ID limits, cancellation, and deterministic byte mutations.
Equivalent CBOR integer widths decode equivalently and re-encode canonically;
encoded byte layout is not logical identity.

## Checks

Focused codec, standalone project probe, and complete project IR suites pass.
Fixture/contract hashes are synchronized through `cargo xtask fixtures sync`;
all 50 fixture/oracle pairs (including the new encoded descriptor) are pinned. `cargo tree --package
neutral-probe --edges normal` contains no compiler or test-support dependency.
`cargo xtask ci pr` passes formatting, checks, dependency/hygiene/traceability
validation, Clippy with warnings denied, all 438 nextest tests (none skipped),
one compile-fail doctest, command smoke, standalone probe build, and Rustdoc.
The retained workflow log is under ignored
`test-results/workflows/ci/pr/` (no manual per-command record required).
Broken stdout is checked as a bounded `probe-output-failed` host outcome,
not a Rust printing panic; malformed input never emits a partial summary.
All fuzz binaries also compile under the installed nightly with the refreshed
locked workspace packages. The sandbox build uses per-command
`CCACHE_DISABLE=1 CXX=/usr/bin/g++ CC=/usr/bin/gcc` to avoid writing the global
compiler cache; no global toolchain or compiler-cache settings were changed.

## Honest remaining gates

There is no complete project identity transcript/digest API yet (Stage 7).
Root invariance is proven for complete capture/IR/serialized identity inputs and
the public-interface fingerprint, not for an unimplemented project hash.
There is also no project incremental cache: replay and changed-unit recapture
equivalence do not claim cached execution. Those two conjunctive checklist items
remain open. No release tag, approval, version bump, or promotion was performed.
The single-file CLI remains unavailable for the complete v1 profile. Updated
IR/probe fuzz targets include project decoding; no full campaign is claimed here.
