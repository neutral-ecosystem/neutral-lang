// SPDX-License-Identifier: Apache-2.0

//! Capture /2 selection, phase precedence, exact bytes, alias scope and replay tests.

use super::super::{
    CAPTURE_REQUEST_VERSION, CapturedSourceInput, CapturedVocabularyInput, ProjectCaptureControls,
    ProjectCaptureLimitValues, capture_project,
};
use super::*;
use neutral_core::{SourceContentDigest, StructuralLimits, VocabularyContentDigest};
use neutral_vocabulary::{VocabularyLimits, VocabularyLock};

/// Creates explicit finite policy independent of package versions.
fn policies() -> (ProjectCaptureLimits, CompositionLimits) {
    (
        ProjectCaptureLimits::new(ProjectCaptureLimitValues {
            total_source_bytes: 4096,
            source_bytes_per_unit: 1024,
            source_units: 4,
            source_id_bytes: 64,
            module_id_bytes: 64,
            vocabulary_units: 4,
            vocabulary_bytes_per_unit: 4096,
            total_vocabulary_bytes: 16_384,
            imports_per_module: 8,
            import_edges: 16,
            scc_units: 4,
            declarations: 64,
            diagnostics: 16,
            output_bytes: 16_384,
        }),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(65_536, 64).unwrap(),
        )),
    )
}

/// Creates a closed successor request whose body is deliberately not yet compiled.
fn request() -> CapturedCompositionProjectRequest {
    let (capture, composition) = policies();
    CapturedCompositionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "source:example",
                "example",
                b"neu \"1.0\"\nmodule example\n".to_vec(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(capture, CancellationToken::new()),
        ),
        profile::REQUIRED_FEATURES
            .iter()
            .map(|feature| (*feature).to_owned())
            .collect(),
        composition,
    )
}

/// Constructs exact explicit schema /2 bytes and their matching content lock.
fn vocabulary() -> CapturedVocabularyInput {
    let bytes = br#"{
  "format": "neutral-vocabulary-bundle",
  "encoding_version": "1.0",
  "schema_version": "2.0",
  "identity": "ExampleDomain",
  "version": "1.0.0",
  "required_features": ["vocabulary-composition-v2"],
  "dependencies": [],
  "types": [
    {
      "kind": "record",
      "name": "Item",
      "public": true,
      "fields": []
    }
  ]
}
"#
    .to_vec();
    let lock = VocabularyLock::new(
        "ExampleDomain",
        "1.0.0",
        neutral_vocabulary::composition::ENCODING_VERSION,
        profile::VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&bytes),
        vec![profile::VOCABULARY_COMPOSITION_FEATURE.to_owned()],
    )
    .unwrap();
    CapturedVocabularyInput::new(bytes, lock)
}

/// Selection must be exact before limits, cancellation or captured content are interpreted.
#[test]
fn unsupported_features_never_fallback_and_win_preflight() {
    for features in [
        Vec::new(),
        vec![profile::TAGGED_VARIANTS_FEATURE.to_owned()],
        profile::REQUIRED_FEATURES
            .iter()
            .rev()
            .map(|v| (*v).to_owned())
            .collect(),
        vec![profile::TAGGED_VARIANTS_FEATURE.to_owned(); 2],
        vec!["unknown-feature".to_owned()],
    ] {
        let mut request = request();
        request.required_features = features;
        request.composition_limits.work = 0;
        request.envelope.controls.cancellation.cancel();
        assert_eq!(
            capture_composition_project(request).unwrap_err().code(),
            ProjectCaptureError::InvalidRequest.code()
        );
    }
    let mut request = request();
    request.envelope.request_version = CAPTURE_REQUEST_VERSION.to_owned();
    assert_eq!(
        capture_composition_project(request.clone())
            .unwrap_err()
            .code(),
        ProjectCaptureError::InvalidRequest.code()
    );
    request.envelope.request_version = profile::CAPTURE_REQUEST_VERSION.to_owned();
    assert_eq!(
        capture_project(request.envelope).unwrap_err(),
        ProjectCaptureError::InvalidRequest
    );
}

