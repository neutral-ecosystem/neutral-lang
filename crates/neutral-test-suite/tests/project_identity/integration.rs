// SPDX-License-Identifier: Apache-2.0

//! Validated-reader integration and real incremental/cache execution.

use super::*;

/// Binds frozen capture-lock facts to a complete independently validated reader.
pub(super) fn identities<'a>(
    reader: &'a neutral_reader::ValidatedProject,
    capture: &CapturedProject,
) -> neutral_reader::ProjectIdentities<'a> {
    let data = vectors();
    let sources = capture
        .sources()
        .iter()
        .map(|source| CapturedIdentitySource {
            module: source.module_id(),
            source_id: source.source_id(),
            digest: source.digest(),
            byte_len: source.bytes().len() as u64,
        })
        .collect::<Vec<_>>();
    let vocabularies = capture
        .vocabularies()
        .iter()
        .map(|v| {
            let lock = v.lock();
            CapturedIdentityVocabulary {
                identity: lock.identity(),
                version: lock.version(),
                encoding_version: lock.encoding_version(),
                schema_version: lock.schema_version(),
                digest: lock.content_digest(),
                byte_len: v.bytes().len() as u64,
                required_features: lock.required_features(),
            }
        })
        .collect::<Vec<_>>();
    reader
        .identities(
            &neutral_reader::ProjectIdentityContext {
                capture: CapturedIdentityInput {
                    profile: capture.profile().source_version(),
                    sources: &sources,
                    vocabularies: &vocabularies,
                },
                producer: text(&data["input"], "producer"),
                producer_version: text(&data["input"], "producer_version"),
                capture_limits: capture.identity_capture_limits(),
            },
            limits(),
            &CancellationToken::new(),
        )
        .unwrap()
}

/// Reader logical identities expose the exact same complete validated transcript.
#[test]
fn integration_project_identity_reader_complete_transcript() {
    let (capture, ir) = baseline();
    let token = CancellationToken::new();
    let reader =
        neutral_reader::ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &token).unwrap();
    assert_eq!(
        reader.logical_identity(limits(), &token).unwrap(),
        canonical_logical_project(&ir, limits(), &token).unwrap()
    );
    let ids = identities(&reader, &capture);
    assert_eq!(
        ids.captured(),
        &capture.identity_transcript(limits(), &token).unwrap()
    );
    assert_eq!(
        ids.logical(),
        &reader.logical_identity(limits(), &token).unwrap()
    );
    assert_eq!(ids.facts().logical, ids.logical().identity());
    assert_eq!(ids.facts().captured, ids.captured().identity());
    assert_eq!(ids.facts().project_limits, ir.limits);
    assert_eq!(
        ids.facts().capture_limits,
        capture.identity_capture_limits()
    );
    assert_eq!(ids.facts().producer, text(&vectors()["input"], "producer"));
    assert_eq!(
        ids.facts().producer_version,
        text(&vectors()["input"], "producer_version")
    );
    literal(ids.captured(), "captured", &vectors());
    literal(ids.logical(), "logical", &vectors());
    literal(ids.derivation(), "derivation", &vectors());
}

