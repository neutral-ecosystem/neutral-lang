// SPDX-License-Identifier: Apache-2.0

//! Exact `/3` capture companions and atomic rejection contracts.

use super::*;
use crate::CapturedVocabularyInput;
use neutral_core::VocabularyContentDigest;
use neutral_vocabulary::VocabularyLock;

/// Builds a valid `/3` capture vocabulary whose content and lock cover its source use.
fn expression_vocabulary() -> CapturedVocabularyInput {
    let bytes = EXPRESSION_VOCABULARY_BYTES.to_vec();
    let lock = VocabularyLock::new(
        "ExampleDomain",
        "1.0.0",
        neutral_vocabulary::composition::ENCODING_VERSION,
        neutral_ir::composition::profile::VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&bytes),
        vec![neutral_ir::composition::profile::VOCABULARY_COMPOSITION_FEATURE.to_owned()],
    )
    .unwrap();
    CapturedVocabularyInput::new(bytes, lock)
}

const EXPRESSION_VOCABULARY_BYTES: &[u8] = br#"{
  "format": "neutral-vocabulary-bundle",
  "encoding_version": "1.0",
  "schema_version": "2.0",
  "identity": "ExampleDomain",
  "version": "1.0.0",
  "required_features": ["vocabulary-composition-v2"],
  "dependencies": [],
  "types": [{ "kind": "record", "name": "Item", "public": true, "fields": [] }]
}
"#;

/// Requests a single `/3` source with the exact required vocabulary lock.
fn capture_expression_with_controls(
    version: &str,
    profiles: expression_profile::SemanticProfiles,
    expression: ExpressionLimits,
    expansion: ExpansionLimits,
    bytes: &[u8],
    cancellation: CancellationToken,
) -> CapturedExpressionProjectRequest {
    request_with_inputs(
        vec![CapturedSourceInput::new(
            "source:main",
            "project::main",
            bytes.to_vec(),
        )],
        vec![expression_vocabulary()],
        version,
        profiles,
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(1_048_576, 64).unwrap(),
        )),
        (expression, expansion),
        cancellation,
    )
}

/// Extracts a capture failure without requiring the successful capture type to implement Debug.
fn expression_capture_failure(
    result: Result<CapturedExpressionProject, ExpressionCaptureFailure>,
) -> ExpressionCaptureFailure {
    match result {
        Err(failure) => failure,
        Ok(_) => panic!("unsupported expression request unexpectedly captured"),
    }
}

/// `/3` capture returns the exact accepted lock, profile and independent controls.
#[test]
fn captured_expression_companions_preserve_exact_inputs() {
    let source = b"neu \"1.0\"\nmodule project::main\nuse ExampleDomain as domain\n";
    let profiles = expression_profile::SemanticProfiles::try_frozen().unwrap();
    let mut expression = ExpressionLimits::hard();
    expression.coefficient_digits -= 1;
    let mut expansion = ExpansionLimits::hard();
    expansion.axes -= 1;
    let composition = CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(8192, 32).unwrap(),
    ));
    let captured = capture_expression_project(request_with_inputs(
        vec![CapturedSourceInput::new(
            "source:main",
            "project::main",
            source.to_vec(),
        )],
        vec![expression_vocabulary()],
        expression_profile::CAPTURE_REQUEST_VERSION,
        profiles,
        composition,
        (expression, expansion),
        CancellationToken::new(),
    ))
    .expect("valid /3 request should capture");
    assert_eq!(captured.sources()[0].bytes(), source);
    assert_eq!(captured.vocabularies()[0].identity(), "ExampleDomain");
    assert_eq!(
        captured.vocabularies()[0].bytes(),
        EXPRESSION_VOCABULARY_BYTES
    );
    assert!(captured.semantic_profiles().matches_frozen());
    assert_eq!(
        captured
            .required_features()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        expression_profile::REQUIRED_FEATURES
    );
    assert_eq!(captured.expression_limits(), expression);
    assert_eq!(captured.expansion_limits(), expansion);
    assert_eq!(captured.composition_limits(), composition);
    let values = captured.limits().values();
    assert_eq!(
        captured.identity_capture_limits(),
        [
            values.total_source_bytes,
            values.source_bytes_per_unit,
            values.source_units,
            values.source_id_bytes,
            values.module_id_bytes,
            values.vocabulary_units,
            values.vocabulary_bytes_per_unit,
            values.total_vocabulary_bytes,
            values.imports_per_module,
            values.import_edges,
            values.scc_units,
            values.declarations,
            values.diagnostics,
            values.output_bytes,
        ]
    );
}

