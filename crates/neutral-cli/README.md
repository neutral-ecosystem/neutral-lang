<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-cli

`neutral-cli` is the host-facing command-line adapter for capture, compilation,
validation, and formatting workflows.

It may own filesystem and process interactions required by a local command, but
it delegates pure compilation to `neutral-compiler` and artifact reading to
`neutral-reader`. It is not the independent probe artifact and must not make
host effects part of the captured compiler contract.

## Commands

Run the locally built command and display its interface with:

```sh
cargo run --package neutral-cli -- --help
```

- `neutral-cli compile --output <artifact|-> [options] <source|->` validates a
  captured source and writes a framed Neutral IR artifact.
- `neutral-cli validate [options] <source|->` validates without producing an
  authoritative output.
- `neutral-cli format --output <source|-> [options] <source|->` validates and
  writes canonical, ordinary Neutral source.
- `neutral-cli profiles` reports every recognized source profile, its current
  availability, and stable implemented capabilities without reading a source.

Use `-` for one selected standard stream. Source and vocabulary data cannot
both read standard input. Status messages and safe diagnostics always use
standard error, with `[info]` or `[error]` prefixes, so standard output remains
clean when it carries an artifact or formatted source.

Existing files are protected unless `--overwrite` is explicit. File output is
written, synchronized, and committed from a same-directory temporary file; a
failed or cancelled operation does not publish partial authoritative output.
Host acquisition failures do not echo paths or source content.

Captured vocabulary is opt-in and exact: `--vocabulary-bundle` must be paired
with identity, version, encoding-version, schema-version, and digest lock
options. Structural ceilings have explicit `--max-*` options and bounded
reviewed defaults. Run `neutral-cli <command> --help` for stable command usage.

## Options

- `--output <path|->`: Output destination (for `compile` and `format`).
- `--overwrite`: Overwrite existing output file if present.
- `--cancel-before-start`: Cooperative pre-work cancellation flag.
- `--vocabulary-bundle <path|->`: Path to host-captured vocabulary bundle JSON.
- `--vocabulary-identity <string>`: Exact vocabulary lock identity.
- `--vocabulary-version <string>`: Exact vocabulary lock release version.
- `--vocabulary-encoding-version <string>`: Exact vocabulary lock encoding version.
- `--vocabulary-schema-version <string>`: Exact vocabulary lock schema version.
- `--vocabulary-digest <sha256>`: Exact vocabulary lock SHA-256 digest.
- `--vocabulary-feature <string>`: Repeatable required vocabulary feature flag.
- `--max-source-bytes <n>`: Source byte limit (default: 16,777,216).
- `--max-diagnostics <n>`: Retained diagnostic count limit (default: 64).
- `--max-string-bytes <n>`: Decoded string byte limit (default: 1,048,576).
- `--max-numeric-digits <n>`: Exact number digit limit (default: 1,000,000).
- `--max-numeric-scale <n>`: Exact number scale limit (default: 1,000,000).
- `--max-declarations <n>`: Root declaration limit (default: 100,000).
- `--max-record-fields <n>`: Record field limit (default: 100,000).
- `--max-nesting-depth <n>`: Value nesting depth limit (default: 128).
- `--max-list-items <n>`: List item count limit (default: 1,000,000).
- `--max-traversal-nodes <n>`: Value traversal node limit (default: 1,000,000).
- `--help`: Display command usage help.
- `--version`: Display version information.

## Exit classes

| Code | Meaning |
| ---: | --- |
| 0 | Success or requested help/version |
| 2 | Invalid command usage |
| 3 | Host input acquisition failed |
| 4 | Source, vocabulary, or artifact validation failed |
| 5 | Output creation, writing, or commit failed |
| 6 | Compilation was cancelled |
| 70 | Internal invariant failure |

Artifact inspection intentionally belongs to the separately built
`neutral-probe`, whose dependency graph excludes the compiler.
