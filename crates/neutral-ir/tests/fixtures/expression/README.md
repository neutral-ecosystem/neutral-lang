<!-- SPDX-License-Identifier: Apache-2.0 -->

# Expression identity fixtures

Runtime-owned literal copies of the frozen expression identity vectors and exact
source bytes. `identity-vectors.json` retains the complete static identity
partitions from `portable/specs/fixtures/expressions/vectors.json`;
`complete-identity-vectors.json` retains the residual/template identity partitions
and all controlled variation digests from `complete-vectors.json` beside it.
Only the oracle fields used by these owning tests are copied. Transcript bytes,
expected digests and source bytes are unchanged; they are not generated or blessed
by the Rust identity implementation. The portable corpus is not a runtime include
dependency and remains untouched.
