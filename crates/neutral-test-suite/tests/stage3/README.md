<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 3 graph validation

This suite owns executable cross-package checks for module/import graph
construction from complete captured requests. It compares the reviewed fixture
corpus with its graph and diagnostic oracles, then repeats every case with
shuffled source submissions and concurrent graph builds. Additional probes
cover diagnostic limits, recovery, ambiguity, and excluded acquisition forms.

The fixture and oracle files are exact crate-owned copies of the active
portable Stage 3 corpus. Keeping executable copies here lets normal tests run
after a finished portable plan is archived or removed. The source plan remains
the normative authority while active; `cargo xtask fixtures check` verifies its
registered hashes.
