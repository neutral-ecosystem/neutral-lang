<!-- SPDX-License-Identifier: Apache-2.0 -->

# Consumer envelopes

This module owns the closed **data-only** description/report boundary over a
`ValidatedCompositionProject`, including independently decoded artifacts. It
borrows checked public payloads, not unvalidated JSON bags. Exact nominal contracts
include vocabulary identity and revision. Feature/code registries are explicit,
sorted and unique; consumer codes cannot use the reserved `NEU-` namespace.

This example expects the public types and binding in the file-backed
[consumer.neu](../../../neutral-test-suite/tests/composition_contract/fixtures/source/consumer.neu).

```rust
use neutral_core::CancellationToken;
use neutral_ir::{LogicalModuleIdentity, ModuleSymbolIdentity, project_interface::ProjectPublicType};
use neutral_reader::{composition::ValidatedCompositionProject, consumer::*};

fn inspect(project: &ValidatedCompositionProject) -> Result<(), ConsumerError> {
    let owner = |name| ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "example"), name);
    let report_contract = ProjectPublicType::Nominal(owner("Receipt"));
    let payload_contract = ProjectPublicType::Nominal(owner("Capability"));
    let payload = owner("description");
    let cancellation = CancellationToken::new();
    let boundary = ConsumerBoundary::new(project, ConsumerProtocol {
        report_contract: &report_contract,
        supported_features: &["emit"],
        diagnostic_codes: &["EXAMPLE-UNSUPPORTED"],
    }, ConsumerLimits::default(), &cancellation)?;
    let description = DescriptionRequest {
        schema: DESCRIPTION_SCHEMA,
        contract: &payload_contract,
        adapter_revision: "adapter-1",
        target_revision: "target-1",
        required_features: &["emit"],
        payload: &payload,
    };
    boundary.description(&description, &cancellation)?;
    let path = [MemberSegment::Field("labels"), MemberSegment::Index(0)];
    let selector = MemberSelector {
        schema: MEMBER_SELECTOR_SCHEMA,
        root: SelectorRoot::Value(&payload),
        path: &path,
        expected_type: &ProjectPublicType::String,
    };
    let selected = boundary.select(&selector, &cancellation)?;
    assert_eq!(selected.resolved_type(), &ProjectPublicType::String);
    let origin = boundary.value_origin(&ValueOriginRequest {
        schema: VALUE_ORIGIN_SCHEMA, symbol: &payload, path: &path,
    }, &cancellation)?;
    assert_eq!(origin.presence(), neutral_ir::composition::ValueOriginKind::Defaulted);
    let report = ReportRequest {
        schema: REPORT_SCHEMA,
        contract: &report_contract,
        project_identity: boundary.project_identity(),
        entry: None,
        context: ConsumerContext {
            adapter_revision: "adapter-1",
            target_revision: "target-1",
            input_revision: "inputs-1",
        },
        status: ConsumerStatus::Accepted,
        diagnostics: &[],
    };
    assert!(boundary.report(&report, &cancellation)?.is_accepted());
    let alternative = ReportRequest { status: ConsumerStatus::AlternativeRequired, ..report };
    assert!(!boundary.report(&alternative, &cancellation)?.is_accepted());
    let rejected = ReportRequest { status: ConsumerStatus::Rejected, ..report };
    assert!(!boundary.report(&rejected, &cancellation)?.is_accepted());
    Ok(())
}
```

Reports bind to independently recomputed **complete logical identity**, not source,
interface or view identity. Consumer revisions and entry selection cannot change
project identity. A checked report validates its envelope, not the truth of the
consumer's decision, authorization or target availability. An alternative requires
explicit host approval and a new validated context; it cannot be auto-approved here.

Subjects must be public bindings; paths name retained materialized occurrences.
Available primary/related locations must exactly match checked public source
maps/origins and fall inside public declaration spans, never private reuse origins.
Unavailable/redacted evidence is explicit. This is not source-text
authentication. Supply only public, non-secret opaque
revision IDs: validation cannot detect secrets inside arbitrary caller strings.
Errors and validated-envelope debug output contain no supplied text or payloads.

Positive independent controls intersect hard ceilings. Identity preprocessing and
each request also respect the effective producer/reader work policy and have
separate bounded accounting. Existing validated payloads are
borrowed; authority scans charge work and envelope data charges conservative bytes.
Diagnostic indexing is fallibly allocated; canonical sorting is allocation-free.
Cancellation/allocation failure publishes no partial result and never mutates the
reader. Output-byte limits apply to the retained index, not transport serialization.

This typed API is **not a JSON decoder**. Transport owners must reject unknown
fields and bound actual encoded input/output bytes separately. Full
`neutral.consumer/1` availability is not advertised: entry discovery, full feature
negotiation, evaluators, adapters, schedulers and effects remain outside this module.
Run `cargo xtask test all` or `cargo test -p neutral-test-suite consumer::`.

## Typed selectors and value origins

`ConsumerBoundary::select` implements `neutral.member-selector/1`: exact public
binding/type roots and field, unsigned index and named alternative segments.
Expected types are checked against validated contracts; they never confer lookup
authority. No nullable unwrapping, reference following or deferred evaluation is
implicit. Type-mode requests retain optional-presence, index and alternative
availability obligations; they cannot promise a future result exists. Value mode
borrows materialized values, with `None` for an omitted optional terminal. Use
`identity()` for the canonical typed tuple, not debug output or JSON spelling.
External selections do not change project identity or historical transcripts.

`value_origin` uses the same checked value traversal, preserving supplied,
explicit-null, omitted and defaulted classifications independently of meaning.
Default facts retain exact public source/vocabulary owner and field; vocabulary
evidence is separately checked by the independent reader. Reuse origins expose
public locations only and explicitly redact private locations. An origin span
is not a consuming use-site span: root reuse uses its unique checked direct
provenance occurrence. Where the producer did not retain a separate child or
omission occurrence, it is explicitly `NotRetained`, never a fabricated
declaration-wide location. Unknown origin ownership remains unavailable.

Requests share the independent positive consumer controls and intersected reader
work policy. Path/name, location and output bounds are checked before publication;
retained projection structs and availability arrays respect `output_bytes`
(serialized bytes remain a transport-owner limit). Availability obligations use
fallible allocation. Cancellation or failure returns
no partial selection and never mutates the reader. JSON transport owners still
must reject unknown fields and invalid index representations before these typed
APIs. Run `cargo test -p neutral-test-suite selectors::` for decoded-artifact,
privacy, boundary and allocation/cancellation regression tests.
