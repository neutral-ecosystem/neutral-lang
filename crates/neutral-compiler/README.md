<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-compiler

`neutral-compiler` owns the captured-input compilation pipeline: private
frontend processing, semantic analysis, and lowering to public logical IR.

It depends on core, IR, and vocabulary contracts only. The CLI supplies any
host-facing capture work before this crate runs; `neutral-compiler` must remain
deterministic and free of filesystem, environment, network, command, locale,
and clock access. Its parser and semantic internals are intentionally private.
The public boundary captures exact bytes immutably. Its private frontend now
recognizes the frozen minimal document plus all Stage 3 source-text, identifier,
comment, exact-number, bounded-string, Boolean, nullable-scalar, null, nominal
record, and contextual-record behavior. It collects the complete root scope
before nominal type resolution and rejects embedded record cycles. Exact
trivia stays private for a later formatter and is never lowered into
authoritative logical IR.
