<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 release approval and publication

Review date: 2026-09-08. Current result: approval, qualification, and draft
assembly pass; publication and roadmap archival are pending.

## Approval

Younes Rabeh, the sole maintainer and release owner, fills the technical,
test/quality, security, release, and standards review roles for v0.1.0. The
staffing exception and its compensating automated, adversarial, fuzz, mutation,
coverage, memory, reproducibility, and standalone-consumer reviews are recorded
in [`../05-RELEASE.md`](../05-RELEASE.md). No independent-review or standards
certification claim is made.

## Publication controls

The tag workflow uses the same stable `cargo xtask release prepare` command as
local qualification. For a pushed `v*` tag it checks out `main` with complete
tag history, proves that the tag dereferences to the exact checked-out `main`
HEAD, assembles the release from that clean branch, and verifies `SHA256SUMS`
before artifact transfer. A separate tag-only job downloads the package,
verifies it again, and creates the GitHub release.

Repository permissions default to `contents: read`. Only the publication job,
which cannot run for manual dispatch or pull-request execution, receives
`contents: write`. The ordinary push CI remains read-only and has no
pull-request trigger or release credential.

## Local validation

The workflow parses as YAML. `cargo fmt --all --check`, the 29 `xtask` unit and
documentation tests, and `cargo xtask check` pass with the strengthened
executable workflow contract.

## Draft identities

The signed annotated `v0.1.0` tag dereferences to qualified clean `main` commit
`ceb439b5af0644682c3be95f6d9593a86b5de796`. GitHub Actions run
[`34245962106`](https://github.com/neutral-ecosystem/neutral-lang/actions/runs/34245962106)
passed both qualification and remote asset assembly. The non-prerelease draft
is titled `neutral-lang v0.1.0` and is visible to authorized maintainers at
[`v0.1.0`](https://github.com/neutral-ecosystem/neutral-lang/releases/tag/untagged-65229ac5ced4cf1f59f2).

An isolated download of all draft assets passed the draft `SHA256SUMS`. The
draft payload digests are:

| Asset | SHA-256 |
| --- | --- |
| `INSTALL.md` | `589478314b89cf4a14bca7be80a69447b8edde3b2b2524ff3001f8dd1a21b497` |
| `LICENSE` | `4cc43ca60ec44fff6aa013a9801284639a69ced19b9d6f262bf44e66678ebd63` |
| `README.md` | `1079588a26d12788b6fa10aec9e314ebe3566e46a95eb832e1ce2d44ac874fbe` |
| `SBOM-Cargo.lock` | `300bc9e55dd409d35f4b747da774c73200035aa37689102f0ae4c332b7058c49` |
| `neutral-cli` | `ae6f32ad45000496d88ba30ee585966128c94a6da40c25bfdae0e2f9d9feb41f` |
| `neutral-lang-v0.1.0-source.tar` | `47e80065f9408408e898f1a679ec2a950d367b02158bca1369a4902dddc4f33a` |
| `neutral-probe` | `a9e6773bf7a615f9b032794915ee3fc1c1c1d3a618ce991f081101d5c83cd651` |
| `package-summary.json` | `6ea3b2630d1ea5db74ae461280af14dcc75d52983180e16d5ee2699fc105e8e9` |
| `provenance.json` | `9e6f361cb12ed11e92e1e0901dc9acdfea8cf8cda556223a0d9ad3c766e46155` |
| `release-manifest.json` | `4fdd347b10a1985c85b51b93396b6b185a268c97de277b9c4d067967d0f8f9e6` |
| `SHA256SUMS` | `0708ca80cbff8223e481c77fd53614341a6814dca3ed537f9a2a8b5f563e4149` |

The v0 roadmap archive must still be landed and identified before v1 is
initialized.
