<!-- SPDX-License-Identifier: Apache-2.0 -->

# GitHub automation adapters

This directory owns thin GitHub Actions entry points. Pushes to `main` run the
PR-quality command and documentation; pushed tags and manual dispatch run
release preparation. Workflows select stable `xtask` commands and permissions
but do not duplicate test, quality, path-safety, or release policy.
