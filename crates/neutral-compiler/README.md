<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-compiler

`neutral-compiler` owns the captured-input compilation pipeline: private
frontend processing, semantic analysis, and lowering to public logical IR.

It depends on core, IR, and vocabulary contracts only. The CLI supplies any
host-facing capture work before this crate runs; `neutral-compiler` must remain
deterministic and free of filesystem, environment, network, command, locale,
and clock access. Its parser and semantic internals are intentionally private.
The public boundary captures exact bytes immutably. Its private Stage 2 frontend
and semantic layer recognize the frozen minimal document shape and lower it to
authoritative public IR without exposing parser or semantic-model types.
