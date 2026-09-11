<!-- SPDX-License-Identifier: Apache-2.0 -->

# GitHub automation adapters

This directory owns thin GitHub Actions entry points. Pushes to `main` run the
the logged `cargo xtask ci pr` composition; pushed tags and manual dispatch run
release preparation. Workflows select stable `xtask` commands and permissions
but do not duplicate test, quality, path-safety, or release policy. CI uploads
the generated workflow event log even when a gate fails.
