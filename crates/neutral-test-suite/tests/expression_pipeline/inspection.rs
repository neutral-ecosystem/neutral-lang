// SPDX-License-Identifier: Apache-2.0

//! Reader-only public entry discovery and probing never require evaluation bindings.

use super::*;
use neutral_ir::project_interface::ProjectPublicType as T;
use neutral_probe::expression::{
    ExpressionEntryProbeRequest, ExpressionProbeError, inspect_expression_encoded,
};
use neutral_reader::consumer::{ENTRY_REQUEST_SCHEMA, EntryRequest, EntrySelection};
use neutral_vocabulary::composition;

/// Roots affect projection only, while exact required features reject before any public summary.
#[test]
#[allow(clippy::too_many_lines)] // Public parity and fail-closed recovery share the same complete artifact.
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
    let roots = [symbol("entry")];
    let direct = project
        .derive_view(
            &ExpressionViewRequest {
                schema: profile::PROJECT_VIEW_SCHEMA,
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(
        unique.view.declarations().collect::<Vec<_>>(),
        direct.declarations().collect::<Vec<_>>()
    );
    assert_eq!(
        unique.view.templates().collect::<Vec<_>>(),
        direct.templates().collect::<Vec<_>>()
    );
    assert_eq!(
        unique.view.availability_obligations().collect::<Vec<_>>(),
        direct.availability_obligations().collect::<Vec<_>>()
    );
    assert_eq!(
        unique.view.restriction_obligations().collect::<Vec<_>>(),
        direct.restriction_obligations().collect::<Vec<_>>()
    );
    assert_eq!(
        unique
            .view
            .dependencies(&roots[0], ExpressionInspectionLimits::default(), &cancel)
            .unwrap(),
        direct
            .dependencies(&roots[0], ExpressionInspectionLimits::default(), &cancel)
            .unwrap()
    );
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
    let missing_contract = T::Nominal(symbol("MissingContract"));
    let mut missing = request(EntrySelection::Unique);
    missing.entry.contract = &missing_contract;
    assert!(matches!(
        inspect_expression_encoded(
            &bytes,
            DecodeLimits::hard(),
            project.model().limits(),
            &missing,
            &cancel
        ),
        Err(ExpressionProbeError::Entry(_))
    ));
    for selector in [profile::EVALUATOR_PROFILE, profile::REQUIRED_FEATURES[0]] {
        let mut forged = bytes.clone();
        let start = forged
            .windows(selector.len())
            .position(|w| w == selector.as_bytes())
            .unwrap();
        forged[start + selector.len() - 1] = b'9';
        assert!(matches!(
            inspect_expression_encoded(
                &forged,
                DecodeLimits::hard(),
                project.model().limits(),
                &request(EntrySelection::Unique),
                &cancel
            ),
            Err(ExpressionProbeError::Decode(_))
        ));
    }
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

/// Vocabulary restrictions and guarded ports remain complete after standalone inspection.
#[test]
fn integration_expression_probe_preserves_vocabulary_interpretation_contracts() {
    let project = super::consumer::vocabulary_project(
        include_bytes!("fixtures/vocabulary.neu"),
        include_bytes!("fixtures/domain.json"),
    );
    let cancel = CancellationToken::new();
    let bytes = encode_expression_project(&project, &cancel).unwrap();
    let contract = T::VocabularyNominal {
        identity: "PipelineDomain".into(),
        version: "1.0.0".into(),
        name: "Plan".into(),
    };
    let request = ExpressionEntryProbeRequest {
        semantic_profiles: &project.ir().semantic_profiles,
        entry: EntryRequest {
            schema: ENTRY_REQUEST_SCHEMA,
            contract: &contract,
            selection: EntrySelection::Unique,
        },
        limits: ExpressionViewLimits::default(),
    };
    let summary = inspect_expression_encoded(
        &bytes,
        DecodeLimits::hard(),
        project.model().limits(),
        &request,
        &cancel,
    )
    .unwrap();
    let entry = symbol("entry");
    let direct = project
        .derive_view(
            &ExpressionViewRequest {
                schema: profile::PROJECT_VIEW_SCHEMA,
                semantic_profiles: &project.ir().semantic_profiles,
                roots: std::slice::from_ref(&entry),
            },
            &cancel,
        )
        .unwrap();
    let expected = direct.vocabularies().next().unwrap();
    let actual = summary.view.vocabularies().next().unwrap();
    assert_eq!(
        (actual.identity(), actual.version(), actual.schema_version()),
        ("PipelineDomain", "1.0.0", composition::SCHEMA_VERSION)
    );
    assert_eq!(
        actual.definitions().collect::<Vec<_>>(),
        expected.definitions().collect::<Vec<_>>()
    );
    assert_eq!(actual.dependencies(), expected.dependencies());
    assert_eq!(
        summary.view.restriction_obligations().collect::<Vec<_>>(),
        direct.restriction_obligations().collect::<Vec<_>>()
    );
    assert_eq!(
        summary.view.availability_obligations().collect::<Vec<_>>(),
        direct.availability_obligations().collect::<Vec<_>>()
    );
    let edges = summary
        .view
        .dependencies(&entry, ExpressionInspectionLimits::default(), &cancel)
        .unwrap();
    assert_eq!(edges.edges().len(), 2);
    assert_eq!(
        edges.edges()[1].guards[0].condition,
        GuardCondition::Boolean(true)
    );
}
