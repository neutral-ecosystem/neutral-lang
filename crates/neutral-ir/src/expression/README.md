<!-- SPDX-License-Identifier: Apache-2.0 -->

# Raw expression contracts

[`mod.rs`](mod.rs) defines typed trees, binding bodies, selectors, lexical frames
and closed operators. A declared input is not a null value or a missing definition.
Both branches remain data; nothing here evaluates, acquires inputs or schedules work.

| Module | Responsibility |
| --- | --- |
| [profile.rs](profile.rs) | Exact semantic/artifact selectors, required features and independently bounded expression controls. Recognition is not availability. |
| [project.rs](project.rs) | Complete raw project data, declaration/body shape and claimed resource facts. |
| [templates.rs](templates.rs) | Non-callable public template contracts and finite-expansion controls. |
| [attribution.rs](attribution.rs) | Original-byte occurrences and generated-declaration lineage, separate from logical meaning. |
| [inspection](inspection/README.md) | Allocation-free borrowed shape and retention preflight. |
| [fallible](fallible/README.md) | Bounded, cancellation-aware copies of every retained branch. |

Use `Expression::try_clone_bounded` for hostile raw-tree retention. Shape checks
are not type checks: `neutral-semantics` checks supported semantic sections, while
complete artifacts still require reader/identity/companion validation. These raw
APIs do not activate successor source syntax, transport or evaluation.

Run `cargo xtask test all`, or `cargo nextest run --config-file config/nextest.toml
--package neutral-ir --test expression` for the raw-tree tests.
