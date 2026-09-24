// SPDX-License-Identifier: Apache-2.0

//! Core Stage 2 project-capture tests.

use super::*;
use neutral_core::VocabularyContentDigest;
use neutral_vocabulary::{VOCABULARY_ENCODING_VERSION, VOCABULARY_SCHEMA_VERSION};

/// Returns complete small, explicit project limits.
fn limits() -> ProjectCaptureLimits {
    ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 1024,
        source_units: 4,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 2,
        vocabulary_bytes_per_unit: 1024,
        total_vocabulary_bytes: 2048,
        imports_per_module: 8,
        import_edges: 16,
        scc_units: 4,
        declarations: 64,
        diagnostics: 16,
        output_bytes: 16_384,
    })
}

/// Creates a complete request from exact already-acquired data.
fn request(
    sources: Vec<CapturedSourceInput>,
    vocabularies: Vec<CapturedVocabularyInput>,
) -> CapturedProjectRequest {
    CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        sources,
        vocabularies,
        ProjectCaptureControls::new(limits(), CancellationToken::new()),
    )
}

/// Creates one request with caller-selected explicit limits.
fn request_with_limits(
    sources: Vec<CapturedSourceInput>,
    vocabularies: Vec<CapturedVocabularyInput>,
    values: ProjectCaptureLimitValues,
) -> CapturedProjectRequest {
    CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        sources,
        vocabularies,
        ProjectCaptureControls::new(ProjectCaptureLimits::new(values), CancellationToken::new()),
    )
}

/// Creates one exact vocabulary input whose bytes match its lock.
fn vocabulary(identity: &str, version: &str) -> CapturedVocabularyInput {
    let bytes = b"{}\n".to_vec();
    let lock = VocabularyLock::new(
        identity,
        version,
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&bytes),
        Vec::new(),
    )
    .expect("test lock must be valid");
    CapturedVocabularyInput::new(bytes, lock)
}

#[test]
fn retains_the_complete_supplied_closure_in_canonical_order() {
    let captured = capture_project(request(
        vec![
            CapturedSourceInput::new(
                "source:orphan",
                "project::orphan",
                b"neu \"1.0\"\nmodule project::orphan\n".to_vec(),
            ),
            CapturedSourceInput::new(
                "source:main",
                "project::main",
                b"neu \"1.0\"\nmodule project::main\n".to_vec(),
            ),
            CapturedSourceInput::new(
                "source:shared",
                "project::shared",
                b"neu \"1.0\"\nmodule project::shared\n".to_vec(),
            ),
        ],
        Vec::new(),
    ))
    .expect("complete supplied set must capture");
    let modules: Vec<_> = captured
        .sources()
        .iter()
        .map(CapturedProjectSource::module_id)
        .collect();
    assert_eq!(
        modules,
        ["project::main", "project::orphan", "project::shared"]
    );
}

#[test]
fn accepts_an_exact_vocabulary_cover_and_freezes_bytes() {
    let mut source =
        b"neu \"1.0\"\nmodule vocabulary::consumer\nuse ExampleDomain as domain\n".to_vec();
    let expected = SourceContentDigest::from_bytes(&source);
    let captured = capture_project(request(
        vec![
            CapturedSourceInput::new("source:vocabulary", "vocabulary::consumer", source.clone())
                .requiring_digest(expected),
        ],
        vec![vocabulary("ExampleDomain", "1.0.0")],
    ))
    .expect("exact cover must capture");
    source.fill(b'x');
    assert_eq!(captured.sources()[0].digest(), expected);
    assert_eq!(captured.vocabularies()[0].identity(), "ExampleDomain");
    assert_eq!(captured.vocabularies()[0].bytes(), b"{}\n");
}

