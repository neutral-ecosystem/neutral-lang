<!-- SPDX-License-Identifier: Apache-2.0 -->

# Parser fault-injection tests

These crate-private tests truncate normalized token streams, including their
EOF sentinel, to verify bounded parser failures at every grammar boundary.
They complement source-level fixtures and protect the compiler's internal
lexer-to-parser contract without exposing syntax implementation types.
