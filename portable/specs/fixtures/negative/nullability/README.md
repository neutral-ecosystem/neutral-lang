<!-- SPDX-License-Identifier: Apache-2.0 -->

# Negative nullability fixtures

This directory contains source programs that the active scalar language must
reject. It fixes the boundaries for null in non-nullable contexts and repeated
postfix nullability.

Within the ecosystem these immutable inputs protect the parser and semantic
checker from accepting invalid nullable shapes. Their exact failure class,
diagnostic code, and original-byte span are frozen in
`conformance/oracles/stage3`.