#[test]
fn rejects_missing_extra_duplicate_and_mismatched_vocabulary_locks() {
    let using = || {
        CapturedSourceInput::new(
            "source:vocabulary",
            "vocabulary::consumer",
            b"neu \"1.0\"\nmodule vocabulary::consumer\nuse ExampleDomain as domain\n".to_vec(),
        )
    };
    assert_eq!(
        capture_project(request(vec![using()], Vec::new())),
        Err(ProjectCaptureError::MissingVocabulary)
    );
    let no_use = CapturedSourceInput::new(
        "source:plain",
        "project::plain",
        b"neu \"1.0\"\nmodule project::plain\n".to_vec(),
    );
    assert_eq!(
        capture_project(request(
            vec![no_use],
            vec![vocabulary("ExampleDomain", "1.0.0")]
        )),
        Err(ProjectCaptureError::ExtraVocabulary)
    );
    assert_eq!(
        capture_project(request(
            vec![using()],
            vec![
                vocabulary("ExampleDomain", "1.0.0"),
                vocabulary("ExampleDomain", "1.1.0"),
            ]
        )),
        Err(ProjectCaptureError::DuplicateVocabulary)
    );
    let bytes = b"wrong".to_vec();
    let lock = VocabularyLock::new(
        "ExampleDomain",
        "1.0.0",
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(b"expected"),
        Vec::new(),
    )
    .expect("lock must be valid");
    assert_eq!(
        capture_project(request(
            vec![using()],
            vec![CapturedVocabularyInput::new(bytes, lock)]
        )),
        Err(ProjectCaptureError::IntegrityMismatch)
    );
}

#[test]
fn rejects_duplicate_id_header_limit_version_profile_and_cancellation_failures() {
    let first = || {
        CapturedSourceInput::new(
            "source:same",
            "duplicate::first",
            b"neu \"1.0\"\nmodule duplicate::first\n".to_vec(),
        )
    };
    let second = CapturedSourceInput::new(
        "source:same",
        "duplicate::second",
        b"neu \"1.0\"\nmodule duplicate::second\n".to_vec(),
    );
    assert_eq!(
        capture_project(request(vec![first(), second], Vec::new())),
        Err(ProjectCaptureError::DuplicateSourceId)
    );
    let mismatch = CapturedSourceInput::new(
        "source:mismatch",
        "requested::module",
        b"neu \"1.0\"\nmodule declared::module\n".to_vec(),
    );
    assert_eq!(
        capture_project(request(vec![mismatch], Vec::new())),
        Err(ProjectCaptureError::ModuleHeaderMismatch)
    );

    let mut zero = limits().values();
    zero.output_bytes = 0;
    let invalid_limits = CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![first()],
        Vec::new(),
        ProjectCaptureControls::new(ProjectCaptureLimits::new(zero), CancellationToken::new()),
    );
    assert_eq!(
        capture_project(invalid_limits),
        Err(ProjectCaptureError::LimitExceeded)
    );

    let wrong_version = CapturedProjectRequest::new(
        "neutral.capture/v2",
        LanguageProfile::V1_0,
        vec![first()],
        Vec::new(),
        ProjectCaptureControls::new(limits(), CancellationToken::new()),
    );
    assert_eq!(
        capture_project(wrong_version),
        Err(ProjectCaptureError::InvalidRequest)
    );

    let wrong_request_profile = CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V0_1,
        vec![first()],
        Vec::new(),
        ProjectCaptureControls::new(limits(), CancellationToken::new()),
    );
    assert_eq!(
        capture_project(wrong_request_profile),
        Err(ProjectCaptureError::ProfileMismatch)
    );

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![first()],
        Vec::new(),
        ProjectCaptureControls::new(limits(), cancellation),
    );
    assert_eq!(
        capture_project(cancelled),
        Err(ProjectCaptureError::Cancelled)
    );
}

#[test]
fn stable_capture_codes_cover_the_complete_failure_catalogue() {
    let errors = [
        ProjectCaptureError::InvalidRequest,
        ProjectCaptureError::ProfileMismatch,
        ProjectCaptureError::EmptySourceSet,
        ProjectCaptureError::DuplicateSourceId,
        ProjectCaptureError::DuplicateModuleId,
        ProjectCaptureError::ModuleHeaderMismatch,
        ProjectCaptureError::InvalidHeader,
        ProjectCaptureError::MissingVocabulary,
        ProjectCaptureError::ExtraVocabulary,
        ProjectCaptureError::DuplicateVocabulary,
        ProjectCaptureError::IntegrityMismatch,
        ProjectCaptureError::LimitExceeded,
        ProjectCaptureError::Cancelled,
    ];
    for (index, error) in errors.into_iter().enumerate() {
        assert_eq!(error.code(), format!("NEU-CAP-{:03}", index + 1));
    }
}

