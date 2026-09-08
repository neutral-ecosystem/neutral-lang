<!-- SPDX-License-Identifier: Apache-2.0 -->

# Static work-product review

Review date: 2026-09-08. Owner/reviewer: sole maintainer. Independence: not
claimed. Result: pass with the sole-maintainer review limitation recorded.

Reviewed work products include accepted requirements and syntax inventories,
contract decisions, fixture/oracle manifest, grouped fixtures, public APIs,
dependency/effect boundaries, source and vocabulary identity transcripts,
closed vocabulary and external IR schemas, hostile decoder validation order,
CLI publication policy, tests, documentation, and traceability.

Findings resolved during Stage 9:

- Inline test bodies were removed from production source and retained as
  path-based modules below each crate’s `tests/` directory.
- CI now rejects reintroduction of inline or non-path test modules.
- Compiler cancellation was checked only before compilation; explicit checks
  now guard capture, post-vocabulary, post-frontend, and pre-publication semantic
  handoffs.
- The first mutation run found 38 surviving name/version/feature predicate
  mutants. Focused exhaustive boundary tests were added; the repeat run caught
  all 38.
- The broader review initially retained viable decoder and logical-equality
  mutants. Added hostile recursive decoder, error-offset, parser fault,
  diagnostic-ordering, artifact-equivalence, CLI fault, probe-rendering, and
  automation-failure tests. The final 271-mutant review caught 244 and marked
  27 unviable, with no missed viable mutant.
- The configured all-target coverage gate now passes at 90.57% lines, 90.71%
  functions, and 81.72% regions without a reduced threshold or scope.
- Valgrind Massif and Memcheck now cover the direct benchmark executable.
  Release and 50,000-iteration extended-soak profiles have the same 524,640 B
  Massif total peak; Memcheck reports no memory errors or leak classes.

No unresolved normative contradiction or unsafe/native build surface was found.
