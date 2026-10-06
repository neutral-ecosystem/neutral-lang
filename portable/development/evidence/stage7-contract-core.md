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

## Standalone composition catalogue/value slice (06-10-2026)

This slice does not extend the frozen project identity profile. Eight literal
positive/negative/boundary/migration catalogue inputs and acceptance/error oracles
are registered with SHA-256 byte pins and runtime-owned copies. Supplied closed
values reuse the default/type/restriction validator, including heterogeneous
variant payloads. Independent materialization-node, work, depth, list, string-byte,
numeric-digit and scale limits reject without partial results. Optional absence,
explicit null, defaults and supplied occurrences remain separate safe origin
facts; equivalent materialized values compare equal without equating origins.

The compiler-independent reader exposes public materialization and bounded
reference-type paths through lists, nullable wrappers and every variant alternative.
Private/missing root types share a safe unavailable classification. These are type
dependencies and occurrence classifications, not source binding edges, source-map
spans, a serialized project, or independently verified new logical identities.

Validation: `cargo xtask test all` passes 638 Nextest tests (zero skips) plus the
compiler compile-fail doctest. `cargo xtask ci pr`, strict workspace Clippy,
`cargo xtask fixtures check` (59 fixture/oracle entries), and portable verification
pass. The vocabulary fuzz harness additionally exercises bounded supplied-value
probes on accepted catalogues and builds with `CCACHE_DISABLE=1`; no full fuzz
campaign or allocation-fault gate is claimed by this slice. The existing public
semantics hash was synchronized with the already accepted trailing-comma change.

Still required: full source variant/composition syntax and semantics, non-null
reference-value edges, complete project IR/provenance/source attribution, new codec
and independent probe/view integration, frozen new identity transcripts and literal
independent vectors, actual clean/cache equivalence, and complete hostile/allocation
validation. The full extension and promotion-conformance checklist remains open;
creating a package tag does not substitute for these requirements.

## Composition contract and literal registration (06-10-2026)

This closes only the two contract-freeze and fixture/vector-registration items.
The [schema/default/restriction contract](../../specs/contracts/VOCABULARY-COMPOSITION.md),
[shared variant rules](../../specs/contracts/VARIANTS.md), and
[composition profile](../../specs/contracts/COMPOSITION-PROFILE.md) are frozen
with exact file hashes. Explicit capture /2 features select successor project
IR/result/view/identity profiles; no package version or failed old parse selects
them. Compatibility, source diagnostic precedence/recovery, safe attribution,
public/embedded/reference closure and independent limit counting are specified.
The successor binary layout is a requirement, not a codec availability claim.

The manifest adds five frozen fixture records: four request families and one
identity corpus. Together they register 26 literal positive/negative/boundary/
migration cases with exact embedded source/bundle byte digests and transitive
locks. Cases cover both variant origins, imported variants, heterogeneous values,
composed list/nullable/reference types, default/absence behavior, source error
classifications, diamond/missing/extra/conflicting/private/cyclic dependencies,
embedded recursion, executable members, exact/one-over alternatives and edges,
and explicit old-schema adapter/relabel rejection. Catalogue outcomes execute
against the existing standalone validator; future source outcomes remain frozen
requirements, not passing compiler tests.

The new identity corpus freezes five complete captured/logical/derivation/project-
artifact/view-artifact transcript byte strings and SHA-256 digests, twelve
semantic/control variations and eight literal rejection vectors. Variations
include unused defaults, restrictions, exact fractional/sign/scale choices,
optional absence versus null, selected variants, heterogeneous list order,
visibility, nested list/nullable/reference contracts, canonical dependencies and
exact revisions. Roots/presentation/occurrence facts are excluded from complete
meaning. The test-only Python oracle extends only unchanged independent /1
primitives; a separate Rust length-prefix walker/hash check verifies the literal
frames/digests. Tests do not generate or bless expected outputs. /1 vectors are
unchanged and protected by their original byte digest.

All runtime copies live under the owning test crate, without portable-path reads.
The separate oracle also pins each copied fixture's file digest, preventing
runtime/plan drift after archival. Missing Python fails instead of skipping.
Independent exact-byte/frame boundaries, zero controls, cancellation, normalized
numbers, canonical ordering and invalid reference targets are tested.

Validation: `cargo xtask test all` and standard `cargo xtask ci pr` pass 646
Nextest tests, zero skipped, plus the compile-fail doctest. Strict Clippy,
formatting, repository link/ownership checks, portable verification and
`cargo xtask fixtures check` pass with 64 registered fixture/oracle pairs.
Contract, reference-oracle, vector, manifest and review hashes are retained in
`freeze.toml`; routine command logs remain ignored generated output.

Activation remains blocked on full source/compiler/project/IR/codec/independent
reader/probe/view implementation, production comparison to every new vector and
oracle, and hostile decoder/allocation-fault/fuzz/clean-cache validation. The
new suite remains `frozen`, not `required`. Before activation, verify its existing
pins and all production comparisons; synchronization is not permission to rewrite
accepted expectations. No version, release, tag, commit or push was performed.