#[test]
fn every_capture_limit_is_required_and_source_unit_bounds_are_exact() {
    macro_rules! assert_zero_rejected {
        ($field:ident) => {{
            let mut values = limits().values();
            values.$field = 0;
            let request = CapturedProjectRequest::new(
                CAPTURE_REQUEST_VERSION,
                LanguageProfile::V1_0,
                vec![CapturedSourceInput::new(
                    "source:bounded",
                    "limit::bounded",
                    b"neu \"1.0\"\nmodule limit::bounded\n".to_vec(),
                )],
                Vec::new(),
                ProjectCaptureControls::new(
                    ProjectCaptureLimits::new(values),
                    CancellationToken::new(),
                ),
            );
            assert_eq!(
                capture_project(request),
                Err(ProjectCaptureError::LimitExceeded),
                "zero {} must fail",
                stringify!($field)
            );
        }};
    }
    assert_zero_rejected!(total_source_bytes);
    assert_zero_rejected!(source_bytes_per_unit);
    assert_zero_rejected!(source_units);
    assert_zero_rejected!(source_id_bytes);
    assert_zero_rejected!(module_id_bytes);
    assert_zero_rejected!(vocabulary_units);
    assert_zero_rejected!(vocabulary_bytes_per_unit);
    assert_zero_rejected!(total_vocabulary_bytes);
    assert_zero_rejected!(imports_per_module);
    assert_zero_rejected!(import_edges);
    assert_zero_rejected!(scc_units);
    assert_zero_rejected!(declarations);
    assert_zero_rejected!(diagnostics);
    assert_zero_rejected!(output_bytes);

    let first = CapturedSourceInput::new(
        "source:first",
        "limit::first",
        b"neu \"1.0\"\nmodule limit::first\n".to_vec(),
    );
    let second = CapturedSourceInput::new(
        "source:second",
        "limit::second",
        b"neu \"1.0\"\nmodule limit::second\n".to_vec(),
    );
    let mut exact_values = limits().values();
    exact_values.source_units = 2;
    let exact = CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![first.clone(), second.clone()],
        Vec::new(),
        ProjectCaptureControls::new(
            ProjectCaptureLimits::new(exact_values),
            CancellationToken::new(),
        ),
    );
    assert_eq!(
        capture_project(exact)
            .expect("exact source-unit bound must pass")
            .sources()
            .len(),
        2
    );
    exact_values.source_units = 1;
    let over = CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![first, second],
        Vec::new(),
        ProjectCaptureControls::new(
            ProjectCaptureLimits::new(exact_values),
            CancellationToken::new(),
        ),
    );
    assert_eq!(
        capture_project(over),
        Err(ProjectCaptureError::LimitExceeded)
    );
}

#[test]
fn rejects_profile_module_digest_and_malformed_header_disagreement() {
    let wrong_profile = CapturedSourceInput::new(
        "source:profile",
        "mismatch::profile",
        b"neu \"0.1\"\nmodule mismatch::profile\n".to_vec(),
    );
    assert_eq!(
        capture_project(request(vec![wrong_profile], Vec::new())),
        Err(ProjectCaptureError::ProfileMismatch)
    );
    let duplicate_module = vec![
        CapturedSourceInput::new(
            "source:first",
            "duplicate::module",
            b"neu \"1.0\"\nmodule duplicate::module\n".to_vec(),
        ),
        CapturedSourceInput::new(
            "source:second",
            "duplicate::module",
            b"neu \"1.0\"\nmodule duplicate::module\n".to_vec(),
        ),
    ];
    assert_eq!(
        capture_project(request(duplicate_module, Vec::new())),
        Err(ProjectCaptureError::DuplicateModuleId)
    );
    let digest_mismatch = CapturedSourceInput::new(
        "source:digest",
        "mismatch::digest",
        b"neu \"1.0\"\nmodule mismatch::digest\n".to_vec(),
    )
    .requiring_digest(SourceContentDigest::from_bytes(b"different"));
    assert_eq!(
        capture_project(request(vec![digest_mismatch], Vec::new())),
        Err(ProjectCaptureError::IntegrityMismatch)
    );
    let duplicate_header = CapturedSourceInput::new(
        "source:header",
        "invalid::header",
        b"neu \"1.0\"\nmodule invalid::header\nmodule invalid::header\n".to_vec(),
    );
    assert_eq!(
        capture_project(request(vec![duplicate_header], Vec::new())),
        Err(ProjectCaptureError::InvalidHeader)
    );
}