/// Each independent zero composition budget rejects before an already-cancelled request.
#[test]
fn every_zero_composition_budget_precedes_cancellation() {
    for index in 0..15 {
        let mut request = request();
        let limits = &mut request.composition_limits;
        *match index {
            0 => &mut limits.bundles,
            1 => &mut limits.captured_bytes,
            2 => &mut limits.dependencies_per_bundle,
            3 => &mut limits.dependency_edges,
            4 => &mut limits.dependency_depth,
            5 => &mut limits.total_types,
            6 => &mut limits.total_fields,
            7 => &mut limits.alternatives_per_type,
            8 => &mut limits.total_alternatives,
            9 => &mut limits.choices_per_field,
            10 => &mut limits.total_choices,
            11 => &mut limits.type_depth,
            12 => &mut limits.value_depth,
            13 => &mut limits.value_nodes,
            _ => &mut limits.work,
        } = 0;
        request.envelope.controls.cancellation.cancel();
        assert_eq!(
            capture_composition_project(request).unwrap_err().code(),
            ProjectCaptureError::LimitExceeded.code()
        );
    }
}

/// Cancellation at every phase prevents publication of even an otherwise valid capture.
#[test]
fn cancellation_at_each_handoff_is_whole_request_failure() {
    for phase in [
        ProjectCaptureCheckpoint::Start,
        ProjectCaptureCheckpoint::Integrity,
        ProjectCaptureCheckpoint::Headers,
        ProjectCaptureCheckpoint::Publish,
    ] {
        let request = request();
        let signal = request.envelope.controls.cancellation.clone();
        let result = capture_with_checkpoints(request, |checkpoint| {
            if checkpoint == phase {
                signal.cancel();
            }
        });
        assert_eq!(
            result.unwrap_err().code(),
            ProjectCaptureError::Cancelled.code()
        );
    }
}

/// The complete catalogue is canonical while repeated aliases remain source-local.
#[test]
fn repeated_identity_aliases_share_one_validated_lock() {
    let mut request = request();
    request.envelope.sources[0]
        .bytes
        .extend_from_slice(b"use ExampleDomain as first\nuse ExampleDomain as second\n");
    request.envelope.vocabularies.push(vocabulary());
    let captured = capture_composition_project(request).unwrap();
    assert_eq!(captured.vocabularies().len(), 1);
    assert_eq!(captured.catalogue().bundles().len(), 1);
    assert_eq!(
        captured.vocabulary_for_alias("example", "first"),
        captured.vocabulary_for_alias("example", "second")
    );
    assert_eq!(captured.vocabulary_for_alias("other", "first"), None);
    assert_eq!(
        captured.vocabulary_for_alias("example", "ExampleDomain"),
        None
    );
    assert!(captured.module_graph(&CancellationToken::new()).is_ok());
    let replay =
        capture_composition_project(captured.replay_request(CancellationToken::new())).unwrap();
    assert_eq!(captured.sources(), replay.sources());
    assert_eq!(captured.vocabularies(), replay.vocabularies());
    assert_eq!(captured.catalogue(), replay.catalogue());
    assert_eq!(captured.composition_limits(), replay.composition_limits());
}

/// Duplicate aliases cannot overwrite an earlier module-local namespace mapping.
#[test]
fn duplicate_alias_fails_before_catalogue_publication() {
    let mut request = request();
    request.envelope.sources[0]
        .bytes
        .extend_from_slice(b"use ExampleDomain as domain\nuse ExampleDomain as domain\n");
    request.envelope.vocabularies.push(vocabulary());
    assert_eq!(
        capture_composition_project(request).unwrap_err().code(),
        ProjectCaptureError::InvalidHeader.code()
    );
}

/// Exact integrity failures remain capture failures, not schema or semantic diagnostics.
#[test]
fn changed_source_or_vocabulary_bytes_reject_exact_pins() {
    let mut source_request = request();
    source_request.envelope.sources[0].expected_digest =
        Some(SourceContentDigest::from_bytes(b"different"));
    assert_eq!(
        capture_composition_project(source_request)
            .unwrap_err()
            .code(),
        ProjectCaptureError::IntegrityMismatch.code()
    );
    let mut bundle_request = request();
    bundle_request.envelope.sources[0]
        .bytes
        .extend_from_slice(b"use ExampleDomain as domain\n");
    let mut input = vocabulary();
    input.bytes.push(b' ');
    bundle_request.envelope.vocabularies.push(input);
    assert_eq!(
        capture_composition_project(bundle_request)
            .unwrap_err()
            .code(),
        ProjectCaptureError::IntegrityMismatch.code()
    );
}

/// Missing direct requirements and duplicate locks retain their original capture classifications.
#[test]
fn direct_lock_failures_are_distinct_from_transitive_schema_errors() {
    let mut request = request();
    request.envelope.sources[0]
        .bytes
        .extend_from_slice(b"use ExampleDomain as domain\n");
    assert_eq!(
        capture_composition_project(request.clone())
            .unwrap_err()
            .code(),
        ProjectCaptureError::MissingVocabulary.code()
    );
    request.envelope.vocabularies = vec![vocabulary(), vocabulary()];
    assert_eq!(
        capture_composition_project(request).unwrap_err().code(),
        ProjectCaptureError::DuplicateVocabulary.code()
    );
}

