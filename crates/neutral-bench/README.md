<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-bench

`neutral-bench` owns benchmark harnesses and immutable benchmark-corpus
identities for Neutral.

It is non-published evaluation infrastructure, separate from production and
conformance dependencies. It measures approved behavior and resource limits;
it does not define language semantics, make acceptance decisions, or alter
compiler output.

`cargo bench --package neutral-bench --bench stage9 -- pr|release|soak` runs
the controlled Stage 9 profile. PR measurements are informational; release
thresholds require a recorded dedicated runner and reviewed baseline.
