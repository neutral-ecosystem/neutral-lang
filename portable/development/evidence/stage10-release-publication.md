<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 release approval and publication

Review date: 2026-09-08. Current result: approvals and publication automation
pass; remote publication and roadmap archival are pending.

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

No release URL or published digest is recorded yet. Those identities may be
added only after the final tracked changes are committed, the clean final
`main` HEAD passes `cargo xtask release prepare`, the stale local tag is signed
again at that exact commit, and GitHub confirms the pushed release assets. The
v0 roadmap archive must then be landed and identified before v1 is initialized.
