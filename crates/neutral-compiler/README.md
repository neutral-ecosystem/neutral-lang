<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-compiler

`neutral-compiler` owns the captured-input compilation pipeline: private
frontend processing, semantic analysis, and lowering to public logical IR.

It depends on core, IR, and vocabulary contracts only. The CLI supplies any
host-facing capture work before this crate runs; `neutral-compiler` must remain
deterministic and free of filesystem, environment, network, command, locale,
and clock access. Its parser and semantic internals are intentionally private.
The public boundary captures exact bytes immutably. The v1 project-capture
boundary accepts only a closed, versioned, data-only request with explicit
independent limits. It retains the complete supplied source set, validates
request/header agreement and exact vocabulary-lock coverage, and exposes no
resolver, callback, path, URL, root, or ambient acquisition hook. Its private
frontend recognizes the supported source, identifier, comment, exact-number,
bounded-string, Boolean, nullable-scalar, null, nominal-record, and
contextual-record behavior.
It collects the complete root scope before nominal type resolution and rejects
embedded record cycles. It accepts one exact host-captured vocabulary bundle and
lock, validates it before source payloads, and resolves optional `use` plus
qualified nominal types without performing acquisition. Exact trivia stays
private for the reference formatter and is never lowered into authoritative
logical IR.

The reference formatter accepts only captured source that completes the compiler
validation path. It emits canonical header order, LF newlines,
four-space record/list indentation, one field or item per line with a trailing
comma, normalized delimiter spacing, and no semicolons. Comments retain their
text and source order with LF-normalized line endings, and are deterministically
lifted to standalone top-level positions before the root construct that
contained or followed them; comments
after the final construct remain at end of file. Formatter output is ordinary
source bytes with a new source digest when recaptured—not logical IR, canonical
artifact encoding, or signing material.

## Command

This is a library crate. Verify its compilation pipeline with:

```sh
cargo test --package neutral-compiler
```
