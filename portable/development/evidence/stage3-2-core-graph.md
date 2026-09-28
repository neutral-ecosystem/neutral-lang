<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 3.2 core module graph

Status: core implementation complete; public captured-project integration remains Stage 3.3

Date: 2026-09-28  
Target gate: `v0.3.2`

The compiler now exposes a pure `build_module_graph(&CapturedProject,
&CancellationToken)` operation. It scans captured source bytes with the
existing lexer, validates the v1 header and exact logical import syntax, and
resolves targets only against the complete captured source set. There is no
resolver, filesystem, network, URL, or path input in this API. No graph is
returned after a syntax, closure, bound, or cancellation failure.

Modules and edges have stable order and retain original-byte spans. Iterative
Kosaraju traversal finds SCCs without recursion; dependency-first condensation
uses lexical tie-breaking. Per-module imports, total edges, and SCC size use
their separate captured-request limits. The number of condensation components
cannot exceed the already bounded source-unit count.

Compiler-owned tests cover the frozen positive cycle, semantic-cycle deferral,
missing/self/duplicate/alias and forbidden-import cases, the forbidden-form
diagnostic-order oracle, each exact/one-over graph limit, input shuffling,
dependency tie-breaking, cancellation, and a 512-unit chain. The tests construct
captured requests in memory and do not depend on the transient `portable/`
directory. Stage 3.3 still needs to attach the graph and diagnostics to the
public captured-project compilation result; Stage 3.4 still needs the full
executable fixture corpus and schedule/ambiguity validation.

## Verification

```text
cargo test -p neutral-compiler
cargo clippy -p neutral-compiler --all-targets -- -D warnings
cargo xtask check
cargo xtask ci pr
```

The composed PR workflow passed at `test-results/workflows/ci/pr/run-2-5`.
