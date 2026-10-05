<!-- SPDX-License-Identifier: Apache-2.0 -->

# Project identity contracts

This module owns the versioned captured, complete logical, derivation, and
artifact transcript schemas. Framing performs checked, cancellation-aware,
bounded writes; layer modules select only their governing inputs. It does not
parse source, validate project semantics, acquire inputs, or implement caching.
Compiler capture adapters and future validated-reader APIs consume this boundary.
Run `cargo test --package neutral-test-suite project_identity`.
