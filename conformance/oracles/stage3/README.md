<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 3 conformance oracles

This directory freezes expected results for Stage 3 source-text and scalar
vertical slices, including bounded strings, Booleans, exact numbers, nullable
scalar types, and explicit null. Each oracle binds one immutable fixture digest
to either the complete observable compilation result or an exact rejection
class, diagnostic code, and original-byte span.

Within the ecosystem these files are test authority, not compiler input. The
compiler produces artifacts or diagnostics, the test suite compares them with
these expectations, and release automation records the resulting evidence.