/// Reader artifact roots are checked without mutating any complete upstream layer.
#[test]
fn security_project_identity_reader_root_validation() {
    let token = CancellationToken::new();
    let capture = capture_project(crate::project_capture::request_fixture(include_str!(
        "../project_ir/complete.toml"
    )))
    .unwrap();
    let ir = compile_project(&capture, &token).unwrap();
    let reader =
        neutral_reader::ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &token).unwrap();
    let ids = identities(&reader, &capture);
    let roots = ir
        .public_interface
        .exports()
        .iter()
        .map(|e| e.identity().clone())
        .collect::<Vec<_>>();
    let reverse = roots.iter().rev().cloned().collect::<Vec<_>>();
    let artifact = |selection: &[ModuleSymbolIdentity]| {
        ids.artifact(
            &ArtifactIdentityInput {
                kind: ArtifactKind::View,
                format: PROJECT_VIEW_SCHEMA,
                roots: selection,
                options: &[],
            },
            limits(),
            &token,
        )
    };
    assert_eq!(artifact(&roots).unwrap(), artifact(&reverse).unwrap());
    assert_ne!(artifact(&roots).unwrap(), artifact(&[]).unwrap());
    let before = (
        ids.captured().clone(),
        ids.logical().clone(),
        ids.derivation().clone(),
    );
    for selection in [&[][..], &roots[..], &roots[..1], &reverse[..]] {
        artifact(selection).unwrap();
        let after = identities(&reader, &capture);
        assert_eq!(
            before,
            (
                after.captured().clone(),
                after.logical().clone(),
                after.derivation().clone()
            )
        );
    }
    let private = ir
        .declarations
        .iter()
        .find(|decl| !decl.public)
        .unwrap()
        .identity
        .clone();
    assert!(matches!(
        artifact(&[private]),
        Err(neutral_reader::ProjectIdentityReadError::View(
            neutral_reader::ProjectReadError::View
        ))
    ));
    assert!(artifact(&[roots[0].clone(), roots[0].clone()]).is_err());
    let unknown = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "unknown"),
        "missing",
    );
    assert!(artifact(&[unknown]).is_err());
    assert!(
        ids.artifact(
            &ArtifactIdentityInput {
                kind: ArtifactKind::Project,
                format: PROJECT_VIEW_SCHEMA,
                roots: &roots[..1],
                options: &[]
            },
            limits(),
            &token
        )
        .is_err()
    );
    assert!(artifact(&roots).is_ok());
    assert!(
        ids.artifact(
            &ArtifactIdentityInput {
                kind: ArtifactKind::View,
                format: PROJECT_VIEW_SCHEMA,
                roots: &roots,
                options: &[]
            },
            IdentityLimits { bytes: 1, nodes: 1 },
            &token,
        )
        .is_err()
    );
}

/// Capture companions are checked before identity exposure, and cancellation takes priority.
#[test]
fn security_project_identity_reader_capture_facts_and_cancellation() {
    let (capture, ir) = baseline();
    let token = CancellationToken::new();
    let reader =
        neutral_reader::ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &token).unwrap();
    let bad = [CapturedIdentitySource {
        module: &ir.sources[0].module,
        source_id: "stale-source-id",
        digest: ir.sources[0].digest,
        byte_len: ir.sources[0].byte_len,
    }];
    let context = neutral_reader::ProjectIdentityContext {
        capture: CapturedIdentityInput {
            profile: V1_SOURCE_PROFILE,
            sources: &bad,
            vocabularies: &[],
        },
        producer: "test-producer",
        producer_version: "reviewed-revision",
        capture_limits: capture.identity_capture_limits(),
    };
    assert!(matches!(
        reader.identities(&context, limits(), &token),
        Err(neutral_reader::ProjectIdentityReadError::CaptureMismatch)
    ));
    for field in 0..4 {
        let source = &ir.sources[0];
        let facts = [CapturedIdentitySource {
            module: if field == 0 {
                "stale-module"
            } else {
                &source.module
            },
            source_id: if field == 1 {
                "stale-source-id"
            } else {
                &source.source_id
            },
            digest: if field == 2 {
                SourceContentDigest::from_bytes(b"stale")
            } else {
                source.digest
            },
            byte_len: source.byte_len + u64::from(field == 3),
        }];
        let changed = neutral_reader::ProjectIdentityContext {
            capture: CapturedIdentityInput {
                profile: V1_SOURCE_PROFILE,
                sources: &facts,
                vocabularies: &[],
            },
            ..context
        };
        assert!(matches!(
            reader.identities(&changed, limits(), &token),
            Err(neutral_reader::ProjectIdentityReadError::CaptureMismatch)
        ));
    }
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        reader.logical_identity(limits(), &cancelled),
        Err(IdentityError::Cancelled)
    );
    assert!(matches!(
        reader.identities(&context, limits(), &cancelled),
        Err(neutral_reader::ProjectIdentityReadError::Identity(
            IdentityError::Cancelled
        ))
    ));
}

