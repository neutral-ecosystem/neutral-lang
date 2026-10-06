<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-reader

`neutral-reader` validates external Neutral artifacts and exposes immutable,
typed reader views over accepted logical data.

It depends on core, IR, and vocabulary contracts. It does not acquire artifacts
or perform vocabulary lookup: callers supply the bytes and the exact captured
vocabulary contract. The CLI and standalone probe use this boundary to inspect
artifacts without depending on compiler internals.

It also exposes the shared deterministic language-profile catalogue used by
hosts for compatibility discovery. This catalogue comes from `neutral-core`,
so reader and CLI reporting cannot drift into separate version tables.

The active reader exposes validated in-process compiler artifacts through
immutable scalar, nominal-record, and qualified vocabulary traversal; recursive
type/value/default-provenance validation; exact vocabulary-contract validation;
and indexed source lookup. `neutral-encoding` owns hostile framed-byte decoding
and exposes a reader view only after the complete external artifact validates.
The separate `ValidatedProjectInterface` checks a public-only,
in-process interface snapshot without compiler linkage. It exposes public
exports, checked signature fingerprints, and cross-module value/reference
enumeration; no source IDs, private declarations, or raw provenance are part
of that view. It is not a complete project-IR reader.
The project-interface reader validates URL/path and canonical vocabulary-nominal
signature variants against the public vocabulary catalogue retained in the
snapshot. It checks canonical ordering, identity/revision, public type membership,
and fingerprints independently of the compiler. The catalogue exposes canonical
identities and revisions; source-local aliases and authoring metadata are excluded.

`ValidatedProject::from_ir` is the complete in-process project boundary. It
independently checks every module/declaration/type/value/default, locked schemas,
source-map and provenance coverage, dependency cycles, resource accounting,
and the exact public export index. Caller and producer bounds intersect;
cancellation and malformed content fail before publication. It depends on no
compiler-private parser or AST. `derive_view` selects public roots and their
type/value/reference/vocabulary closure without exposing private identities or
source accounting, or mutating complete IR. Source evidence is not a proof that
an arbitrary producer faithfully compiled the supplied bytes. The separate
`neutral-encoding::project` codec returns this validated boundary after hostile
wire/schema checks; the standalone project probe then exposes a selected public
view without compiler linkage. Captured artifact-byte limits are independent
of in-process value work and are enforced by the codec.

`ValidatedProject::logical_identity` exposes the bounded complete logical
transcript and typed digest. `identities` additionally binds explicit capture
locks and producer facts to the validated source/vocabulary companions and
returns distinct captured, logical, and derivation identities with processing
facts. These caller claims are not producer authentication or proof of faithful
compilation. `ProjectIdentities::artifact` checks public/existing view roots
before publishing a typed artifact identity. Roots cannot mutate complete
transcripts; a public interface fingerprint is not a complete logical identity.

`composition::CompositionCatalogue` inspects a separately validated composite
vocabulary catalogue without compiler linkage. Public lookup returns complete
record/variant contracts, restrictions/defaults and exact dependencies; private
and missing types are indistinguishable to callers, including diagnostic debug
output. Bounded `reference_types` includes unselected variant alternatives and
list/nullable wrappers. `materialize` checks closed supplied values with the
vocabulary validator and exposes safe origin classifications, not invented source
spans or binding edges. This component does not imply support in the existing project codec or
compiled-project identity profiles.
`from_shared` accepts an immutable already-validated catalogue without copying
contracts or reparsing bytes. Only the validator can construct that catalogue;
sharing does not bypass validation or expose mutable private contracts.

`CompositionTypeCatalogue` additionally inspects a validated resolved scope of
source- and vocabulary-owned records/variants. It filters private source types,
shares closed-value validation and enumerates exact reference-type dependencies
without compiler linkage. This is a type catalogue, not complete successor
project IR, a public view or source provenance.

## Command

Use `cargo xtask test all` for complete repository validation, including doctests.
The focused Cargo command below runs this crate's test binaries only.

This is a library crate. Verify its public artifact-reading contracts with:

```sh
cargo test --package neutral-reader
```
