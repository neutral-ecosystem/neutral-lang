<!-- SPDX-License-Identifier: Apache-2.0 -->

# IR source layout

Start at [lib.rs](lib.rs). The crate root exports shared scalars, values,
nominal identities and checked source/provenance primitives. The current whole
project data model is [expression/project.rs](expression/project.rs); its
retained companions are declared in [project.rs](project.rs). Neither raw
construction nor identity framing implies semantic validity.

| Area | Responsibility |
| --- | --- |
| [language.rs](language.rs), [profile.rs](profile.rs) | Shared spellings, predicates and frozen profile selectors. |
| [value.rs](value.rs), [project_interface.rs](project_interface.rs) | Shared scalar/type/value primitives and exact public type contracts. |
| [composition](composition/README.md) | Shared closed values, record/variant contracts and composition bounds. |
| [expression](expression/README.md) | Current raw expression project, residual trees, templates and companions. |
| [project_identity](project_identity/README.md) | Bounded `/3` capture, logical, interface, derivation and artifact transcripts. |

Historical complete document/project models and their identity profiles have
been removed. Keep current literal transcript vectors with their independent
tests. Run `cargo xtask test all` for repository validation.