/// Capture failures retain the selected stable diagnostic class through public conversions.
#[test]
fn expression_capture_failure_codes_preserve_wrapped_classification() {
    assert_eq!(
        ExpressionCaptureFailure::Unsupported.code(),
        crate::frontend::expression_codes::EXP_UNSUPPORTED
    );
    assert_eq!(
        ExpressionCaptureFailure::Limit.code(),
        crate::frontend::expression_codes::EXP_LIMIT
    );
    for error in [
        ProjectCaptureError::InvalidHeader,
        ProjectCaptureError::IntegrityMismatch,
        ProjectCaptureError::Cancelled,
        ProjectCaptureError::LimitExceeded,
    ] {
        let failure = ExpressionCaptureFailure::from(error);
        assert_eq!(failure.code(), error.code());
        assert!(matches!(
            failure,
            ExpressionCaptureFailure::Capture(CompositionCaptureFailure::Capture(actual))
                if actual == error
        ));
    }
}

/// Captured `/3` identity is byte-exact, capture-order invariant and changed by source bytes.
#[test]
fn captured_expression_identity_matches_exact_byte_oracle() {
    use neutral_ir::project_identity::IdentityLimits;

    let sources = [
        CapturedSourceInput::new(
            "source:zeta",
            "project::zeta",
            b"neu \"1.0\"\nmodule project::zeta\n".to_vec(),
        ),
        CapturedSourceInput::new(
            "source:alpha",
            "project::alpha",
            b"neu \"1.0\"\nmodule project::alpha\nuse ExampleDomain as domain\npublic domain::Item item = {}\n".to_vec(),
        ),
    ];
    let identity_limits = IdentityLimits {
        bytes: neutral_ir::project_identity::MAX_TRANSCRIPT_BYTES,
        nodes: neutral_ir::project_identity::MAX_TRANSCRIPT_NODES,
    };
    let capture = |ordered: Vec<CapturedSourceInput>| {
        capture_expression_project(request_with_inputs(
            ordered,
            vec![expression_vocabulary()],
            expression_profile::CAPTURE_REQUEST_VERSION,
            expression_profile::SemanticProfiles::try_frozen().unwrap(),
            CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
                StructuralLimits::new(1_048_576, 64).unwrap(),
            )),
            (ExpressionLimits::hard(), ExpansionLimits::hard()),
            CancellationToken::new(),
        ))
        .unwrap()
    };
    let first = capture(sources.to_vec());
    let permuted = capture(sources.into_iter().rev().collect());
    let changed = capture(
        [
            CapturedSourceInput::new(
                "source:alpha",
                "project::alpha",
                b"neu \"1.0\"\nmodule project::alpha\nuse ExampleDomain as domain\npublic domain::Item item = {}\n// changed\n".to_vec(),
            ),
            CapturedSourceInput::new(
                "source:zeta",
                "project::zeta",
                b"neu \"1.0\"\nmodule project::zeta\n".to_vec(),
            ),
        ]
        .to_vec(),
    );
    let cancellation = CancellationToken::new();
    let transcript = first
        .identity_transcript(identity_limits, &cancellation)
        .unwrap();
    assert_eq!(
        transcript,
        permuted
            .identity_transcript(identity_limits, &CancellationToken::new())
            .unwrap()
    );
    assert_ne!(
        transcript,
        changed
            .identity_transcript(identity_limits, &CancellationToken::new())
            .unwrap()
    );
    assert_captured_expression_identity_oracle(&first, identity_limits);
}

