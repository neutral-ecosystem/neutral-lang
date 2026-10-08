<!-- SPDX-License-Identifier: Apache-2.0 -->

# Allocation test support

`testing.rs` provides thread-local, unwind-safe reservation failure scopes behind
the non-default `allocation-testing` feature. Cross-package tests enable it to
exercise the same fallible ownership and capacity operations used in production.
It does not replace the global allocator or simulate operating-system exhaustion.
Production builds without the feature carry no test policy or mutable failpoints.
