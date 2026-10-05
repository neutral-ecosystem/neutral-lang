<!-- SPDX-License-Identifier: Apache-2.0 -->

# Pipeline source fixtures

Positive and negative source programs cover the supported project language.
Every file is explicitly registered in the runner; module IDs and expected values
are test inputs, never inferred by splitting or rewriting the source headers.
Vocabulary bundles are supplied as exact data-only captured inputs.

The current project-capture contract requires the profile and module on the
first two physical lines. `.neu` fixtures need no SPDX comment; the runner passes
complete original bytes without stripping or rewriting them.
