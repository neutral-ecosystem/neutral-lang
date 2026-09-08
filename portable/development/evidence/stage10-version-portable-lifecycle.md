<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 version and portable lifecycle evidence

Review date: 2026-09-08. Status: Step 3 implementation complete.

## Version and dependency ownership

- Root `[workspace.package].version` is the sole package-release version and
  every member manifest uses `version.workspace = true`.
- Release tag identity is derived as `v<workspace package version>`; release
  configuration contains no duplicated version, commit, or evidence path.
- `version show` reports package identity separately from every entry in the
  approved freeze's `contract_versions` section.
- `version check` validates the root release lock with locked offline Cargo
  metadata, workspace package entries, registry checksums, and the declared
  no-Git/no-external-path policy. The separate cargo-fuzz lock is explicitly
  non-release tooling.
- The shared `sha2` requirement is declared once in `[workspace.dependencies]`;
  production digest contracts and snapshot automation inherit it independently.
- `version prepare` validates forward SemVer precedence and emits a deterministic
  review-required JSON plan under ignored results. It changes no tracked file,
  tag, frozen version, or normative digest.

## Generated and portable ownership

`config/generated-outputs.toml` inventories Rustdoc, coverage, fuzz, mutation,
benchmark, package, SBOM, release, and portable-snapshot outputs with an owner,
source command, validation command, and ignored path.

`portable/lifecycle.toml` identifies v0 as the active series, the immutable
roadmap destination, the future v1 source, and the required local redirect.
`cargo xtask portable verify` checks active/package version agreement, required
roots, local Markdown links, manifest fixture/oracle ownership, frozen fixture
manifest digests, archived-input exclusions, and any retained snapshot bytes.
`cargo xtask portable snapshot` creates an atomic copy plus sorted per-file
SHA-256 manifest and migration report beneath ignored, tree-digest-addressed
results. It performs no network or external-repository write.

## Executed evidence

- `cargo fmt --all -- --check`
- `cargo clippy -p xtask --all-targets --all-features -- -D warnings`
- `cargo test -p xtask --all-targets`
- `cargo xtask version show`
- `cargo xtask version check`
- `cargo xtask version prepare 0.2.0` — review plan generated
- `cargo xtask version prepare 0.1.0` — correctly rejected as non-forward
- `cargo xtask portable verify`
- `cargo xtask portable snapshot`
- `cargo xtask check`
- `cargo xtask quality --profile pr`

All generated outputs remain ignored. The previously signed local `v0.1.0` tag
is not moved by this work; it consequently does not identify Step 3 changes.
