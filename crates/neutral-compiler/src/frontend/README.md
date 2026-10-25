<!-- SPDX-License-Identifier: Apache-2.0 -->

# Private source frontend

`lexer.rs` owns bounded shared nominal/header tokens, strings, trivia and exact
original-byte spans. `graph_syntax.rs` scans explicit module/import facts.
`project_semantics/expressions/` owns current expression parsing, all-branch
resolution, static normalization, finite expansion, formatting and syntax caching.
`project_semantics/nominal/` owns shared record/variant contracts, defaults,
import/SCC checking and captured companion facts. The physical tree follows these
private Rust module boundaries; production modules do not redirect sibling files.

Pure expressions are current language data; effects, acquisition and scheduling
remain outside the compiler. Public consumers receive checked diagnostics and
complete IR that still requires independent reader validation. No partial syntax
or recovered declaration becomes authoritative output.
