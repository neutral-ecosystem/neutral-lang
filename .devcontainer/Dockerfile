# SPDX-License-Identifier: Apache-2.0

FROM rust:bookworm

RUN rustup toolchain install stable --profile minimal --component rustfmt --component clippy \
    && rustup default stable \
    && cargo install cargo-nextest --locked \
    && useradd --create-home --shell /bin/bash neutral \
    && mkdir -p /usr/local/cargo/registry /workspaces/neutral-lang/target /workspaces/neutral-lang/test-results \
    && chown -R neutral:neutral /usr/local/cargo /workspaces/neutral-lang

USER neutral
WORKDIR /workspaces/neutral-lang

# Rebuilding the container follows the repository's rolling stable policy.
ENV RUSTUP_TOOLCHAIN=stable
