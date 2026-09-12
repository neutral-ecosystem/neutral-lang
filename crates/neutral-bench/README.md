<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-bench

`neutral-bench` owns benchmark harnesses and immutable benchmark-corpus
identities for Neutral.

It is non-published evaluation infrastructure, separate from production and
conformance dependencies. It measures approved behavior and resource limits;
it does not define language semantics, make acceptance decisions, or alter
compiler output.

`cargo xtask test performance --profile pr|release|soak` runs the controlled
performance profiles. PR measurements are informational; release thresholds
require a recorded dedicated runner and reviewed baseline.

## Command

Run the controlled local performance profile with:

```sh
cargo xtask test performance --profile pr
```

Use the `release` or `soak` profile only when performing the corresponding
reviewed quality activity.
