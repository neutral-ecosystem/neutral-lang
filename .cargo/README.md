<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cargo integration

This directory owns repository-local Cargo aliases and nonsemantic build
defaults. It exposes `cargo xtask` and `cargo docs`; it must not define language,
test, quality, packaging, or release policy. Those responsibilities belong to
`xtask` and tracked configuration.
