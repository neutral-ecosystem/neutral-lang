<!-- SPDX-License-Identifier: Apache-2.0 -->

# Development container

This directory owns the reproducible supported container definition. It pins
the base image and stable toolchain, mounts generated Cargo/results storage,
and delegates environment verification to `cargo xtask environment verify`.
It is a developer adapter, not a release artifact or language input.
