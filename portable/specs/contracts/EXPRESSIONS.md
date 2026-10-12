<!-- SPDX-License-Identifier: Apache-2.0 -->

# Pure expression contract — review candidate

Profile: `neutral.expression/1`. Covers V1-CONS-011..014. Status: concrete review
candidate, not frozen or activated. Review this with [EVALUATOR](EVALUATOR.md) and
[EXPANSION](EXPANSION.md) before checking the expression contract-freeze gate.

## Selection and scope

Source header `neu "1.0"` alone does not activate expressions. An explicitly
selected successor capture/profile must require `pure-expressions-v1`,
`decimal-division-v1`, and `pure-evaluator-v1`. Historical source/capture/IR/wire/
identity profiles, values and vectors are immutable. The successor transport,
identity transcript and authoring extension need their own registered vectors
and compatibility freeze; this document does not extend project /2 in place.

Expressions produce immutable typed values. No assignments after declaration,
imperative blocks, general functions, callbacks, dynamic dispatch, effects,
ambient lookup or unbounded iteration exist. Vocabularies describe data, not
executable validators. Semantic restrictions apply to computed/bound values too.

## Source forms and precedence

Existing literals, records, variants, lists, binding reuse and `ref(symbol)` remain
contextually typed. Parentheses group expressions. The expansion contract adds
`instantiate` as an expression primary. New forms are:

```neu
num third = 1 / 3
bool selected = replicas >= 2 && !maintenance
num count = if selected { replicas * 2 } else { 1 }
string summary = $"count=${text(count)}"
string state = match outcome {
    ready(message) => message,
    failed(code) => text(code),
}
```

`if` requires both branches and produces their common exact resolved type.
Braces contain one expression, not statements. A match arm is `tag(binder) =>
expression`; every tag of the exact closed variant occurs exactly once, with no
wildcard/default arm. The binder has that tag's payload type and is arm-local.
No structural conversion between different nominal types is allowed.

From strongest to weakest: primary/grouping/intrinsics; prefix `!` and `-`;
`* /`; `+ -`; `++`; `< <= > >=`; `== !=`; `&&`; `||`. Binary operators at one
level associate left-to-right; chained comparison/equality without parentheses
is rejected. `if`/`match` are expression primaries whose bodies parse complete
expressions. A leading minus in an expression is unary negation of its operand.
There are no compound assignments, increment/decrement, remainder or power.

`+ - * /` require two `num` operands; unary `-` requires `num`.
`&& || !` require `bool`. Ordered comparisons accept two `num` or two `string`
operands; string ordering is lexicographic UTF-8 byte ordering, without Unicode
normalization or locale. Equality requires identical resolved types and compares
normalized values structurally; records compare fields by canonical name, lists
by position, variants by nominal type/tag/payload, and refs by stable target
identity without dereferencing. Optional absence, null and a present value differ.
Nullable equality needs the same nullable context for both operands. No coercion
from number to Boolean, string to number, or nominal type to a lookalike exists.

`++` concatenates two strings or two lists with exactly the same element type.
List elements and record/variant payloads may be expressions. Empty lists require
an expected list type; they do not infer an arbitrary element type. `text(expr)`
is a reserved conversion intrinsic, not a callable user function: it accepts
`num`, `bool`, or `string` only. Booleans become `true`/`false`; numbers use their
canonical finite decimal spelling (zero `0`, no negative zero, no redundant
fractional zeroes or exponent). Check output-byte budgets before expanding a
large scale into text. URLs, paths, refs, null and nominal values need explicit
consumer formatting outside this intrinsic.

Interpolation uses `$"...${string_expression}..."`; ordinary quoted strings
remain literal. `\${` escapes an interpolation opener in this form only; other
escapes retain ordinary string rules. Nested strings inside `${...}` are parsed
as expressions, not by textual replacement. Each inserted expression must be a
string; use `text` for explicit numeric/Boolean conversion. No escaping for shell,
SQL or a backend is implied. Scalar-valid UTF-8 is retained without normalization.

## One `num` and approximate division

Literal and stored decimals remain exact normalized sign/coefficient/scale values.
Addition, subtraction, multiplication and negation are exact within independent
digit, scale, work and storage limits. Decimal division is not integer division.
Reduce the exact quotient first: a denominator containing only factors 2 and 5
has a terminating decimal, retained exactly if it fits the limits. Otherwise
round to 34 significant decimal digits, nearest with ties to even. This arithmetic
context is profile semantics, not caller-configurable precision.

For example, `1 / 8` is `0.125`; `1 / 3` is
`0.3333333333333333333333333333333333`. Determine the exact decimal exponent and
round the magnitude, then apply the sign and normalize. Compare twice the
remainder with the denominator to choose the nearest retained coefficient; on
equality increment only an odd last retained digit. Handle carry into the exponent.
Zero is canonical unsigned zero. Do not approximate operands using host floats.
Exact terminating results need not be reduced to 34 digits. Numeric limits may
reject them, but cannot silently change precision or select a different rounding
mode. No infinities, NaNs, saturation or silent scale overflow exist.

