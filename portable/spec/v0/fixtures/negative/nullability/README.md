<!-- SPDX-License-Identifier: Apache-2.0 -->

# Negative nullability fixtures

This directory contains source programs that the active scalar language must
reject. It fixes the boundaries for null in non-nullable contexts, repeated
postfix nullability, and generic or inner widening before generic types exist.

Within the ecosystem these immutable inputs protect the parser and semantic
checker from accepting later-stage grammar. Their exact failure class,
diagnostic code, and original-byte span are frozen in
`conformance/oracles/stage3`.
