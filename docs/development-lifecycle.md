<!-- SPDX-License-Identifier: Apache-2.0 -->

# Development lifecycle

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

Neutral has one short daily loop. The active `portable/` package owns future
requirements and the checklist; the released `conformance/` bundle owns existing
behavior. `cargo xtask` owns validation order and writes run logs beneath ignored
`test-results/`. Do not create a Markdown evidence report for every small edit.

## The normal loop

```sh
# implement the next contract-backed task with its owning tests and fixtures
cargo xtask dev
cargo xtask ci pr
```

The developer chooses the next task from the active portable checklist and
marks it complete only after reviewing its implementation and evidence. No
command chooses tasks or changes checklist markers on the developer's behalf.

`dev` formats and runs the ordinary code, test, lint, smoke, fuzz-regression,
and documentation checks in maintained order. `ci pr` is the non-mutating
pre-push gate used by hosted CI; its generated summary and event log record the
exact commit, worktree cleanliness, package version, toolchain, steps, and
result. Run a focused `cargo xtask test <level>` while coding, then the
complete gate before marking
the checklist item `[x]`.

## What remains authored

| Authored source | When to change it |
| --- | --- |
| `portable/specs/` contracts and decisions | A normative language rule changes or a question is resolved. Reopen the freeze review first. |
| Fixtures, oracles, manifest, and traceability | Observable behavior or its executable evidence changes. Run `cargo xtask fixtures check` and `cargo xtask portable verify`. |
| Owning crate tests, code, and Rustdoc | The implementation changes. Every function, including private ones, needs a doc comment. |
| `portable/` checklist | A complete reviewed task passes its applicable gates; change only its checkbox. |
| `docs/` or crate READMEs | A user-facing command, public behavior, boundary, or development rule changes. |
| `quality/reviews/` | A human security, risk, dependency, or policy judgment changes. |

The checklist is the only routine hand-maintained completion tracker. Existing
historical stage notes may stay for audit, but new per-slice Markdown evidence
files are optional, not a gate. Prefer the test, frozen fixture/oracle, and
generated CI log. Write a short durable review only when a result needs human
interpretation, a limitation must be accepted, or policy requires it. The
validation ledger is historical context, not a second daily status source.

Do not copy raw generated output into tracked Markdown. Do not edit generated
`quality/STATUS.md`; `cargo xtask quality render` derives it from approval
records. `cargo xtask check` verifies the durable quality-document inventory.

## One feature, one boundary

1. Read the next unchecked task and its linked contract in the active portable
   plan.
2. Add positive, negative, boundary, and hostile fixtures where relevant; pin
   expected outcomes before accepting implementation behavior.
3. Add focused tests in the owning crate's `tests/` tree. Add public-boundary,
   deterministic, cancellation, and limit tests when the feature crosses those
   boundaries.
4. Implement without ambient I/O in compiler core, documenting every function.
5. Run focused tests, `cargo xtask dev`, then `cargo xtask ci pr`. Fix failures
   rather than lowering quality thresholds or rewriting frozen expectations.
6. Update the single checklist marker and only the authored sources in the
   table above that actually changed.

An unresolved normative question pauses implementation. Reopen the contract,
decision, fixture/oracle, and freeze review together; a test or generated
report cannot silently redefine the language.

## Stage and release boundaries

At a stage boundary, run the active conformance corpus and `cargo xtask ci pr`.
Review that the stage checklist, manifest, traceability, diagnostics, limits,
and public integration agree. The gate logs are generated, not copied into a
new note for every step. Only a reviewer can mark the stage complete.

For a release, follow [release and versioning](release-and-versioning.md):
prepare the workspace version, evaluate the clean `main` commit, record the
quality approval, assemble and inspect the package, then push the signed tag.
Those actions intentionally remain separate because they change the release
authority or publish an immutable ref.

Before replacing a completed active portable plan, promote the accepted
contracts, fixtures, oracles, and manifest into an immutable
`conformance/releases/<version>/` bundle. Run `cargo xtask portable verify` and
`cargo xtask portable snapshot`, review the generated snapshot, archive it in
the roadmap repository, and only then remove `portable/`. Normal compiler and
test behavior must continue to work without that temporary planning folder.

## Stable repository rules

- Derive package and tag identity from root `Cargo.toml`; keep language and
  artifact contract versions independent.
- Use stable Rust by default. Use nightly only for tools that require it.
- Keep host acquisition, filesystem, network, credentials, and execution out of
  compiler core.
- Use owning constants/configuration for shared values and `[category] message`
  for command output.
- Keep test bodies in owning `tests/` trees, and generated results under ignored
  `target/` or `test-results/`.
- Preserve the inherited released corpus. Never edit a released contract or
  oracle to make a new implementation pass.