/// Reader-only encoded probe publishes the same complete digest for every selection.
#[test]
fn integration_project_identity_standalone_probe_root_invariance() {
    let token = CancellationToken::new();
    let capture = capture_project(crate::project_capture::request_fixture(include_str!(
        "../project_ir/complete.toml"
    )))
    .unwrap();
    let ir = compile_project(&capture, &token).unwrap();
    let reader =
        neutral_reader::ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &token).unwrap();
    let before = reader.logical_identity(limits(), &token).unwrap();
    let bytes = neutral_encoding::project::encode_project(&reader, &token).unwrap();
    let first = &ir.public_interface.exports()[0];
    let selected = [format!(
        "{}::{}",
        first.identity().module().module_name(),
        first.identity().declaration_name()
    )];
    for roots in [None, Some(&[][..]), Some(&selected[..])] {
        let summary = neutral_probe::project::inspect_project_encoded(
            &bytes,
            neutral_encoding::DecodeLimits::hard(),
            ir.limits,
            roots,
            &token,
        )
        .unwrap();
        assert_eq!(summary.logical_identity, before.identity());
        assert_eq!(summary.identity_profile, IDENTITY_PROFILE);
        let json: Value = serde_json::from_str(
            &neutral_probe::project::render_project_summary_json(&summary),
        )
        .unwrap();
        assert_eq!(json["logical_identity"], before.identity().to_string());
        assert_eq!(json["identity_profile"], IDENTITY_PROFILE);
        assert_eq!(reader.logical_identity(limits(), &token).unwrap(), before);
    }
}

/// Rebuilds exact capture inputs with changed bytes, IDs, order, or controls.
fn changed_capture(
    capture: &CapturedProject,
    change: impl Fn(&str, &str) -> (String, Vec<u8>),
    controls: ProjectCaptureLimits,
) -> CapturedProject {
    let sources = capture
        .sources()
        .iter()
        .rev()
        .map(|source| {
            let (id, bytes) = change(
                source.source_id(),
                std::str::from_utf8(source.bytes()).unwrap(),
            );
            CapturedSourceInput::new(id, source.module_id(), bytes)
        })
        .collect();
    capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        capture.profile(),
        sources,
        Vec::new(),
        ProjectCaptureControls::new(controls, CancellationToken::new()),
    ))
    .unwrap()
}

/// Cached execution is observed, and changed/private/stale source facts match a fresh compile.
#[test]
fn property_project_identity_actual_incremental_execution_matches_clean() {
    let token = CancellationToken::new();
    let original = capture_project(crate::project_capture::request_fixture(include_str!(
        "../project_ir/complete.toml"
    )))
    .unwrap();
    let mut cache =
        neutral_compiler::ProjectCompilationCache::new(neutral_compiler::ProjectCacheLimits {
            source_units: original.limits().values().source_units,
            source_bytes: original.limits().values().total_source_bytes,
        })
        .unwrap();
    let count = original.sources().len() as u64;
    let (cold, stats) = cache.compile(&original, &token).unwrap();
    assert_eq!(stats.parsed_units, count);
    assert_eq!(stats.reused_units, 0);
    let (warm, stats) = cache.compile(&original, &token).unwrap();
    assert_eq!(stats.parsed_units, 0);
    assert_eq!(stats.reused_units, count);
    assert_eq!(cold, warm);
    let changed = changed_capture(
        &original,
        |id, source| {
            (
                id.to_owned(),
                source
                    .replace("materialized private value", "changed private value")
                    .into_bytes(),
            )
        },
        original.limits(),
    );
    let (incremental, stats) = cache.compile(&changed, &token).unwrap();
    assert_eq!(stats.parsed_units, 1);
    assert_eq!(stats.reused_units, count - 1);
    assert_eq!(stats.rejected_entries, 1);
    let clean = compile_project(&changed, &token).unwrap();
    assert_eq!(incremental, clean);
    assert_ne!(
        canonical_logical_project(&cold, limits(), &token).unwrap(),
        canonical_logical_project(&incremental, limits(), &token).unwrap()
    );
    assert!(
        incremental
            .sources
            .iter()
            .zip(&cold.sources)
            .any(|(a, b)| a.digest != b.digest)
    );
    let changed_ids = changed_capture(
        &changed,
        |id, source| (format!("new:{id}"), source.as_bytes().to_vec()),
        changed.limits(),
    );
    let (incremental, stats) = cache.compile(&changed_ids, &token).unwrap();
    assert_eq!(stats.reused_units, count);
    assert_eq!(stats.parsed_units, 0);
    let clean = compile_project(&changed_ids, &token).unwrap();
    assert_eq!(incremental, clean);
    let read = neutral_reader::ValidatedProject::from_ir(
        Arc::clone(&incremental),
        incremental.limits,
        &token,
    )
    .unwrap();
    let clean_read =
        neutral_reader::ValidatedProject::from_ir(Arc::clone(&clean), clean.limits, &token)
            .unwrap();
    assert_eq!(
        read.logical_identity(limits(), &token).unwrap(),
        clean_read.logical_identity(limits(), &token).unwrap()
    );
    assert_eq!(
        identities(&read, &changed_ids).derivation(),
        identities(&clean_read, &changed_ids).derivation()
    );
    assert_eq!(
        neutral_encoding::project::encode_project(&read, &token).unwrap(),
        neutral_encoding::project::encode_project(&clean_read, &token).unwrap()
    );
}

