<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 7 contract, vectors, and identity integration

Reviewed: 05-10-2026. Scope: contract/vector, core, integration, and validation gates; no package
version bump, tag, approval, or release is performed.

The [identity contract](../../specs/contracts/PROJECT-IDENTITY.md) is pinned in
[freeze.toml](../../specs/contracts/freeze.toml). The
[conformance manifest](../../conformance/manifest.toml) pins six literal complete
NHT transcript/SHA-256 vectors and their oracle. Crate-owned vector copies remain
executable after the portable plan is archived.

## Contract decisions and ownership

- `neutral-core` owns the existing shared NHT envelope and SHA-256 primitive.
  `neutral-ir::project_identity` owns the frozen profile, four separate domain
  tags, typed identities, and exact framing/field order. Package release numbers
  do not select identity behavior; changing the frozen schema requires new
  domains/profile and new vectors, not overwriting accepted expectations.
- Captured identity covers every exact source and vocabulary lock, including
  disconnected members, byte digests/lengths, and source IDs. Capture permutation,
  correlation keys, host mappings, controls, and roots are excluded. The compiler
  adapter projects already validated capture facts without I/O or recapture.
- Complete logical identity covers private declarations, disconnected modules,
  canonical signatures/values/defaults, imports, locked semantic vocabulary
  schemas, and public dependency topology. It excludes source/byte evidence,
  aliases, graph-local labels, host mappings, roots, and processing companions.
  Exact numbers and decoded strings normalize; list order and inert location
  types remain significant. Public fingerprints are not complete project hashes.
- Derivation adds exact captured identity, explicit producer identity/revision,
  and acceptance/producer limits. Artifact identity adds explicit kind, format,
  normalized view roots, and options. Artifact identity is not a checksum of
  arbitrary noncanonical wire bytes. These digests confer no authority.
- Construction uses independent intersected byte/node limits, shared type/value
  depth bounds, bounded comparison keys, checked arithmetic, fallible temporary
  reservations, in-place frames, and cancellation before publication. Errors
  return `Limit`, `InvalidInput`, or `Cancelled`, never a partial transcript.
  Structural identity construction does not replace semantic reader validation
  or validation of a view's root existence/publicness.

## Initial contract/core checks

| Command | Outcome |
| --- | --- |
| `cargo xtask fixtures sync` / `cargo xtask fixtures check` | 51 fixture/oracle pairs registered; literal vector and contract digests pinned |
| `cargo test --package neutral-test-suite project_identity` | 13 identity tests pass; all six literal transcripts and SHA-256 digests match exactly |
| `cargo test --package neutral-core` | Complete-transcript hashing preserves existing NHT framing; all foundational tests pass |
| Strict Clippy over core, IR, compiler, and cross-package tests | Pass; no lint suppression introduced |
| `cargo xtask ci pr` | Pass: 465 workspace tests, zero skips, doc tests, strict lint, dependency/hygiene checks, smoke, standalone probe build, and generated docs |

Identity tests additionally cover vocabulary permutation/alias/byte-evidence
separation, producer/control changes, root normalization, private-content
non-equivalence, all four layers at exact byte/node boundaries and one below,
zero budgets, duplicate/malformed context, aggregate import bounds, depth, and
cancellation. Routine toolchain/command results stay in ignored workflow logs,
not separate Markdown reports.

## Public integration and validation review

The crate-owned [independent encoder](../../../crates/neutral-test-suite/tests/project_identity/reference.py)
uses Python standard-library big-endian packing and SHA-256. It does not import
Rust framing, transcripts, or computed expectations. The test adapter projects
typed input facts only. All six unchanged literal byte/hash baselines agree;
the complete/private/cyclic/vocabulary/reuse/location corpus and typed-value
cases also agree. Separate malformed capture, canonical order, derivation,
artifact option/root, cancellation, depth, exact-byte/node, and one-over-work
vectors compare acceptance or failure classifications. Framed tuple boundaries
distinguish delimiter-free collision paths. Missing Python fails, never skips.

`ValidatedProject` exposes complete logical transcripts and a context-bound set
of typed captured/logical/derivation identities with processing facts. Explicit
capture companions must match the validated IR; producer and lock claims remain
caller inputs, not authentication or proof of faithful compilation. Producer
text is bounded before ownership. Artifact construction validates public/existing
roots before publication. Empty, subset, full, and permuted root selections
leave all complete upstream bytes/digests unchanged. The standalone reader-only
probe computes the same complete logical digest without compiler linkage.

The compiler's caller-owned syntax cache executes real parse reuse, with parsed,
reused, and rejected unit counts. Source-sensitive syntax keys use exact source
digest **and bytes**, module, and profile; logical identity/public fingerprints
never substitute for those keys. Private source changes reparse the affected
unit. Changed source IDs, capture order, vocabulary bytes/visibility, and controls
always reconstruct current graph, semantics, source maps, provenance, accounting,
and complete output. Invalid runs preserve the prior generation. Tests compare
full IR, derivation/logical transcripts, diagnostics, and encoded artifacts with
clean compilation, including shuffled concurrent schedules, stale/misplaced
entries, simulated digest collisions, and retention bounds. There is no external
cache import, persistent cache, shared global cache, or final-artifact memoization.

Immutability review: the normative contract, profile, four domain tags, framing,
SHA-256 usage, and six literal vectors are unchanged. Package versions do not
select identity behavior. A future semantic change must introduce a new identity
profile/domain and new vectors; neither expected digests nor existing versioned
transcripts may be rewritten to bless incompatible behavior. Existing document
identity and wire contracts are unchanged.

| Final command | Outcome |
| --- | --- |
| `cargo test --package neutral-test-suite project_identity` | 31 tests pass, including the independent encoder and public integration |
| `cargo test --package neutral-compiler cache` | Five cache tests pass, including collision/context rejection and exact/over retention bounds |
| `cargo xtask fixtures sync` / `cargo xtask fixtures check` | Oracle now covers V1-ID-001..005; literal vector bytes are unchanged |
| `cargo xtask ci pr` | 495 workspace tests pass, zero skips; strict Clippy, doc tests, contract/hygiene checks, command smoke, standalone probe build, and docs pass |

Release promotion confirmation, release-quality approval, and the next release
remain separate unchecked tasks. No version, tag, commit, or publication was changed.