/// Reconstructs the captured transcript from public source and vocabulary facts independently.
fn assert_captured_expression_identity_oracle(
    captured: &CapturedExpressionProject,
    identity_limits: neutral_ir::project_identity::IdentityLimits,
) {
    use neutral_core::profile::V1_SOURCE_PROFILE;
    use neutral_ir::project_identity::{
        CapturedIdentityInput, CapturedIdentitySource, CapturedIdentityVocabulary,
        captured_expression_closure,
    };
    let sources: Vec<_> = captured
        .sources()
        .iter()
        .map(|source| CapturedIdentitySource {
            module: source.module_id(),
            source_id: source.source_id(),
            digest: source.digest(),
            byte_len: source.bytes().len() as u64,
        })
        .collect();
    let vocabularies: Vec<_> = captured
        .vocabularies()
        .iter()
        .map(|vocabulary| {
            let lock = vocabulary.lock();
            CapturedIdentityVocabulary {
                identity: lock.identity(),
                version: lock.version(),
                encoding_version: lock.encoding_version(),
                schema_version: lock.schema_version(),
                digest: lock.content_digest(),
                byte_len: vocabulary.bytes().len() as u64,
                required_features: lock.required_features(),
            }
        })
        .collect();
    let oracle = captured_expression_closure(
        &CapturedIdentityInput {
            profile: V1_SOURCE_PROFILE,
            sources: &sources,
            vocabularies: &vocabularies,
        },
        captured.semantic_profiles(),
        identity_limits,
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(
        captured
            .identity_transcript(identity_limits, &CancellationToken::new())
            .unwrap(),
        oracle
    );
}

/// Identity transcript exact byte/node bounds, allocation failures and cancellation recover.
#[test]
fn captured_expression_identity_rejects_bounds_and_recovers_after_faults() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    use neutral_ir::project_identity::{IdentityError, IdentityLimits};

    let captured = capture_expression_project(capture_expression_with_controls(
        expression_profile::CAPTURE_REQUEST_VERSION,
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
        b"neu \"1.0\"\nmodule project::main\nuse ExampleDomain as domain\n",
        CancellationToken::new(),
    ))
    .unwrap();
    let cancellation = CancellationToken::new();
    let (baseline, checkpoints) = observe(None, || {
        captured.identity_transcript(
            IdentityLimits {
                bytes: neutral_ir::project_identity::MAX_TRANSCRIPT_BYTES,
                nodes: neutral_ir::project_identity::MAX_TRANSCRIPT_NODES,
            },
            &cancellation,
        )
    });
    let baseline = baseline.unwrap();
    assert!(checkpoints > 0);
    let exact_bytes = baseline.bytes().len() as u64;
    let mut low = 1;
    let mut high = neutral_ir::project_identity::MAX_TRANSCRIPT_NODES;
    while low < high {
        let middle = low + (high - low) / 2;
        let result = captured.identity_transcript(
            IdentityLimits {
                bytes: exact_bytes,
                nodes: middle,
            },
            &CancellationToken::new(),
        );
        if result.is_ok() {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    let exact = IdentityLimits {
        bytes: exact_bytes,
        nodes: low,
    };
    assert_eq!(
        captured
            .identity_transcript(exact, &CancellationToken::new())
            .unwrap(),
        baseline
    );
    assert_eq!(
        captured
            .identity_transcript(
                IdentityLimits {
                    bytes: exact_bytes - 1,
                    nodes: low,
                },
                &CancellationToken::new(),
            )
            .unwrap_err(),
        IdentityError::Limit
    );
    assert_eq!(
        captured
            .identity_transcript(
                IdentityLimits {
                    bytes: exact_bytes,
                    nodes: low - 1,
                },
                &CancellationToken::new(),
            )
            .unwrap_err(),
        IdentityError::Limit
    );
    for index in 0..checkpoints {
        let (failure, observed) = observe(Some(index), || {
            captured.identity_transcript(exact, &CancellationToken::new())
        });
        assert_eq!(failure.unwrap_err(), IdentityError::Limit, "fault {index}");
        assert!(observed > index, "fault {index} should be reached");
        let token = CancellationToken::new();
        let (cancelled, _) = observe_cancellation(index, &token, || {
            captured.identity_transcript(exact, &token)
        });
        assert!(token.is_cancelled(), "cancel checkpoint {index}");
        assert_eq!(cancelled.unwrap_err(), IdentityError::Cancelled);
    }
    assert_eq!(
        captured
            .identity_transcript(exact, &CancellationToken::new())
            .unwrap(),
        baseline
    );
}

/// Exact `/3` version and semantic tuple rejection precede source parsing.
#[test]
fn expression_capture_rejects_unsupported_selection_before_parsing() {
    let malformed = b"not a language header";
    let request = capture_expression_with_controls(
        "neutral.capture/unsupported",
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
        malformed,
        CancellationToken::new(),
    );
    let failure = expression_capture_failure(capture_expression_project(request));
    assert_eq!(failure, ExpressionCaptureFailure::Unsupported);
    assert_eq!(
        failure.code(),
        crate::frontend::expression_codes::EXP_UNSUPPORTED
    );

    let mut profiles = expression_profile::SemanticProfiles::try_frozen().unwrap();
    profiles.expression.push_str("-unsupported");
    let request = capture_expression_with_controls(
        expression_profile::CAPTURE_REQUEST_VERSION,
        profiles,
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
        malformed,
        CancellationToken::new(),
    );
    assert_eq!(
        expression_capture_failure(capture_expression_project(request)),
        ExpressionCaptureFailure::Unsupported
    );
}

/// Each independent expression and expansion control is required to be positive.
#[test]
fn expression_capture_rejects_zero_semantic_controls() {
    let source = b"neu \"1.0\"\nmodule project::main\nuse ExampleDomain as domain\n";
    let baseline = capture_expression_project(capture_expression_with_controls(
        expression_profile::CAPTURE_REQUEST_VERSION,
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
        source,
        CancellationToken::new(),
    ));
    assert!(baseline.is_ok(), "positive controls should be accepted");
    let expression = ExpressionLimits::hard();
    let expansion = ExpansionLimits::hard();
    for expression in [
        ExpressionLimits {
            nodes: 0,
            ..expression
        },
        ExpressionLimits {
            syntax_depth: 0,
            ..expression
        },
        ExpressionLimits {
            evaluation_depth: 0,
            ..expression
        },
        ExpressionLimits {
            bindings: 0,
            ..expression
        },
        ExpressionLimits {
            dependency_edges: 0,
            ..expression
        },
        ExpressionLimits {
            work: 0,
            ..expression
        },
        ExpressionLimits {
            retained_bytes: 0,
            ..expression
        },
        ExpressionLimits {
            output_bytes: 0,
            ..expression
        },
        ExpressionLimits {
            coefficient_digits: 0,
            ..expression
        },
        ExpressionLimits {
            absolute_scale: 0,
            ..expression
        },
        ExpressionLimits {
            diagnostics: 0,
            ..expression
        },
    ] {
        let request = capture_expression_with_controls(
            expression_profile::CAPTURE_REQUEST_VERSION,
            expression_profile::SemanticProfiles::try_frozen().unwrap(),
            expression,
            expansion,
            source,
            CancellationToken::new(),
        );
        assert_eq!(
            expression_capture_failure(capture_expression_project(request)),
            ExpressionCaptureFailure::Limit
        );
    }
}

/// Each independent finite expansion control rejects zero without changing expression policy.
#[test]
fn expression_capture_rejects_zero_expansion_controls() {
    let source = b"neu \"1.0\"\nmodule project::main\nuse ExampleDomain as domain\n";
    let expression = ExpressionLimits::hard();
    let expansion = ExpansionLimits::hard();
    for expansion in [
        ExpansionLimits {
            definitions: 0,
            ..expansion
        },
        ExpansionLimits {
            applications: 0,
            ..expansion
        },
        ExpansionLimits {
            parameters: 0,
            ..expansion
        },
        ExpansionLimits {
            axes: 0,
            ..expansion
        },
        ExpansionLimits {
            axis_items: 0,
            ..expansion
        },
        ExpansionLimits {
            product_tuples: 0,
            ..expansion
        },
        ExpansionLimits {
            aggregate_tuples: 0,
            ..expansion
        },
        ExpansionLimits {
            expanded_bindings: 0,
            ..expansion
        },
        ExpansionLimits {
            expanded_nodes: 0,
            ..expansion
        },
        ExpansionLimits {
            depth: 0,
            ..expansion
        },
        ExpansionLimits {
            work: 0,
            ..expansion
        },
        ExpansionLimits {
            retained_bytes: 0,
            ..expansion
        },
        ExpansionLimits {
            output_bytes: 0,
            ..expansion
        },
    ] {
        let request = capture_expression_with_controls(
            expression_profile::CAPTURE_REQUEST_VERSION,
            expression_profile::SemanticProfiles::try_frozen().unwrap(),
            expression,
            expansion,
            source,
            CancellationToken::new(),
        );
        assert_eq!(
            expression_capture_failure(capture_expression_project(request)),
            ExpressionCaptureFailure::Limit
        );
    }
}

/// Capture allocation, cancellation and integrity failures never return partial companions.
#[test]
fn expression_capture_faults_preserve_atomic_rejection_and_recovery() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};

    let make_request = |cancellation| {
        capture_expression_with_controls(
            expression_profile::CAPTURE_REQUEST_VERSION,
            expression_profile::SemanticProfiles::try_frozen().unwrap(),
            ExpressionLimits::hard(),
            ExpansionLimits::hard(),
            b"neu \"1.0\"\nmodule project::main\nuse ExampleDomain as domain\n",
            cancellation,
        )
    };
    let request = make_request(CancellationToken::new());
    let (baseline, checkpoints) = observe(None, || capture_expression_project(request));
    let baseline = baseline.unwrap();
    assert!(checkpoints > 0);
    for index in 0..checkpoints {
        let request = make_request(CancellationToken::new());
        let (failure, observed) = observe(Some(index), || capture_expression_project(request));
        let Err(failure) = failure else {
            panic!("allocation fault {index} did not reject the complete capture");
        };
        assert!(observed > index, "allocation fault {index} must be reached");
        assert!(
            matches!(
                failure,
                ExpressionCaptureFailure::Limit
                    | ExpressionCaptureFailure::Capture(
                        CompositionCaptureFailure::Capture(ProjectCaptureError::LimitExceeded)
                            | CompositionCaptureFailure::Catalogue(
                                neutral_vocabulary::composition::CompositionError::Allocation
                                    | neutral_vocabulary::composition::CompositionError::Vocabulary(
                                        neutral_vocabulary::VocabularyError::JsonLimitExceeded
                                    )
                            )
                    )
            ),
            "allocation fault {index}: {failure:?}"
        );

        let token = CancellationToken::new();
        let request = make_request(token.clone());
        let (cancelled, observed) =
            observe_cancellation(index, &token, || capture_expression_project(request));
        assert!(
            observed > index,
            "cancellation checkpoint {index} must be reached"
        );
        assert!(token.is_cancelled(), "cancellation checkpoint {index}");
        assert!(cancelled.is_err(), "cancelled checkpoint {index} published");
    }
    let recovery = capture_expression_project(capture_expression_with_controls(
        expression_profile::CAPTURE_REQUEST_VERSION,
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
        b"neu \"1.0\"\nmodule project::main\nuse ExampleDomain as domain\n",
        CancellationToken::new(),
    ))
    .unwrap();
    assert_eq!(recovery.sources()[0].bytes(), baseline.sources()[0].bytes());
    assert_eq!(
        recovery.vocabularies()[0].bytes(),
        baseline.vocabularies()[0].bytes()
    );
    assert_eq!(recovery.required_features(), baseline.required_features());
    assert_eq!(recovery.semantic_profiles(), baseline.semantic_profiles());

    let bad_digest = CapturedSourceInput::new(
        "source:main",
        "project::main",
        b"neu \"1.0\"\nmodule project::main\nuse ExampleDomain as domain\n".to_vec(),
    )
    .requiring_digest(neutral_core::SourceContentDigest::from_bytes(b"different"));
    let failure = capture_expression_project(request_with_inputs(
        vec![bad_digest],
        vec![expression_vocabulary()],
        expression_profile::CAPTURE_REQUEST_VERSION,
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(1_048_576, 64).unwrap(),
        )),
        (ExpressionLimits::hard(), ExpansionLimits::hard()),
        CancellationToken::new(),
    ))
    .err()
    .expect("integrity mismatch must reject the complete capture");
    assert!(matches!(
        failure,
        ExpressionCaptureFailure::Capture(CompositionCaptureFailure::Capture(
            ProjectCaptureError::IntegrityMismatch
        ))
    ));
}