#[test]
fn cancellation_at_every_capture_checkpoint_prevents_publication() {
    for cancelled_at in [
        ProjectCaptureCheckpoint::Start,
        ProjectCaptureCheckpoint::Integrity,
        ProjectCaptureCheckpoint::Headers,
        ProjectCaptureCheckpoint::Publish,
    ] {
        let cancellation = CancellationToken::new();
        let request = CapturedProjectRequest::new(
            CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "source:cancel",
                "capture::cancel",
                b"neu \"1.0\"\nmodule capture::cancel\n".to_vec(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(limits(), cancellation.clone()),
        );
        assert_eq!(
            capture_project_with_checkpoints(request, |checkpoint| {
                if checkpoint == cancelled_at {
                    cancellation.cancel();
                }
            }),
            Err(ProjectCaptureError::Cancelled)
        );
    }
}

#[test]
fn every_active_collection_and_byte_bound_rejects_one_over() {
    let source = || {
        CapturedSourceInput::new(
            "source:bounded",
            "limit::bounded",
            b"neu \"1.0\"\nmodule limit::bounded\n".to_vec(),
        )
    };
    let source_length = u64::try_from(source().bytes.len()).expect("fixture length must fit");
    let mut values = limits().values();
    values.source_bytes_per_unit = source_length - 1;
    assert_eq!(
        capture_project(request_with_limits(vec![source()], Vec::new(), values)),
        Err(ProjectCaptureError::LimitExceeded)
    );

    values = limits().values();
    values.total_source_bytes = source_length - 1;
    assert_eq!(
        capture_project(request_with_limits(vec![source()], Vec::new(), values)),
        Err(ProjectCaptureError::LimitExceeded)
    );

    values = limits().values();
    values.source_id_bytes = u64::try_from("source:bounded".len()).expect("length must fit") - 1;
    assert_eq!(
        capture_project(request_with_limits(vec![source()], Vec::new(), values)),
        Err(ProjectCaptureError::LimitExceeded)
    );

    values = limits().values();
    values.module_id_bytes = u64::try_from("limit::bounded".len()).expect("length must fit") - 1;
    assert_eq!(
        capture_project(request_with_limits(vec![source()], Vec::new(), values)),
        Err(ProjectCaptureError::LimitExceeded)
    );

    values = limits().values();
    values.source_id_bytes = 3;
    let short_id_source = CapturedSourceInput::new(
        "src",
        "limit::bounded",
        b"neu \"1.0\"\nmodule limit::bounded\n".to_vec(),
    );
    let project_key_over =
        request_with_limits(vec![short_id_source], Vec::new(), values).with_project_key("four");
    assert_eq!(
        capture_project(project_key_over),
        Err(ProjectCaptureError::LimitExceeded)
    );

    values = limits().values();
    values.vocabulary_units = 1;
    assert_eq!(
        capture_project(request_with_limits(
            vec![source()],
            vec![
                vocabulary("ExampleDomain", "1.0.0"),
                vocabulary("SecondDomain", "1.0.0"),
            ],
            values,
        )),
        Err(ProjectCaptureError::LimitExceeded)
    );

    values = limits().values();
    values.vocabulary_bytes_per_unit = 2;
    assert_eq!(
        capture_project(request_with_limits(
            vec![source()],
            vec![vocabulary("ExampleDomain", "1.0.0")],
            values,
        )),
        Err(ProjectCaptureError::LimitExceeded)
    );

    values = limits().values();
    values.total_vocabulary_bytes = 2;
    assert_eq!(
        capture_project(request_with_limits(
            vec![source()],
            vec![vocabulary("ExampleDomain", "1.0.0")],
            values,
        )),
        Err(ProjectCaptureError::LimitExceeded)
    );
}