/// Failed semantic or resource validation cannot publish cache state or bypass new controls.
#[test]
fn security_project_identity_cached_failures_match_clean_and_recover() {
    let token = CancellationToken::new();
    let changed_ids = capture_project(crate::project_capture::request_fixture(include_str!(
        "../project_ir/complete.toml"
    )))
    .unwrap();
    let count = changed_ids.sources().len() as u64;
    let mut cache =
        neutral_compiler::ProjectCompilationCache::new(neutral_compiler::ProjectCacheLimits {
            source_units: changed_ids.limits().values().source_units,
            source_bytes: changed_ids.limits().values().total_source_bytes,
        })
        .unwrap();
    cache.compile(&changed_ids, &token).unwrap();
    let bad = changed_capture(
        &changed_ids,
        |id, source| {
            (
                id.to_owned(),
                source
                    .replace("ref(shared::api)", "ref(shared::missing)")
                    .into_bytes(),
            )
        },
        changed_ids.limits(),
    );
    assert_eq!(
        format!("{:?}", cache.compile(&bad, &token).unwrap_err()),
        format!("{:?}", compile_project(&bad, &token).unwrap_err())
    );
    let (_, recovered) = cache.compile(&changed_ids, &token).unwrap();
    assert_eq!(recovered.reused_units, count);
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(cache.compile(&changed_ids, &cancelled).is_err());
    let mut controls = changed_ids.limits().values();
    controls.declarations = 1;
    let restricted = changed_capture(
        &changed_ids,
        |id, source| (id.to_owned(), source.as_bytes().to_vec()),
        ProjectCaptureLimits::new(controls),
    );
    assert_eq!(
        format!("{:?}", cache.compile(&restricted, &token).unwrap_err()),
        format!("{:?}", compile_project(&restricted, &token).unwrap_err())
    );
    assert_eq!(
        cache.compile(&changed_ids, &token).unwrap().1.reused_units,
        count
    );
}

