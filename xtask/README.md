<!-- SPDX-License-Identifier: Apache-2.0 -->

# xtask

`xtask` is Neutral's repository automation package. It provides the stable
`cargo xtask` commands for bootstrap, boundary checks, testing, CI, generated
evidence, and safe cleanup.

It is non-published and outside production dependency graphs. It enforces
repository policy around the compiler rather than implementing Neutral language
behavior, and writes generated evidence only beneath the configured
`test-results/` root.
