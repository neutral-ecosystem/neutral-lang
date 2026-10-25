<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-bench

`neutral-bench` owns benchmark harnesses and immutable current-language corpora.
It is non-published evaluation infrastructure and does not define language
semantics or alter compiler output. The harness measures expression-project
capture, graph construction, compilation, independent reader/wire work,
identities, cache reuse, growth and concurrency.

`cargo xtask test performance --profile pr` runs the informational PR profile.
Use `release` or `soak` only for the corresponding reviewed quality activity.
Source-bound measurements describe these exact current inputs and need fresh
qualification after the corpus or implementation changes.
