<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 candidate-preparation record

Status: superseded historical RC record; not a published release.

Qualification now selects the clean checked-out `main` `HEAD`. This historical
record is retained for auditability but is no longer a configured release input.

This record identifies the clean `main` revision approved for Stage 10
workflow work. It is a candidate record, not a release claim: final publication
remains blocked on the remaining Stage 10 qualification and approval gates.

## Candidate source identity

| Field | Value |
| --- | --- |
| Branch | `main` |
| Full Git revision | `3e8fa5d69769a45e2c89b143a023b6824850090c` |
| Revision subject | `[ADD] allocation profiler test` |
| Worktree at inspection | clean |
| Annotated candidate tag | `v0.1.0-rc.1` |
| `Cargo.lock` SHA-256 | `c55a9e7bfa8ad3b89459326c68d3313772ca368c82001351f12ec8cc37776aef` |
| Fixture manifest SHA-256 | `f05752b28c7e1bc1d58d5a2200c85c58837e93932ec07c9db457e01b564075fe` |
| Fixture-oracle review SHA-256 | `4ed8ff3a2429c6f2c795b5cfbe2c129d662bab27ae81373e2d1aedc2c0fa09de` |

## Toolchain identity

`rust-toolchain.toml` SHA-256:
`0c9b811689c2817b6404b2858b3e8f7aebf513af379da94774521025b7008863`.

```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
profile = "minimal"
```

The release run must additionally record the exact resolved stable compiler,
host, and environment manifest; the rolling `stable` channel alone is not a
complete reproducibility identity.

## Freeze reconfirmation

The approved `neutral-v0-0.1.0-encoding-0.1-portable-layout-2026-09-06` freeze
manifest was rechecked without modification. Its governing revisions are:

| Contract | Revision |
| --- | --- |
| Language behavior, logical IR schema, source map, provenance, derivation | `0.1.0` |
| Vocabulary schema and bundle encoding | `0.1.0` |
| Digest transcript, compiler API, reader API, resolver API | `0.1.0` |
| Diagnostic catalogue and structural limits profile | `0.1.0` |
| External IR encoding | `NIR-CBOR/0.1` |

`cargo xtask traceability check` passed while preparing this record.

## Recorded release-authority decisions

- [x] The sole maintainer approved [the Stage 9 residual-risk record](../../../quality/residual-risks.md)
      on 2026-09-08 for candidate preparation.
- [x] The v0 distribution scope is an annotated source tag and GitHub binary
      assets. crates.io publication is not selected.
- [x] The local annotated tag `v0.1.0-rc.1` names the source identity above.

The tag has not been pushed, and no package publication, GitHub release, binary
upload, or version-metadata change is authorized by this preparation record.
