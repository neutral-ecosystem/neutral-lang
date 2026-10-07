<!-- SPDX-License-Identifier: Apache-2.0 -->

# Explicit successor source integration

This directory contains request-local parsing, type resolution and lowering for
the composition capture profile. It reuses the private lexer and inherited type
grammar, then feeds source and vocabulary contracts through one shared semantic
validator. Frozen project compilation remains on its separate entry point.

Complete producer IR must pass the independent reader before encoding. Nothing
here acquires imports, opens inert locations, chooses public roots or executes
operations.
