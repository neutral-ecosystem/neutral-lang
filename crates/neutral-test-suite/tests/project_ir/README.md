<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete project IR conformance

This suite checks the compiler/IR/independent-reader boundary and post-compilation
public selections. Its fixture copies remain executable when the active portable
is archived. It owns complete/private/disconnected projects, contextual values
and defaults, source/provenance accounting, malformed in-process artifacts,
deterministic ordering, bounded failures, and public dependency-closure checks.
Encoded project transport and standalone probe publication belong to later gates.

Run `cargo test --package neutral-test-suite project_ir`.