/// Independent caches under shuffled concurrent schedules reproduce complete clean artifacts.
#[test]
fn property_project_identity_incremental_concurrent_schedules() {
    let original = capture_project(crate::project_capture::request_fixture(include_str!(
        "../project_ir/complete.toml"
    )))
    .unwrap();
    let changed = changed_capture(
        &original,
        |id, source| {
            (
                format!("changed:{id}"),
                source
                    .replace("materialized private value", "other private value")
                    .into_bytes(),
            )
        },
        original.limits(),
    );
    std::thread::scope(|scope| {
        for reverse in [false, true, false, true] {
            let original = &original;
            let changed = &changed;
            scope.spawn(move || {
                let token = CancellationToken::new();
                let mut cache = neutral_compiler::ProjectCompilationCache::new(
                    neutral_compiler::ProjectCacheLimits {
                        source_units: original.limits().values().source_units,
                        source_bytes: original.limits().values().total_source_bytes,
                    },
                )
                .unwrap();
                let schedule = if reverse {
                    [changed, original, original, changed]
                } else {
                    [original, changed, changed, original]
                };
                for capture in schedule {
                    let (cached, _) = cache.compile(capture, &token).unwrap();
                    let clean = compile_project(capture, &token).unwrap();
                    assert_eq!(cached, clean);
                    let cached_reader = neutral_reader::ValidatedProject::from_ir(
                        Arc::clone(&cached),
                        cached.limits,
                        &token,
                    )
                    .unwrap();
                    let clean_reader = neutral_reader::ValidatedProject::from_ir(
                        Arc::clone(&clean),
                        clean.limits,
                        &token,
                    )
                    .unwrap();
                    assert_eq!(
                        identities(&cached_reader, capture).logical(),
                        identities(&clean_reader, capture).logical()
                    );
                    assert_eq!(
                        identities(&cached_reader, capture).derivation(),
                        identities(&clean_reader, capture).derivation()
                    );
                    assert_eq!(
                        neutral_encoding::project::encode_project(&cached_reader, &token).unwrap(),
                        neutral_encoding::project::encode_project(&clean_reader, &token).unwrap()
                    );
                }
            });
        }
    });
}

/// Rebuilds exact vocabulary locks while preserving source bytes for cache isolation tests.
fn changed_vocabulary(capture: &CapturedProject, private: bool) -> CapturedProject {
    let vocabularies = capture
        .vocabularies()
        .iter()
        .map(|v| {
            let mut bundle: Value = serde_json::from_slice(v.bytes()).unwrap();
            if private {
                bundle["types"][0]["public"] = serde_json::json!(false);
            }
            let bytes = serde_json::to_vec_pretty(&bundle).unwrap();
            let lock = v.lock();
            CapturedVocabularyInput::new(
                bytes.clone(),
                neutral_vocabulary::VocabularyLock::new(
                    lock.identity(),
                    lock.version(),
                    lock.encoding_version(),
                    lock.schema_version(),
                    VocabularyContentDigest::from_bytes(&bytes),
                    lock.required_features().to_vec(),
                )
                .unwrap(),
            )
        })
        .collect();
    let sources = capture
        .sources()
        .iter()
        .map(|s| CapturedSourceInput::new(s.source_id(), s.module_id(), s.bytes().to_vec()))
        .collect();
    capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        capture.profile(),
        sources,
        vocabularies,
        ProjectCaptureControls::new(capture.limits(), CancellationToken::new()),
    ))
    .unwrap()
}

/// Syntax hits cannot reuse old vocabulary visibility, captured facts, or semantic conclusions.
#[test]
fn security_project_identity_cached_vocabulary_is_revalidated() {
    let original = capture_project(crate::project_capture::request_fixture(include_str!(
        "../public_semantics/fixtures/positive/vocabulary-multiple-alias.toml"
    )))
    .unwrap();
    let token = CancellationToken::new();
    let mut cache =
        neutral_compiler::ProjectCompilationCache::new(neutral_compiler::ProjectCacheLimits {
            source_units: original.limits().values().source_units,
            source_bytes: original.limits().values().total_source_bytes,
        })
        .unwrap();
    let (cold, _) = cache.compile(&original, &token).unwrap();
    let changed = changed_vocabulary(&original, false);
    let (warm, stats) = cache.compile(&changed, &token).unwrap();
    assert_eq!(stats.reused_units, original.sources().len() as u64);
    assert_eq!(stats.parsed_units, 0);
    assert_eq!(warm, compile_project(&changed, &token).unwrap());
    assert_eq!(
        canonical_logical_project(&cold, limits(), &token).unwrap(),
        canonical_logical_project(&warm, limits(), &token).unwrap()
    );
    assert_ne!(
        original.identity_transcript(limits(), &token).unwrap(),
        changed.identity_transcript(limits(), &token).unwrap()
    );
    let private = changed_vocabulary(&original, true);
    assert_eq!(
        format!("{:?}", cache.compile(&private, &token).unwrap_err()),
        format!("{:?}", compile_project(&private, &token).unwrap_err())
    );
    assert_eq!(
        cache.compile(&original, &token).unwrap().1.reused_units,
        original.sources().len() as u64
    );
}
