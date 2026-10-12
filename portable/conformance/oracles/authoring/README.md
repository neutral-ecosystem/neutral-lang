<!-- SPDX-License-Identifier: Apache-2.0 -->

# Authoring foundation oracles

This directory owns reviewed literal results for the
[foundation requests](../../../specs/fixtures/authoring/foundation.json).
`foundation.json` joins requests by unique case ID; accepted cases declare
specific facts, source bytes, or checked mapping spans. Rejected cases declare
the stable `NEU-AUTH` code and always require no partial output or replacement
of the prior accepted generation.

These are contract expectations, not generated production evidence. The frozen
suite becomes required-active only after the authoring implementation and public
integration compare real results against these unchanged values. Future
expression/template authoring profiles require separate fixture/oracle review.
