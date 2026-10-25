<!-- SPDX-License-Identifier: Apache-2.0 -->

# Current captured source compilation

`expressions/` owns parsing, resolution, lowering, finite expansion, formatting
and the caller-owned syntax cache. `nominal/` owns shared nominal declarations,
defaults, import graphs and captured companion facts. They are sibling modules;
their physical locations follow their Rust ownership. The shared bounded type
grammar lives in `type_syntax.rs`.

All graph, catalogue/default, semantic and companion work stays fresh on each
compile. Failed runs never publish a pending cache generation. Iterative graph
passes enforce component-size and traversal-work bounds before expression
expansion. Complete producer IR still requires independent reader validation.

Nothing here acquires imports, opens inert locations, chooses public roots or
executes operations.
