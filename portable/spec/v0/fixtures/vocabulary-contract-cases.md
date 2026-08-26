# Neutral v0 vocabulary contract cases

The fixture source uses one logical name, `Fixture`. The host supplies one exact
captured bundle and lock record.

## Required success

| Case | Expected result |
| --- | --- |
| Exact permitted identity, digest, and schema version | Compile and record exact facts in IR/derivation. |
| `Fixture::Metadata` with all required typed fields | Accept and preserve the qualified nominal type. |
| Omitted field with a closed vocabulary default | Apply final value and record default provenance. |
| External reader receives the exact captured contract | Validate without external I/O. |

## Required failure

| Case | Expected result |
| --- | --- |
| `use Fixture` with no captured lock entry | Fail closed. |
| Captured digest differs from lock entry | Integrity failure. |
| Unknown qualified type | Name/type diagnostic. |
| Missing, duplicate, unknown, or incompatible payload field | Vocabulary payload diagnostic. |
| Unknown required structural feature | Unsupported-feature failure. |
| Unknown field in the closed bundle schema | Bundle-schema failure. |
| Script, callback, bytecode, native module, or executable entry point | Bundle-schema failure before payload validation. |
| Encoded IR decoded without the exact captured vocabulary contract | Reader validation failure; no hidden lookup. |

## Required identity behavior

- Mutable names and paths are provenance, not bundle identity.
- Published schema/feature IDs cannot change meaning in place.
- Producer build and source-map differences do not alter logical payload
  equality.
- Vocabulary-owned values remain inspectable typed data; the generic probe does
  not assign application meaning.
