<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-cli

`neutral-cli` is the host-facing command-line adapter for capture, compilation,
validation, and formatting workflows.

It may own filesystem and process interactions required by a local command, but
it delegates pure compilation to `neutral-compiler` and artifact reading to
`neutral-reader`. It is not the independent probe artifact and must not make
host effects part of the captured compiler contract.

## Commands

- `neutral-cli compile --output <artifact|-> [options] <source|->` validates a
  captured source and writes a framed Neutral IR artifact.
- `neutral-cli validate [options] <source|->` validates without producing an
  authoritative output.
- `neutral-cli format --output <source|-> [options] <source|->` validates and
  writes canonical, ordinary Neutral source.

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