Division by zero is a distinct error; nontermination is not an error. Intermediate
digit/scale/work overflow or allocation failure is a resource error, even if a
later operation would cancel it. Preserve evaluation order: `(1 / 3) * 3` need
not equal `1`; do not reassociate or apply algebraic rewrites across rounding.

## Names, phases and guards

Resolve symbols and exact types in every branch, including unreachable branches.
Unknown/private names, bad operators and incomplete matches always fail typing.
Evaluate ordinary operands left-to-right. `&&` evaluates its right operand only
when its left is true; `||` only when false. `if` evaluates its guard and selected
branch; `match` evaluates its scrutinee and selected payload arm. Unselected
branches neither evaluate arithmetic nor demand external/output bindings.

Declare an externally supplied immutable value as `input Type name` or
`public input Type name`. Inputs have no initializer or implicit default. Their
stable symbol, type and restrictions are project meaning; supplied values are
consumer context. The compiler does not read an environment variable with this
name. Ordinary binding dependencies may not form value cycles.

`output<Type>(ref(public_target), ResultType, path_literal)` is a reserved
deferred-output intrinsic. `ResultType` is an exact public nominal type and the
path is a compile-time list of Consumer/1 field/index/alternative segment records.
Resolve it in type mode, preserving future index/tag/presence obligations; its
resolved type must exactly equal `Type`. The target must be an existing public
nominal binding, not a host locator. Core does not assert that an arbitrary target
produces `ResultType`: the consumer must verify that association before binding.
No selector implicitly dereferences a ref or creates a scheduling prerequisite.

Fully static selected expressions normalize to ordinary typed values. Deferred
guards keep their guarded bodies; do not fold potentially failing paths early.
Classify dependencies as structural references, guarded potential value bindings,
and explicit consumer prerequisites, never one undifferentiated edge set.
Binding-demand traversal follows evaluation order and discovers only selected
paths; unavailable guards do not demand both branches.

## Canonical residual model and identity

The semantic model has typed nodes `value`, `input`, `output`, `unary`, `binary`,
`if`, `match`, `text`, `interpolate`, `list`, `record`, and `variant`. Each node
has its exact resolved result type; operator and profile IDs are closed enums.
Input/output leaves use stable symbols and canonical typed selector tuples.
Match arms sort by tag for encoding, while evaluation chooses the exact tag.
Operands retain source evaluation order. Limits/attribution are separate facts,
not executable nodes. A decoder must independently retype all retained nodes,
check visibility/closure and reject forged result types or missing obligations.

Static normalized values and expanded descriptions determine logical meaning;
residual expressions retain typed ordered operator/operand/guard structure.
No commutative reordering or general algebraic equivalence is claimed. Arithmetic
and evaluator profile IDs are semantic inputs. External binding values, selected
entries and adapter/target policy belong to consumer plan keys, not immutable
project identity. Captured/derivation/artifact layers retain relevant source,
context and encoding facts under separately versioned transcripts. Do not modify
historical domains or vectors. Wire/transcript byte grammar remains the next
explicit compatibility/vector gate, not an implied extension to an old format.

## Diagnostics and limits

| Code | Meaning |
| --- | --- |
| NEU-EXP-001 | Unsupported profile/feature |
| NEU-EXP-002 | Malformed expression or unknown intrinsic/operator |
| NEU-EXP-003 | Unknown/private symbol or type |
| NEU-EXP-004 | Operand/result/branch type mismatch |
| NEU-EXP-005 | Missing/duplicate/unknown match alternative |
| NEU-EXP-006 | Division by zero on an evaluated path |
| NEU-EXP-007 | Value cycle |
| NEU-EXP-008 | Restriction or selector-contract violation |
| NEU-EXP-009 | Resource/checked-arithmetic/allocation failure |
| NEU-EXP-010 | Cancellation |

Positive independent hard ceilings: 1,000,000 expression nodes, 64 syntactic and
evaluation depth, 1,000,000 bindings and dependency edges each, 100,000,000 work
visits, 67,108,864 retained/output bytes each, 4,096 coefficient digits and
absolute scale, 1,000,000 diagnostics. Intersect with existing lower capture,
reader, composition and caller limits. Count children, digits, output and visits
before allocation/expansion; cancellation/fallible retention applies throughout.
Typing errors precede value evaluation; unsupported envelopes precede typing.
At checkpoints resource/cancellation failures terminate atomically. Sort ordinary
diagnostics by phase, stable symbol, original byte range, code and typed path.
No failure publishes partial accepted IR or replaces a valid cache generation.
