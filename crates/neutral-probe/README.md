<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-probe

`neutral-probe` is the reader-only inspection library and standalone probe
binary for Neutral artifacts.

It depends only on `neutral-core` and `neutral-reader`. Its purpose is to prove
that artifacts can be inspected through public reader contracts alone; it must
not import the compiler, private parser/semantic models, capture logic, or a
filesystem resolver. It reports observations, not application-specific meaning.

The active probe implements deterministic binding and nominal-record schema
summaries plus one consumer-owned diagnostic mapped through the public reader
source map. The standalone binary remains an encoded-input shell and does not
link the compiler.
