<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 5 contract, core, and public integration

Status: `v0.5.1` contract/fixture gate and `v0.5.2` core gate implemented;
two of three `v0.5.3` integration checks implemented. The authoring metadata
presentation check awaits the separate Stage 8 input/schema. Stage 5.4 schema,
fixture, inert-location, limit, and vocabulary-fuzz checks pass. Release
promotion remains separate.

The [vocabulary contract](../../specs/contracts/VOCABULARY.md) freezes repeated
aliased requirements, exact lock cover, closed v1 project-bundle schema,
source-authorable/public type closure, metadata separation, and inert
`url`/`path` text. The exact Stage 5 requests and expected outcomes are pinned
in the [conformance manifest](../../conformance/manifest.toml) and
[Stage 5 oracle](../../conformance/oracles/stage5/vocabulary-and-locations.toml).
The already-pinned Stage 2 capture fixtures/oracles remain authoritative for
missing, extra, and conflicting locks, avoiding duplicate mutable fixtures.

Core project validation checks exact bundle bytes against locks, the complete
closed JSON envelope, duplicate/unknown/executable members, public type
closure, and embedded nominal cycles before publishing any semantic model.
Captured vocabulary contracts are ordered by canonical identity, while source
aliases stay module-local. Qualified source signatures may name only public
vocabulary types and public interface signatures retain canonical identity and
locked semantic release. A direct `url` or `path` binding requires a quoted
literal and retains its exact decoded text in a distinct IR variant. No URL or
path lookup, normalization, authorization, or acquisition API is present.

This is the internal captured-project semantic path. The standalone v1 profile
remains unavailable, and the complete contextual value graph/project artifact
is Stage 6 work. Stage 5.3 carries a canonical, public-only catalogue of every
locked vocabulary in the project interface. Catalogue facts participate in its
fingerprint. The compiler excludes aliases and private type names; the
independent reader checks canonical ordering, release syntax, public type
membership, signature-to-lock closure, and fingerprint. Alias renaming yields
an identical complete interface. Inline metadata still fails the closed
semantic bundle schema. There is no separate authoring metadata input or
presentation projection yet, so a presentation-only effect cannot be tested
until Stage 8 defines that input. Stage 5.4 executes crate-owned copies of
all four pinned vocabulary/location requests against their oracle outcomes;
their SHA-256 values match the reviewed fixture manifest, while `portable/`
remains removable. The crate-owned oracle changes only fixture paths. Strict
v1 tests reject duplicate, unknown,
and executable members, all truncation boundaries, invalid UTF-8, BOM,
oversized strings, and over-limit structural input. Location tests accept an
invalid URL and a nonexistent opaque path without lookup or normalization.
Static source and normal-dependency inspection found no filesystem or network
acquisition path in the vocabulary, compiler semantic, IR, or reader boundary.

A seeded address-sanitized vocabulary fuzz run exposed stack exhaustion in the
recursive JSON parser when the host supplied an extremely large depth. The
effective vocabulary nesting limit is now capped at the named stack-safe
ceiling, and the saved crash input replays successfully after the fix. The
seeded v1 vocabulary target then completed its full configured 900-second
address-sanitized rerun with 22,329,752 executions, no new crash, and 682 MiB
peak RSS. This runner is ptraced, so LeakSanitizer cannot run; the rerun used
`LSAN_OPTIONS=detect_leaks=0` while retaining AddressSanitizer. Leak checking
on an untraced runner remains a separate review item.

Verified on 2026-10-02:

```text
cargo test --workspace --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask fixtures check
cargo xtask portable verify
cargo xtask ci pr
cargo xtask test conformance
cargo xtask test security
cargo xtask fuzz smoke
```

The Stage 5 TOML requests were also parsed and each `bundle_utf8` SHA-256
checked against its exact lock. The direct compiler tests cover multiple
aliases, identical complete public interfaces after alias renaming,
source-inaccessible private types, executable members, embedded recursion,
and distinct inert locations. Independent reader tests cover missing,
misversioned, inaccessible, duplicate, and unordered vocabulary facts, plus
fingerprint tampering. Inline authoring metadata is rejected by the semantic
bundle schema. The composed CI profile passed with its ignored workflow log
at `test-results/workflows/ci/pr/run-2-31`.
