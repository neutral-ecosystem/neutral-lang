# Section 2: lexical source text

## SYN-LEX-001 — Encoding and spans

Source is UTF-8. One UTF-8 BOM is allowed only at byte offset zero and is not a
token. Malformed UTF-8, another BOM, and unescaped NUL are fatal errors.

Captured source retains original bytes and content digest. `CRLF` and lone `CR`
normalize to logical `LF`, while diagnostic spans remain half-open offsets into
the original bytes.

## SYN-LEX-002 — Whitespace and termination

Space and horizontal tab separate tokens and have no indentation meaning.
Declarations end with semantic `LINE_END`; v0 has no semicolon or backslash
continuation.

```text
raw lexer -> newline/layout normalizer -> parser
```

The raw lexer emits physical newlines. The layout stage emits `LINE_END` after a
complete header, `use`, or declaration. It suppresses line ends inside `()`,
`[]`, and value/record `{}` delimiters and after syntactically incomplete tokens.
It inserts a final `LINE_END` at EOF when needed.

## SYN-LEX-003 — Comments

`//` continues through the next physical newline. `/* ... */` is a non-nesting
block comment. Comments are whitespace/trivia and never attach documentation.
Unterminated block comments are errors.

## SYN-LEX-004 — Identifiers

Identifiers are ASCII only.

```text
snake_name       = [a-z][a-z0-9]*("_"[a-z][a-z0-9]*)*
upper_name       = [A-Z][A-Za-z0-9]*
```

Bindings, fields, and modules require `snake_name`. Records and vocabulary
names require `upper_name`; `UpperCamelCase` is the style recommendation, while
the compiler enforces only the regex. Leading/trailing/repeated underscores and
wrong-category casing are invalid.

Core names `num`, `string`, `bool`, `List`, `Ref`, `neu`, `module`, `use`,
`record`, `true`, `false`, `null`, and `ref` cannot be redeclared.

## SYN-LEX-005 — Qualification

`::` qualifies a type through the single imported vocabulary:

```text
Fixture::Metadata
```

v0 has no local namespace, module path, `.` member/static selection, or chained
qualification beyond `Vocabulary::Type`.

## SYN-LEX-006 — Delimiters and literals

- `{}` enclose record declarations and contextual record values;
- `[]` enclose list values;
- `()` enclose `ref(name)`;
- `<>` enclose `List<T>` and `Ref<T>` arguments;
- `:` separates a record field name from its value;
- `=` introduces a binding value or field default;
- `?` after a type makes it nullable; and
- `,` is required after record declaration/value fields and optional after the
  final list element.

Strings use double quotes and support `\"`, `\\`, `\n`, `\r`, `\t`, `\0`, and
`\u{HEX}` for one to six hexadecimal digits denoting a Unicode scalar. Unknown
escapes, isolated surrogates, raw control characters, and unterminated strings
are errors.

`true` and `false` are `bool`. `null` is the only explicit null value and is
valid only with a nullable expected type.

## Numeric literals

A source `num` is a signed exact base-10 rational, not a host integer or float.
Accepted spelling is digits with optional `_` separators, optional decimal
fraction, and optional base-10 exponent. Separators must occur between digits.
Non-finite values and base-prefixed numbers are invalid.

IR retains the normalized exact rational. v0 performs no signed/unsigned integer,
decimal-width, or IEEE binary target conversion. Values such as `0.1`, `0.5`,
and `16_777_217` remain exact Neutral values rather than inheriting a host
representation.
