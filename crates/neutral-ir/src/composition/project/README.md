<!-- SPDX-License-Identifier: Apache-2.0 -->

# Shared composition policy and inspection

This module defines the common composition resource tuple and shared binding
signature, origin and attribution contracts used by current expression
projects. Its bounded inspectors preflight nominal bodies, vocabulary
catalogues and retained expression-project data before copying or recursive
consumption. They are structural accounting, not semantic validation or
reader authority.

`inspect_composition_bodies` and `inspect_composition_catalogue` support raw
vocabulary compilation. `inspect_expression_project_resources` is used by the
current expression producer and reader. Neither inspector constructs a
complete composition-project model.
