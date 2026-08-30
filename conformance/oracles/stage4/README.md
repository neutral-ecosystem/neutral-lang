<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 4 conformance oracles

This directory freezes expected results for record, default, and list vertical
slices. Slice 4.1 entries cover nominal schemas, contextual values, collection
order, field validation, recursion, and bounded record structure. Slice 4.2
entries cover closed scalar/null/record defaults, omission materialization,
field provenance, and rejection of dependency-bearing defaults.
Slice 4.3 entries cover ordered, empty, nested, nullable-element, and defaulted
lists plus bounded item, nesting, and traversal failures.

Within the ecosystem these files are test authority rather than compiler input.
They bind immutable fixture bytes to public IR observations or exact rejection
class, diagnostic code, and original-byte span.
