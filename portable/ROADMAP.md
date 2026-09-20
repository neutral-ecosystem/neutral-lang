<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 portable roadmap

Status: accepted planning baseline

The roadmap is the nine-stage release train in [PLAN.md](PLAN.md). Its order is
intentional: capture and module semantics precede public APIs; project IR and
identity precede dynamic authoring; and conformance precedes the `v1.0.0`
release.

No Editor product work starts from a compiler-private AST. No Flow execution
work starts before the generic language, reader, vocabulary, and authoring
boundaries have passed their probes.
