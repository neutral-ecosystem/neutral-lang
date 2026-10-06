<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition profile selectors

This module owns exact composition contract selectors shared across producers
and independent consumers. Package versions never select language behavior.
Raw type/value contracts remain in the parent `composition` module. Selectors
do not imply that source compilation, binary encoding or identity APIs implement
the successor profile.
