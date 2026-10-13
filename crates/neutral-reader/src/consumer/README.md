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
Unavailable/redacted evidence is explicit. This is not general selector
traversal or source-text authentication. Supply only public, non-secret opaque
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
`neutral.consumer/1` availability is not advertised: general selectors, entry
discovery, evaluators, adapters, schedulers and effects remain outside this module.
Run `cargo xtask test all` or `cargo test -p neutral-test-suite consumer::`.
