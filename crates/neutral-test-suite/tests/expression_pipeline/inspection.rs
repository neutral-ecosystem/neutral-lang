// SPDX-License-Identifier: Apache-2.0

//! Reader-only public entry discovery and probing never require evaluation bindings.

use super::*;
use neutral_ir::project_interface::ProjectPublicType as T;
use neutral_probe::expression::{ExpressionEntryProbeRequest, inspect_expression_encoded};
use neutral_reader::consumer::{ENTRY_REQUEST_SCHEMA, EntryRequest, EntrySelection};

/// Roots affect projection only, while exact required features reject before any public summary.
#[test]
fn integration_expression_probe_is_independent_of_roots_and_external_bindings() {
    let (bytes, project) = pipeline(include_bytes!("fixtures/guarded.neu"));
    let cancel = CancellationToken::new();
    let contract = T::Nominal(symbol("Plan"));
    let request = |selection| ExpressionEntryProbeRequest {
        semantic_profiles: &project.ir().semantic_profiles,
        entry: EntryRequest {
            schema: ENTRY_REQUEST_SCHEMA,
            contract: &contract,
            selection,
        },
        limits: ExpressionViewLimits::default(),
    };
    let unique = inspect_expression_encoded(
        &bytes,
        DecodeLimits::hard(),
        project.model().limits(),
        &request(EntrySelection::Unique),
        &cancel,
    )
    .unwrap();
    let empty = inspect_expression_encoded(
        &bytes,
        DecodeLimits::hard(),
        project.model().limits(),
        &request(EntrySelection::Explicit(&[])),
        &cancel,
    )
    .unwrap();
    assert_eq!(unique.logical_identity, empty.logical_identity);
    assert_eq!(unique.view.roots().collect::<Vec<_>>(), [&symbol("entry")]);
    assert_eq!(empty.view.roots().count(), 0);
    assert_eq!(empty.view.declarations().count(), 0);
    assert_eq!(unique.resources.output_leaves, 1);
    let mut unsupported = SemanticProfiles::try_frozen().unwrap();
    unsupported.required_features.remove(0);
    let mut wrong = request(EntrySelection::Unique);
    wrong.semantic_profiles = &unsupported;
    assert!(
        inspect_expression_encoded(
            &bytes,
            DecodeLimits::hard(),
            project.model().limits(),
            &wrong,
            &cancel
        )
        .is_err()
    );
    for end in [0, 7, bytes.len() - 1] {
        assert!(
            inspect_expression_encoded(
                &bytes[..end],
                DecodeLimits::hard(),
                project.model().limits(),
                &request(EntrySelection::Unique),
                &cancel
            )
            .is_err()
        );
    }
    assert!(
        inspect_expression_encoded(
            &bytes,
            DecodeLimits::hard(),
            project.model().limits(),
            &request(EntrySelection::Unique),
            &cancel
        )
        .is_ok()
    );
}
