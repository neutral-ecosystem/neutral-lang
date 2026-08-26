<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-ir

`neutral-ir` owns Neutral's public logical intermediate-representation and
source-accounting contracts. It is where accepted program meaning is described
independently of source spelling, storage, or external encoding.

It depends only on `neutral-core`. The compiler produces this logical model;
the reader validates and exposes it; the vocabulary layer refers to it; and the
probe observes it through public reader contracts. It must not acquire input or
perform host effects.
