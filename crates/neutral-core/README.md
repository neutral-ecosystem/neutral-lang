<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-core

`neutral-core` is the foundation shared by every production-facing Neutral
component. It owns source identity, spans, diagnostics, structural limits, and
cancellation contracts.

It sits at the bottom of the dependency graph. It must not depend on compiler,
reader, CLI, host, automation, or test packages, and it must not perform host
I/O. Higher layers use these stable value contracts to communicate without
coupling to a particular source parser or artifact encoding.
