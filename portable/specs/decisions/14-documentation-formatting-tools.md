# Section 11: documentation and formatting

## SYN-TOL-001 — Comments and stable formatting

`//` and `/* ... */` are non-semantic trivia. v0 has no documentation comment,
annotation, attribute, or attachment model. Documentation tools may display
source comments but cannot treat them as logical IR.

### Formatter

One reference formatter defines stable output for v0 syntax:

- canonical header order;
- four-space indentation inside record declarations and values;
- one declaration field per line with trailing comma;
- normalized spaces around `=`, after `:`, and after commas;
- no semicolons; and
- deterministic preservation/placement of comments.

Formatting must preserve logical IR. Formatted bytes are not canonical IR bytes,
source identity, or signing material.

Idempotence and parse/format/parse logical equality are conformance properties.
