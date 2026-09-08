<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 consumer-deliverable verification

Review date: 2026-09-08. Result: pass for Stage 10 Step 7.

## Source and build identity

Verification began from clean checked-out `main` revision
`088541693a9382bd1d00abd3ce32437ed496ed7e`. `cargo xtask build --profile
release` built the complete workspace, including `neutral-cli`,
`neutral-probe`, compiler/reader libraries, external encoding, and the reference
formatter. `cargo xtask docs` generated the workspace index and all crate
documentation. `cargo test --workspace --doc` passed every public documentation
test.

## Released CLI boundary

The release-mode CLI validated, compiled, and formatted the frozen
`positive/syntax/minimal-core.neu` fixture. The formatted source validated again
and the successful external artifact had SHA-256
`f48a6182544195c9f47d1e4d8f5f3f8609cc9680b065b4917251a5848710cae6`.

The released boundary then exercised these fail-closed paths:

- invalid identifier: `NEU-NAME-001`, with no output artifact;
- pre-start cancellation: `cancelled`, with no output artifact;
- one-byte source limit: `input-limit-exceeded`, with no output artifact;
- mismatched explicit vocabulary digest: `NEU-VOC-002`, with no output
  artifact; and
- existing destination without overwrite: `output-exists`, preserving the
  exact pre-existing bytes and leaving no temporary output.

## Reader and probe equivalence

The successful compilation was inspected by the release standalone probe. The
focused `neutral-test-suite` system test reconstructed equivalent reader views
from the encoded compilation. The focused `neutral-probe` system test compared
the in-process decoder, probe-library summary, rendered lines, and standalone
executable output and required exact equality modulo the producer/envelope
metadata excluded by the logical-equivalence contract.

For the clean-consumer proof, copies of only the two release binaries were run
from `/tmp/neutral-step7.IXoRVc`, outside the workspace. Explicit copied source
and vocabulary files produced an artifact with SHA-256
`c50d8cc2c35eac42aa7369e0423e28f6e280aa1601e41ea7d6e2709c8734e0ae`.
The copied standalone probe validated and summarized it without a Cargo
workspace, compiler linkage, cache, or ambient vocabulary lookup.

## Standalone dependency boundary

`cargo tree --locked --package neutral-probe --edges normal` resolved only the
probe, encoding, reader, IR, vocabulary, core, and reviewed SHA-256 utility
closure. It contained no `neutral-compiler`, CLI, test-suite, test-support,
frontend, or host-I/O package. The executable boundary and the automated
workspace dependency allowlist both passed.

## Documentation inspection

The generated `target/doc/index.html` workspace page and the `neutral_cli`,
`neutral_probe`, and `neutral_reader` crate pages were present and nonempty.
The index contained the Cargo-derived package inventory; crate pages contained
the generated `Workspace index` return control. Inline code and Rustdoc source
controls remained available through the generated theme assets.

The released CLI, reader, encoding, and probe boundaries therefore match the
frozen v0 API, diagnostic, compatibility, effect, and exclusion contracts for
the Step 7 consumer scenarios.
