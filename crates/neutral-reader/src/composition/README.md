<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition catalogue inspection

This module exposes independently validated vocabulary and resolved-scope
catalogues. It supports exact public contract lookup, safe closed-value
materialization, declared reference-type inspection, and public binding/value
reference inspection. These APIs do not accept complete captured projects,
derive project views, or confer execution authority.

The current complete-project reader is `ValidatedExpressionProject` in the
parent crate API. It retains expression and composition meaning together and
supports public projection through the current expression APIs.
