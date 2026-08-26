<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-cli

`neutral-cli` is the host-facing command-line adapter for capture, compilation,
validation, and formatting workflows.

It may own filesystem and process interactions required by a local command, but
it delegates pure compilation to `neutral-compiler` and artifact reading to
`neutral-reader`. It is not the independent probe artifact and must not make
host effects part of the captured compiler contract.
