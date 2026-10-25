<!-- SPDX-License-Identifier: Apache-2.0 -->

# Expression-project IR

[`mod.rs`](mod.rs) defines residual typed trees, binding bodies, selectors,
lexical frames, and closed operators. A declared input is not a null value or a
missing definition; both branches remain data until an explicitly supported
consumer handles them.

| Module | Responsibility |
| --- | --- |
| [profile.rs](profile.rs) | Current expression-project schema/profile selectors and expression limits. |
| [project.rs](project.rs) | Complete raw expression-project data, declarations, companions, and claimed resource facts. |
| [templates.rs](templates.rs) | Public template contracts and bounded expansion controls. |
| [attribution.rs](attribution.rs) | Original-byte occurrences and generated-declaration lineage. |
| [inspection](inspection/README.md) | Borrowed structural and retention preflight. |
| [fallible](fallible/README.md) | Bounded, cancellation-aware copies of retained trees and payloads. |

The compiler produces this raw model, `neutral-semantics` validates supported
expression meaning, `neutral-encoding` transports the complete `/3` artifact,
and the independent reader establishes validation authority. Structural
inspection or construction alone is not validation.

Run `cargo xtask test all`, or `cargo nextest run --config-file
config/nextest.toml --package neutral-ir --test expression` for raw expression
model tests.
