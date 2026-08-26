# Section 9: security and limits

## No authority-bearing syntax

v0 has no secret reference, credential, signature, policy, permission, command,
filesystem, environment, or network syntax. A quoted string is always ordinary
text and grants no authority.

## No execution during compilation

Source and vocabulary validation execute no native code, script, callback,
bytecode, plugin, or application operation. Vocabulary bundles are closed data.

## Explicit capture boundary

The host supplies the only resolver. Source has no `include(path)`, URL import,
ambient profile search, current-directory expansion, or registry lookup.

```text
capture(request) -> CapturedCompilation
compileCaptured(captured) -> CompilationResult
```

Resolver credentials remain behind the host boundary. Missing source or
vocabulary input fails closed rather than selecting a fallback.

## Safe diagnostics

Diagnostics separate stable code, safe parameters, source spans, and optional
excerpt. They escape control characters and report logical source identities,
not accidental host paths. Resolver credentials and host policy inputs never
appear in ordinary rendering.

## Structural limits

Compiler and reader APIs receive explicit versioned limits for:

- source and bundle bytes;
- string length and nesting;
- declaration, field, list-item, and reference counts;
- numeric significant digits and decimal scale;
- IR bytes/elements and traversal work; and
- diagnostic count and safe rendered size.

Checks occur before proportional allocation or conversion. Crossing a limit
produces a bounded diagnostic and no authoritative IR.

Wall-clock deadlines, external cancellation, and physical memory ceilings are
implementation/deployment controls, not reproducible language semantics.
