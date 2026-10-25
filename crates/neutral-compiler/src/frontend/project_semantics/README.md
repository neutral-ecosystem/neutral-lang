<!-- SPDX-License-Identifier: Apache-2.0 -->

# Current captured source compilation

This directory contains the nominal parsing, type resolution and source
companion helpers used by current expression compilation. The shared bounded
type grammar lives in `type_syntax`; exact syntax can be retained by the
caller-owned `/3` cache.

All graph, catalogue/default, semantic and companion work stays fresh on each
compile. Failed runs never publish a pending cache generation. Iterative graph
passes enforce component-size and traversal-work bounds before expression
expansion. Complete producer IR still requires independent reader validation.

Nothing here acquires imports, opens inert locations, chooses public roots or
executes operations.
