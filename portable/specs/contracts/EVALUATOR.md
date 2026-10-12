<!-- SPDX-License-Identifier: Apache-2.0 -->

# Shared pure evaluator

Profile: `neutral.evaluator/1`, requiring [expression/1](EXPRESSIONS.md).
Status: approved and frozen on 12-10-2026; reviewed bytes are pinned in
[freeze.toml](freeze.toml). Not implemented or activated. This is a reusable value
evaluator, not a scheduler, executor or interpreter of arbitrary consumer data.

## Boundary

The compiler and independent readers use the same production semantic evaluator.
Its inputs are an independently validated expression closure, exact profile
tuple, explicit bindings, positive controls and cancellation. It cannot import
compiler-private ASTs or take resolver, I/O, clock, randomness, environment,
credential or executable callbacks. It never waits for or acquires a binding.

A binding has a stable declared input symbol or `(public target symbol, canonical
result selector)` key and a state: `value`, `unavailable`, or `failed`. A missing
key is not a state supplied as a value. Unknown/duplicate keys and malformed
binding envelopes fail before evaluation. Validate supplied types, target/result
association, visibility and retained restrictions; no external value becomes
accepted merely because the caller labels it typed. Null is an explicit typed
value and never a substitute for absence, unavailable output or failure.

Results are a closed tagged sum:

- `value`: a complete validated typed value;
- `required-bindings`: sorted unique canonical keys demanded on selected paths;
- `unavailable`: a demanded unavailable binding and safe attribution;
- `failure`: a stable error code and bounded safe diagnostics.

For independent ordinary operands, traverse left-to-right and collect missing
keys; stop atomically on an actual failure/unavailable operand. When a guard is
missing, request that guard's bindings only, not either branch. Do not evaluate
an operator before all its required operands exist. No result exposes a partial
value; `required-bindings` is demand information, not partial success. Retrying
with a new complete binding generation starts a new request; evaluation never
mutates the previous successful project or context.

## Validation, compatibility and evidence

Validate independently bounded envelopes/profile/controls before evaluation.
Require `neutral.expression/1`, `neutral.evaluator/1` and its fixed decimal context;
unsupported semantics fail closed before any operation dispatch. A backend's
coercion, floating-point, rounding, null or lazy-evaluation rules cannot replace
these rules. Consumer execution policy cannot modify evaluator semantics.

Use NEU-EXP codes for expression failures and these evaluator boundary codes:
NEU-EVAL-001 unsupported; 002 invalid/duplicate/unknown binding envelope;
003 binding type/visibility/result-contract mismatch; 004 retained restriction
failure; 005 supplied binding failure; 006 resource/allocation failure;
007 cancellation. `required-bindings` and `unavailable` are explicit non-value
results, not compiler type failures. Only demanded bindings affect those results.
Already supplied malformed values are never ignored as trusted data.

Controls independently bound bindings, binding bytes, expression nodes/depth,
numeric growth, work, dependency edges, diagnostics and output/retention bytes,
intersecting the expression and existing reader hard ceilings. Validate before
retention, check cancellation before expansion, and use fallible allocation.
Order diagnostics by stable public subject, typed path, code and safe byte range.
Reports expose checked public occurrence/contract locations or explicit unavailable
attribution; private reuse origins and host paths are never returned.

Consumer keys bind project logical identity, exact evaluator/arithmetic profiles,
selected entry, adapter/target revisions and relevant binding generation. External
bindings do not rewrite project meaning. Evaluation caches cannot mix projects,
binding generations or incompatible profiles; failed evaluations cannot replace
successful generations. No persistent cache is required by this contract.

Require literal vectors comparing compiler folding, compiler-independent shared
evaluation and a separately implemented interpreter. The independent interpreter
must not call the production evaluator. Cover rounding, guarded missing/skipped
outputs, null/unavailable/failure distinctions, restrictions, cancellation,
allocation faults, bounds, redaction and compatibility rejection. No test executes
backend commands, acquires credentials or changes a real consumer system.
