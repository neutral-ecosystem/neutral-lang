<!-- SPDX-License-Identifier: Apache-2.0 -->

# Active portable lifecycle

`portable/` is the active execution package for the release series declared in
`../lifecycle.toml`. It owns the current plan, contracts, decisions, fixtures,
oracles, progress, and retained review evidence. Production crates and tests may
read this active package, but never an archived roadmap copy.

## Snapshot

Run `cargo xtask portable verify`, then `cargo xtask portable snapshot`. The
snapshot command copies the complete active package into an ignored,
digest-named directory under `test-results/portable/snapshot/`. Its sorted
SHA-256 manifest identifies every byte and its migration report records the
intended roadmap destination. It does not modify or contact another repository.

## v0 archive and v1 rollover

After v0 release evidence and approvals are immutable:

1. Verify and snapshot the clean v0 package.
2. Review the snapshot manifest and land its `content/portable/` tree as a
   separate immutable commit at `neutral-lang/v0/portable` in
   `neutral-ecosystem/neutral-roadmap`.
3. Replace this repository's active `portable/` package from the reviewed
   roadmap path `neutral-lang/v1`; do not merge v0 and v1 planning files.
4. Preserve a small `PORTABLE-ARCHIVE.md` redirect naming the v0 archive commit,
   snapshot digest, release tag, and immutable archive URL.
5. Set the new `lifecycle.toml` to `active_series = "v1"`, adapt repository
   evidence locally, and run `cargo xtask portable verify` before implementation.

Archived material is historical evidence only. It must not become a Cargo
dependency, fixture source, mutable CI input, or fallback for a missing active
file. Contract digest changes remain subject to the contract-freeze process and
are never performed by version or snapshot commands.
