<!-- SPDX-License-Identifier: Apache-2.0 -->

# Project IR oracles

[project.toml](project.toml) records the complete-project membership and
fail-closed public/private/view/malformed outcomes. Detailed mutation and
selection expectations live in the reviewed fixture descriptors. These are
normative expectations, not generated compiler output. The conformance manifest
pins both oracle and fixture bytes; the crate-owned suite independently checks
them through the public compiler and reader APIs.
