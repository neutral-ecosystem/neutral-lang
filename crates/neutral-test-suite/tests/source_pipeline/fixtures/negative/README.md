<!-- SPDX-License-Identifier: Apache-2.0 -->

# Rejected programs

Malformed syntax, forbidden imports, inaccessible names, private leaks, value
cycles, and incompatible contextual values must fail at an asserted phase and
code without publishing partial IR. Any reported original-byte location must be
nonempty, inside the exact captured bytes, and tied to a captured source digest.
The cross-module private-reference test additionally requires and pins its exact
consuming declaration span; not every failure contract promises a location.
