<!-- SPDX-License-Identifier: Apache-2.0 -->

# Successor project structure

This module owns raw complete composition-project data and bounded retained-data
inspection. Logical declarations and vocabulary contracts stay separate from
source maps, provenance, origins, limits and accounting. Inspection bounds raw
structures before recursive consumers or clones; it is not semantic validation.
The compiler produces these contracts, the independent reader validates them,
and the codec transports only independently validated complete artifacts.
