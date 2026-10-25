<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-compiler

This crate owns effect-free capture, source parsing, semantic resolution, finite
expansion and lowering to complete project IR. It uses core, IR, shared pure
semantics and vocabulary contracts. Hosts acquire bytes and exact locks explicitly;
the compiler cannot consult filesystems, networks, environment, processes or clocks.

The supported library path is `capture_expression_project` →
`compile_expression_project`. A host constructs a closed
`CapturedExpressionProjectRequest` with explicit capture, vocabulary, expression
and expansion policies. `CapturedProjectRequestBuilder` helps assemble its byte
envelope without retaining editor locations or host paths.

Capture retains the complete supplied closure, checks source identities and
headers, exact transitive locks, public contracts, defaults and restrictions.
`CapturedExpressionProject::replay_request` copies those inputs fallibly with
fresh caller cancellation; it does not acquire missing inputs or infer a profile.
Captured identity and acceptance controls are distinct from logical project meaning.

Compilation checks every branch and resolves names/types before folding selected
static paths with the shared pure evaluator. Deferred bodies retain their typed
inputs, guarded dependencies and obligations. Templates and finite matrices expand
only at compile time; no scheduler or executor is provided. Successful raw IR must
still pass `neutral_reader::ValidatedExpressionProject::from_ir` before encoding.
That independent reader checks semantics, companions, resource facts and identities.

For code orientation, follow [capture](src/project_capture/expression.rs),
[bounded graph construction](src/frontend/project_semantics/nominal/graph.rs),
[source expression semantics](src/frontend/project_semantics/expressions/mod.rs),
[lowering](src/frontend/project_semantics/expressions/lowering.rs), and
[finite expansion](src/frontend/project_semantics/expressions/expansion.rs).
[Nominal resolution](src/frontend/project_semantics/nominal/README.md) is a sibling
of expression compilation; neither module owns the other. Cache, formatter and
expansion live with their expression owner, without redirected production modules.

`ExpressionCompilationCache` is the only caller-owned syntax cache.
`compile_expression` checks exact bytes, module, semantic tuple and parser policy;
current vocabularies, defaults, restrictions, graph facts, source evidence and
identities are rebuilt on every call. Independent `ProjectCacheLimits` bound retained
units/bytes and `ProjectCacheStats` record actual reuse. Failed or cancelled requests
preserve the prior successful generation. There is no persistent or externally
supplied cache and no memoized semantic authority.

`format_expression_source` validates the entire captured project, formats one
explicit module and recompiles the resulting source before returning it. It keeps
comments and string/interpolation spellings, uses LF and four-space indentation,
and enforces bounded fallible output with cancellation. Formatting changes source
bytes and source evidence; it does not produce canonical artifact bytes.

Development accepts only the current complete source/artifact pipeline. Historical
release fixtures remain records; see the [compatibility policy](../../docs/compatibility-policy.md).

## Commands

Run `cargo xtask test all` for the repository suite, including doctests. To check
this library directly:

```sh
cargo test --package neutral-compiler
```
