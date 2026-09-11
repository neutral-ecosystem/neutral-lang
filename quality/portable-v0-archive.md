<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 portable archive evidence

## Outcome

The completed Neutral v0 portable package is archived immutably in
`neutral-roadmap`. The implementation repository no longer requires a root
`portable/` directory for builds, tests, documentation, packaging, or quality
checks. Released executable inputs live under
`conformance/releases/v0.1.0/`; future planning packages may independently use
the root `portable/` path.

## Archive identity

- Release: `v0.1.0`
- Portable tree SHA-256:
  `595e76668efc92b56060857ed9c7d84f4dfb9722e667356967e0f4db76cfcf38`
- Archive path: `neutral-lang/v0/portable`
- Archive manifest: `neutral-lang/v0/portable.manifest.sha256`
- Archive metadata: `neutral-lang/v0/portable.archive.json`

## Promotion boundary

The specifications, fixtures, vocabulary bundles, expected oracles, and
conformance manifests needed by maintained code were promoted to
`conformance/releases/v0.1.0/`. Source code and tests reference that release
bundle instead of the active-planning path. Repository policy rejects
production-crate dependencies on `portable/`.

Planning history, progress tracking, and release notes were not promoted into
the executable conformance bundle. They remain in the immutable roadmap
archive.

## Validation

The following checks passed on 2026-09-11:

1. `cargo test --workspace --lib --bins --tests` with no root `portable/`.
2. `cargo xtask check` in a clean staged copy with no root `portable/`.
3. `cargo xtask quality --profile pr` in that clean staged copy.
4. `cargo xtask docs` in that clean staged copy.
5. `cargo xtask portable install <directory>` atomically installed and verified
   the archived package as a simulated `v1` active package while workspace
   packages remained at `0.1.0`.
6. An invalid lifecycle-status simulation was rejected, retained under ignored
   generated evidence, and left the root `portable/` path absent.
7. `cargo xtask check` passed in a clean checkout both without `portable/` and
   with the simulated `v1` package installed.

The simulated package was moved out of the repository after validation. The
root `portable/` directory is therefore absent and reserved for the next
reviewed development package.
