<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 7 contract, literal vectors, and core identity boundary

Reviewed: 05-10-2026. Scope: contract/vector and core gates only; no package
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

## Retained checks

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

## Remaining gates

Literal baselines were assembled outside the Rust implementation using explicit
NHT framing and standard SHA-256, not regenerated from Rust output during tests.
This does not claim a retained second full implementation or complete
adversarial-vector reproducibility review. Validated-reader/probe identity APIs,
correct cache-layer integration, actual incremental/cache-versus-clean execution
with stale-cache rejection, and final identity immutability review remain
unchecked Stage 7.3/7.4 work. The existing v0 identities and transport are unchanged.