/// Debug output is safe by construction rather than by later best-effort redaction.
#[test]
fn debug_output_never_contains_source_or_correlation_text() {
    let mut request = request();
    request.envelope.project_key = Some("private-host-correlation".to_owned());
    request.envelope.sources[0]
        .bytes
        .extend_from_slice(b"// sensitive-source-body\n");
    for output in [
        format!("{request:?}"),
        format!("{:?}", capture_composition_project(request).unwrap()),
    ] {
        assert!(!output.contains("sensitive-source-body"));
        assert!(!output.contains("private-host-correlation"));
        assert!(!output.contains("source:example"));
    }
}

/// Aggregate captured bytes pass exactly and fail one over before schema work.
#[test]
fn aggregate_catalogue_byte_limit_is_independent_of_capture_policy() {
    let mut request = request();
    request.envelope.sources[0]
        .bytes
        .extend_from_slice(b"use ExampleDomain as domain\n");
    let input = vocabulary();
    request.composition_limits.captured_bytes = input.bytes.len() as u64;
    request.envelope.vocabularies.push(input);
    assert!(capture_composition_project(request.clone()).is_ok());
    request.composition_limits.captured_bytes -= 1;
    assert_eq!(
        capture_composition_project(request).unwrap_err(),
        CompositionCaptureFailure::Catalogue(CompositionError::Limit)
    );
}

/// Oversized bundle counts fail before integrity/duplicate checks and proportional work.
#[test]
fn aggregate_bundle_count_precedes_lock_hashing_and_duplicate_checks() {
    let mut request = request();
    request.composition_limits.bundles = 1;
    request.envelope.vocabularies = vec![vocabulary(), vocabulary()];
    assert_eq!(
        capture_composition_project(request).unwrap_err(),
        CompositionCaptureFailure::Catalogue(CompositionError::Limit)
    );
}

/// Capture still enforces required source/header controls despite a permissive catalogue policy.
#[test]
fn source_envelope_and_header_checks_are_not_bypassed() {
    let mut empty = request();
    empty.envelope.sources.clear();
    assert_eq!(
        capture_composition_project(empty).unwrap_err().code(),
        ProjectCaptureError::EmptySourceSet.code()
    );
    let mut mismatched = request();
    mismatched.envelope.sources[0].module_id = "different".into();
    assert_eq!(
        capture_composition_project(mismatched).unwrap_err().code(),
        ProjectCaptureError::ModuleHeaderMismatch.code()
    );
    let mut profile = request();
    profile.envelope.sources[0].bytes = b"neu \"0.1\"\nmodule example\n".to_vec();
    assert_eq!(
        capture_composition_project(profile).unwrap_err().code(),
        ProjectCaptureError::ProfileMismatch.code()
    );
}

/// Commented requirements cannot extend closure, create aliases or trigger host acquisition.
#[test]
fn block_commented_requirements_are_not_capture_dependencies() {
    let mut request = request();
    request.envelope.sources[0]
        .bytes
        .extend_from_slice(b"/*\nuse Uncaptured as hidden\n*/\nuse ExampleDomain as actual\n");
    request.envelope.vocabularies.push(vocabulary());
    let captured = capture_composition_project(request).unwrap();
    assert_eq!(captured.vocabulary_for_alias("example", "hidden"), None);
    assert_eq!(
        captured
            .vocabulary_for_alias("example", "actual")
            .unwrap()
            .identity(),
        "ExampleDomain"
    );
}

/// The old compiler boundary must reject successor schema bytes rather than retrying new validation.
#[test]
fn old_project_vocabulary_validation_does_not_fallback_to_composition() {
    let mut request = request();
    request.envelope.sources[0]
        .bytes
        .extend_from_slice(b"use ExampleDomain as domain\n");
    request.envelope.vocabularies.push(vocabulary());
    CAPTURE_REQUEST_VERSION.clone_into(&mut request.envelope.request_version);
    let old_capture = capture_project(request.envelope).unwrap();
    assert_eq!(
        crate::validate_project_vocabularies(&old_capture),
        Err(crate::ProjectVocabularyValidationError::InvalidBundle(
            // The frozen /1 member-set check rejects /2 `dependencies` before
            // checking its schema selector. Preserve that existing precedence.
            neutral_vocabulary::VocabularyError::UnknownMember
        ))
    );
}
