<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete project IR conformance

This suite checks the compiler/IR/independent-reader boundary and post-compilation
public selections. Its fixture copies remain executable when the active portable
is archived. It owns complete/private/disconnected projects, contextual values
and defaults, source/provenance accounting, malformed in-process artifacts,
deterministic ordering, bounded failures, and public dependency-closure checks.
Encoded round-trips cover complete/private/contextual/vocabulary content and
all companions. Captured replay, changed-unit reconstruction, serialization
order, root invariance, and failure envelopes exercise the public boundary.
Hostile codec tests belong to `neutral-encoding`; compiler-free standalone
executable tests belong to `neutral-probe`.

Run `cargo test --package neutral-test-suite project_ir`.
