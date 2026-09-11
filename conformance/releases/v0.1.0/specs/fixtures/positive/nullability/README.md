<!-- SPDX-License-Identifier: Apache-2.0 -->

# Positive nullability fixtures

This directory contains source programs that the active scalar language must
accept. It covers explicit typed null for `string?`, `num?`, and `bool?`, plus
the permitted outer widening from a non-null scalar value to its nullable type.

Within the ecosystem these immutable inputs connect the portable v0 contracts
to compiler, logical-IR, reader, probe, and conformance evidence. Expected
results are frozen in `conformance/oracles/stage3`.
