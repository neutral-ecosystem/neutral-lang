<!-- SPDX-License-Identifier: Apache-2.0 -->

# Project identity contracts

This module owns the versioned captured, complete logical, derivation, and
artifact transcript schemas. Framing performs checked, cancellation-aware,
bounded writes; layer modules select only their governing inputs. It does not
parse source, validate project semantics, acquire inputs, or implement caching.
Compiler capture adapters and independent validated-reader APIs consume this boundary.

| Module | Inputs governed by this layer |
| --- | --- |
| [captured.rs](captured.rs) | Exact supplied source/vocabulary closure, not just interpreted meaning. |
| [logical.rs](logical.rs) | Complete historical project meaning; excludes roots and host/source evidence. |
| [composition.rs](composition.rs) | Separately selected composition logical and public-interface contracts. |
| [derived.rs](derived.rs) | Logical/captured identities plus producer controls, artifact kind/format and selection. |
| [framing.rs](framing.rs) | Shared checked NHT framing, canonical ordering and fallible retention. |

Keep frozen domains/profile IDs explicit and distinct: identical digest storage
does not make identity layers interchangeable. Reuse the framing writer, not a
different layer's transcript or package version. `IdentityTranscript::bytes`
returns the exact hashed bytes, not a serialization of its digest.

Run `cargo xtask test all` for independent vectors and reader comparisons. Run
`cargo nextest run --config-file config/nextest.toml --package neutral-ir
-E 'test(framing)'` for the local framing checks.
