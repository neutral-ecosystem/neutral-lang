<!-- SPDX-License-Identifier: Apache-2.0 -->

# IR source layout

Start at [lib.rs](lib.rs): it preserves the crate-root public imports. Private
document modules separate symbols, numbers, values, equality, fingerprints and
source/processing companions. Do not add compiler state, host I/O or validation
authority to these data structures.

| Area | Responsibility |
| --- | --- |
| [profile.rs](profile.rs), [language.rs](language.rs) | Frozen document selectors and shared source spellings; neither follows Cargo versions. |
| [project.rs](project.rs), [project_interface.rs](project_interface.rs) | Complete historical project data versus the narrow exported signature surface. |
| [composition](composition/README.md) | Shared closed defaults, resolved values, record/variant contracts and composition-project data. |
| [expression](expression/README.md) | Raw typed residual trees, inputs, templates and expression-project companions; not an evaluator. |
| [project_identity](project_identity/README.md) | Bounded, domain-separated captured/logical/derivation/artifact transcripts. |

Folders with child modules use `mod.rs` as their entry point. Keep module paths
stable when moving files, and keep tests in [the owning tests tree](../tests/README.md).
Run `cargo xtask test all`; use `cargo xtask ci pr` for repository checks and docs too.
