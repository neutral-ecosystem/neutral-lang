// SPDX-License-Identifier: Apache-2.0

//! Complete identity partitions and pinned literal cross-package vector evidence.

use neutral_compiler::{
    CAPTURE_REQUEST_VERSION, CapturedProject, CapturedProjectRequest, CapturedSourceInput,
    CapturedVocabularyInput, ProjectCaptureControls, ProjectCaptureLimits, capture_project,
    compile_project,
};
use neutral_core::{
    CancellationToken, SemanticDigest, SourceContentDigest, VocabularyContentDigest,
    profile::{LanguageProfile, V1_SOURCE_PROFILE},
};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    project::{PROJECT_VIEW_SCHEMA, ProjectIr, ViewRequest},
    project_identity::*,
    project_interface::{ProjectPublicSignature, ProjectPublicType},
};
use serde_json::Value;
use std::sync::Arc;

/// Reviewed literal vectors, independent of the runtime implementation.
const VECTORS: &str = include_str!("vectors.json");

mod integration;
mod reference;

/// Independent generous test bounds; exact boundaries are tested separately.
fn limits() -> IdentityLimits {
    IdentityLimits {
        bytes: MAX_TRANSCRIPT_BYTES,
        nodes: MAX_TRANSCRIPT_NODES,
    }
}

/// Decodes the reviewed JSON vector input and expected transcript/digest tables.
fn vectors() -> Value {
    serde_json::from_str(VECTORS).unwrap()
}

/// Returns one required literal text input without coercion or defaults.
fn text<'a>(input: &'a Value, key: &str) -> &'a str {
    input[key].as_str().unwrap()
}

/// Returns the same accepted capture controls as the independent minimal vector.
fn capture_limits() -> ProjectCaptureLimits {
    crate::project_capture::fixture_limits(&crate::project_capture::parse_fixture(include_str!(
        "../project_ir/complete.toml"
    )))
}

/// Captures one complete minimal source, allowing controlled evidence-only variations.
fn captured(source_id: &str, source: &str, controls: ProjectCaptureLimits) -> CapturedProject {
    let data = vectors();
    let input = &data["input"];
    capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![CapturedSourceInput::new(
            source_id,
            text(input, "module"),
            source.as_bytes().to_vec(),
        )],
        Vec::new(),
        ProjectCaptureControls::new(controls, CancellationToken::new()),
    ))
    .unwrap()
}

/// Returns baseline accepted capture and independently reader-valid complete IR.
fn baseline() -> (CapturedProject, Arc<ProjectIr>) {
    let data = vectors();
    let input = &data["input"];
    let capture = captured(
        text(input, "source_id"),
        text(input, "source_utf8"),
        capture_limits(),
    );
    let ir = compile_project(&capture, &CancellationToken::new()).unwrap();
    neutral_reader::ValidatedProject::from_ir(
        Arc::clone(&ir),
        ir.limits,
        &CancellationToken::new(),
    )
    .unwrap();
    (capture, ir)
}

/// Constructs explicit derivation context from the pinned input, never environment metadata.
fn context<'a>(
    data: &'a Value,
    captured: CapturedClosureIdentity,
    capture: &CapturedProject,
    ir: &ProjectIr,
) -> DerivationContext<'a> {
    DerivationContext {
        captured,
        producer: text(&data["input"], "producer"),
        producer_version: text(&data["input"], "producer_version"),
        capture_limits: capture.identity_capture_limits(),
        project_limits: ir.limits,
    }
}

/// Converts exact transcript bytes to lowercase hexadecimal for literal comparison.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut result = String::new();
    for byte in bytes {
        write!(result, "{byte:02x}").unwrap();
    }
    result
}

/// Requires both exact byte equality and the pinned independent SHA-256 result.
fn literal<I: Copy + std::fmt::Display>(actual: &IdentityTranscript<I>, layer: &str, data: &Value) {
    let expected = data["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|vector| vector["layer"] == layer)
        .unwrap();
    assert_eq!(
        actual.bytes().len() as u64,
        expected["transcript_bytes"].as_u64().unwrap(),
        "{layer}"
    );
    assert_eq!(
        hex(actual.bytes()),
        text(expected, "transcript_hex"),
        "{layer}"
    );
    assert_eq!(
        actual.identity().to_string(),
        text(expected, "sha256"),
        "{layer}"
    );
    assert_eq!(
        SemanticDigest::from_transcript(actual.bytes()).to_string(),
        text(expected, "sha256")
    );
}

/// Every identity layer matches literal complete NHT framing and an external SHA-256 oracle.
#[test]
fn conformance_project_identity_literal_vectors() {
    let data = vectors();
    assert_eq!(text(&data, "identity_profile"), IDENTITY_PROFILE);
    let (capture, ir) = baseline();
    let token = CancellationToken::new();
    let captured = capture.identity_transcript(limits(), &token).unwrap();
    assert_eq!(
        hex(&capture.sources()[0].digest().as_bytes()),
        text(&data["input"], "source_sha256")
    );
    let logical = canonical_logical_project(&ir, limits(), &token).unwrap();
    let context = context(&data, captured.identity(), &capture, &ir);
    let derived = derivation_identity(logical.identity(), &context, limits(), &token).unwrap();
    literal(&captured, "captured", &data);
    literal(&logical, "logical", &data);
    literal(&derived, "derivation", &data);
    for (layer, kind, format, roots) in [
        (
            "artifact-project",
            ArtifactKind::Project,
            text(&data["input"], "artifact_format"),
            Vec::new(),
        ),
        (
            "artifact-view",
            ArtifactKind::View,
            PROJECT_VIEW_SCHEMA,
            vec![ir.declarations[0].identity.clone()],
        ),
        (
            "artifact-format",
            ArtifactKind::Project,
            text(&data["input"], "alternate_format"),
            Vec::new(),
        ),
    ] {
        let artifact = artifact_identity(
            derived.identity(),
            &ArtifactIdentityInput {
                kind,
                format,
                roots: &roots,
                options: &[],
            },
            limits(),
            &token,
        )
        .unwrap();
        literal(&artifact, layer, &data);
    }
}

/// Equivalent exact-number and string spellings share meaning; type, list order, and defaults do not.
#[test]
fn property_project_identity_normalizes_values_not_types_or_order() {
    let token = CancellationToken::new();
    let make = |declaration: &str| {
        let source = format!("neu \"{V1_SOURCE_PROFILE}\"\nmodule example\n{declaration}\n");
        let capture = captured("source:example", &source, capture_limits());
        let ir = compile_project(&capture, &token).unwrap();
        neutral_reader::ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &token).unwrap();
        reference::compare(&capture, &ir);
        canonical_logical_project(&ir, limits(), &token)
            .unwrap()
            .identity()
    };
    assert_eq!(
        make("public num value = 1e2"),
        make("public num value = 100")
    );
    assert_eq!(
        make("public string value = \"hello\""),
        make("public string value = \"h\\u{65}llo\"")
    );
    let string = make("public string value = \"../x\"");
    let url = make("public url value = \"../x\"");
    let path = make("public path value = \"../x\"");
    assert_ne!(string, url);
    assert_ne!(string, path);
    assert_ne!(url, path);
    assert_ne!(
        make("public List<num> value = [1, 2]"),
        make("public List<num> value = [2, 1]")
    );
    assert_ne!(
        make("record Config { string label = \"first\", }"),
        make("record Config { string label = \"second\", }")
    );
}

/// Exact vocabulary bytes affect capture, while vocabulary aliases and bundle whitespace do not affect meaning.
#[test]
fn property_project_identity_vocabulary_evidence_and_aliases() {
    let token = CancellationToken::new();
    let capture = capture_project(crate::project_capture::request_fixture(include_str!(
        "../public_semantics/fixtures/positive/vocabulary-multiple-alias.toml"
    )))
    .unwrap();
    let original = capture.identity_transcript(limits(), &token).unwrap();
    let ir = compile_project(&capture, &token).unwrap();
    let logical = canonical_logical_project(&ir, limits(), &token).unwrap();
    for change_evidence in [false, true] {
        let vocabularies = capture
            .vocabularies()
            .iter()
            .rev()
            .map(|vocabulary| {
                let mut bytes = vocabulary.bytes().to_vec();
                if change_evidence {
                    bytes.push(b'\n');
                }
                let lock = vocabulary.lock();
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
            .map(|source| {
                let mut text = String::from_utf8(source.bytes().to_vec()).unwrap();
                if change_evidence {
                    text = text
                        .replace("as alpha", "as local")
                        .replace("alpha::", "local::");
                }
                CapturedSourceInput::new(source.source_id(), source.module_id(), text.into_bytes())
            })
            .collect();
        let other = capture_project(CapturedProjectRequest::new(
            CAPTURE_REQUEST_VERSION,
            capture.profile(),
            sources,
            vocabularies,
            ProjectCaptureControls::new(capture.limits(), token.clone()),
        ))
        .unwrap();
        let other_ir = compile_project(&other, &token).unwrap();
        reference::compare(&other, &other_ir);
        assert!(ir.logical_eq(&other_ir));
        assert_eq!(
            logical,
            canonical_logical_project(&other_ir, limits(), &token).unwrap()
        );
        assert_eq!(
            original == other.identity_transcript(limits(), &token).unwrap(),
            !change_evidence,
        );
    }
}

/// Producer revision and acceptance controls partition derivation, not complete logical meaning.
#[test]
fn property_project_identity_derivation_context_partition() {
    let (capture, ir) = baseline();
    let token = CancellationToken::new();
    let data = vectors();
    let captured = capture
        .identity_transcript(limits(), &token)
        .unwrap()
        .identity();
    let logical = canonical_logical_project(&ir, limits(), &token)
        .unwrap()
        .identity();
    let original = derivation_identity(
        logical,
        &context(&data, captured, &capture, &ir),
        limits(),
        &token,
    )
    .unwrap();
    for field in 0..3 {
        let mut changed = context(&data, captured, &capture, &ir);
        match field {
            0 => changed.producer_version = "different-reviewed-revision",
            1 => changed.capture_limits[0] += 1,
            _ => changed.project_limits.nodes += 1,
        }
        assert_ne!(
            original,
            derivation_identity(logical, &changed, limits(), &token).unwrap()
        );
    }
}

/// Trivia and source IDs change capture/derivation identities, never complete logical meaning.
#[test]
fn property_project_identity_excludes_source_evidence_and_controls() {
    let (capture, ir) = baseline();
    let token = CancellationToken::new();
    let original = capture.identity_transcript(limits(), &token).unwrap();
    let logical = canonical_logical_project(&ir, limits(), &token).unwrap();
    let data = vectors();
    for (source_id, source) in [
        (
            "renamed-source",
            text(&data["input"], "source_utf8").to_owned(),
        ),
        (
            text(&data["input"], "source_id"),
            format!(
                "{}// harmless trivia\n",
                text(&data["input"], "source_utf8")
            ),
        ),
    ] {
        let other = captured(source_id, &source, capture_limits());
        let other_ir = compile_project(&other, &token).unwrap();
        assert!(ir.logical_eq(&other_ir));
        assert_eq!(
            logical,
            canonical_logical_project(&other_ir, limits(), &token).unwrap()
        );
        let other_capture = other.identity_transcript(limits(), &token).unwrap();
        assert_ne!(original.identity(), other_capture.identity());
        assert_ne!(
            derivation_identity(
                logical.identity(),
                &context(&data, original.identity(), &capture, &ir),
                limits(),
                &token
            )
            .unwrap()
            .identity(),
            derivation_identity(
                logical.identity(),
                &context(&data, other_capture.identity(), &other, &other_ir),
                limits(),
                &token
            )
            .unwrap()
            .identity()
        );
    }
    let mut values = capture_limits().values();
    values.output_bytes += 1;
    let other = captured(
        text(&data["input"], "source_id"),
        text(&data["input"], "source_utf8"),
        ProjectCaptureLimits::new(values),
    );
    let other_ir = compile_project(&other, &token).unwrap();
    assert_eq!(
        original,
        other.identity_transcript(limits(), &token).unwrap()
    );
    assert_eq!(
        logical,
        canonical_logical_project(&other_ir, limits(), &token).unwrap()
    );
    assert_ne!(
        derivation_identity(
            logical.identity(),
            &context(&data, original.identity(), &capture, &ir),
            limits(),
            &token
        )
        .unwrap(),
        derivation_identity(
            logical.identity(),
            &context(&data, original.identity(), &other, &other_ir),
            limits(),
            &token
        )
        .unwrap()
    );
}

/// Capture permutation, aliases, and root selection do not change complete logical identity.
#[test]
fn property_project_identity_alias_order_and_root_invariance() {
    let token = CancellationToken::new();
    let original =
        crate::project_capture::request_fixture(include_str!("../project_ir/complete.toml"));
    let capture = capture_project(original).unwrap();
    let ir = compile_project(&capture, &token).unwrap();
    let identity = canonical_logical_project(&ir, limits(), &token).unwrap();
    let sources = capture
        .sources()
        .iter()
        .rev()
        .map(|source| {
            CapturedSourceInput::new(
                source.source_id(),
                source.module_id(),
                source.bytes().to_vec(),
            )
        })
        .collect();
    let shuffled = capture_project(
        CapturedProjectRequest::new(
            CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            sources,
            Vec::new(),
            ProjectCaptureControls::new(capture.limits(), token.clone()),
        )
        .with_project_key("host-correlation-only"),
    )
    .unwrap();
    assert_eq!(
        capture.identity_transcript(limits(), &token).unwrap(),
        shuffled.identity_transcript(limits(), &token).unwrap()
    );
    let renamed_sources = capture
        .sources()
        .iter()
        .map(|source| {
            CapturedSourceInput::new(
                source.source_id(),
                source.module_id(),
                String::from_utf8(source.bytes().to_vec())
                    .unwrap()
                    .replace("as shared", "as local")
                    .replace("shared::", "local::")
                    .into_bytes(),
            )
        })
        .collect();
    let renamed = capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        renamed_sources,
        Vec::new(),
        ProjectCaptureControls::new(capture.limits(), token.clone()),
    ))
    .unwrap();
    assert_eq!(
        identity,
        canonical_logical_project(
            &compile_project(&renamed, &token).unwrap(),
            limits(),
            &token
        )
        .unwrap()
    );
    let reader =
        neutral_reader::ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &token).unwrap();
    for roots in [
        Vec::new(),
        vec![ir.public_interface.exports()[0].identity().clone()],
    ] {
        reader
            .derive_view(
                &ViewRequest {
                    schema: PROJECT_VIEW_SCHEMA.to_owned(),
                    roots,
                },
                &token,
            )
            .unwrap();
        assert_eq!(
            identity,
            canonical_logical_project(reader.complete_ir(), limits(), &token).unwrap()
        );
    }
}

/// Private content and disconnected modules remain identity-bearing complete project members.
#[test]
fn property_project_identity_complete_meaning_is_not_public_fingerprint() {
    let token = CancellationToken::new();
    let request =
        crate::project_capture::request_fixture(include_str!("../project_ir/complete.toml"));
    let capture = capture_project(request).unwrap();
    let ir = compile_project(&capture, &token).unwrap();
    let original = canonical_logical_project(&ir, limits(), &token).unwrap();
    let mut changed = ir.as_ref().clone();
    let orphan = changed
        .declarations
        .iter_mut()
        .find(|decl| decl.identity.module().module_name().ends_with("orphan"))
        .unwrap();
    orphan.value = Some(neutral_ir::project::ProjectValue::String(
        "different private content".to_owned(),
    ));
    assert_eq!(
        ir.public_interface.fingerprint(),
        changed.public_interface.fingerprint()
    );
    assert_ne!(
        original.identity(),
        canonical_logical_project(&changed, limits(), &token)
            .unwrap()
            .identity()
    );
}

/// Artifact selection, options, kind, and format are separate from both logical and derivation layers.
#[test]
fn property_project_identity_artifact_partition_and_root_normalization() {
    let (capture, ir) = baseline();
    let token = CancellationToken::new();
    let data = vectors();
    let captured = capture.identity_transcript(limits(), &token).unwrap();
    let logical = canonical_logical_project(&ir, limits(), &token).unwrap();
    let derived = derivation_identity(
        logical.identity(),
        &context(&data, captured.identity(), &capture, &ir),
        limits(),
        &token,
    )
    .unwrap();
    let first = ir.declarations[0].identity.clone();
    let second = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(capture.profile().source_version(), "other"),
        "root",
    );
    let roots = [first.clone(), second.clone()];
    let reversed = [second, first];
    let make = |roots, options| {
        artifact_identity(
            derived.identity(),
            &ArtifactIdentityInput {
                kind: ArtifactKind::View,
                format: PROJECT_VIEW_SCHEMA,
                roots,
                options,
            },
            limits(),
            &token,
        )
    };
    assert_eq!(make(&roots, &[]).unwrap(), make(&reversed, &[]).unwrap());
    assert_ne!(make(&roots, &[]).unwrap(), make(&[], &[]).unwrap());
    assert_ne!(
        make(&roots, &[]).unwrap(),
        make(&roots, &[("indent", "2")]).unwrap()
    );
    let duplicate_roots = [roots[0].clone(), roots[0].clone()];
    assert_eq!(
        make(&duplicate_roots, &[]).unwrap_err(),
        IdentityError::InvalidInput
    );
    assert_eq!(
        make(&[], &[("indent", "2"), ("indent", "4")]).unwrap_err(),
        IdentityError::InvalidInput
    );
    assert_eq!(
        artifact_identity(
            derived.identity(),
            &ArtifactIdentityInput {
                kind: ArtifactKind::Project,
                format: neutral_encoding::project::ENCODING,
                roots: &roots,
                options: &[]
            },
            limits(),
            &token
        )
        .unwrap_err(),
        IdentityError::InvalidInput
    );
}

/// Independent byte/node bounds accept exact boundaries and reject one-over work.
fn assert_bounds<I: Copy + std::fmt::Debug + Eq>(
    expected: &IdentityTranscript<I>,
    make: impl Fn(IdentityLimits) -> Result<IdentityTranscript<I>, IdentityError>,
) {
    let exact = IdentityLimits {
        bytes: expected.bytes().len() as u64,
        ..limits()
    };
    assert_eq!(expected, &make(exact).unwrap());
    assert_eq!(
        make(IdentityLimits {
            bytes: exact.bytes - 1,
            ..exact
        })
        .unwrap_err(),
        IdentityError::Limit
    );
    let nodes = (1..=expected.bytes().len() as u64)
        .find(|nodes| {
            make(IdentityLimits {
                nodes: *nodes,
                ..limits()
            })
            .is_ok()
        })
        .unwrap();
    assert_eq!(
        expected,
        &make(IdentityLimits { nodes, ..limits() }).unwrap()
    );
    assert_eq!(
        make(IdentityLimits {
            nodes: nodes - 1,
            ..limits()
        })
        .unwrap_err(),
        IdentityError::Limit
    );
    for invalid in [
        IdentityLimits {
            bytes: 0,
            ..limits()
        },
        IdentityLimits {
            nodes: 0,
            ..limits()
        },
    ] {
        assert_eq!(make(invalid).unwrap_err(), IdentityError::Limit);
    }
}

/// Every identity layer enforces independently supplied exact byte and framed-node boundaries.
#[test]
fn security_project_identity_exact_byte_and_node_bounds() {
    let (capture, ir) = baseline();
    let token = CancellationToken::new();
    let data = vectors();
    let captured = capture.identity_transcript(limits(), &token).unwrap();
    assert_bounds(&captured, |bounds| {
        capture.identity_transcript(bounds, &token)
    });
    let logical = canonical_logical_project(&ir, limits(), &token).unwrap();
    assert_bounds(&logical, |bounds| {
        canonical_logical_project(&ir, bounds, &token)
    });
    let context = context(&data, captured.identity(), &capture, &ir);
    let derived = derivation_identity(logical.identity(), &context, limits(), &token).unwrap();
    assert_bounds(&derived, |bounds| {
        derivation_identity(logical.identity(), &context, bounds, &token)
    });
    let input = ArtifactIdentityInput {
        kind: ArtifactKind::Project,
        format: neutral_encoding::project::ENCODING,
        roots: &[],
        options: &[],
    };
    let artifact = artifact_identity(derived.identity(), &input, limits(), &token).unwrap();
    assert_bounds(&artifact, |bounds| {
        artifact_identity(derived.identity(), &input, bounds, &token)
    });
}

/// Duplicate module/source keys and control-bearing source identities fail closed.
#[test]
fn security_project_identity_duplicate_capture_facts() {
    let (capture, _) = baseline();
    let token = CancellationToken::new();
    let source = |module, source_id| CapturedIdentitySource {
        module,
        source_id,
        digest: capture.sources()[0].digest(),
        byte_len: capture.sources()[0].bytes().len() as u64,
    };
    for sources in [
        vec![source("example", "one"), source("example", "two")],
        vec![source("example", "same"), source("other", "same")],
        vec![source("example", "control\n")],
    ] {
        assert_eq!(
            captured_closure(
                &CapturedIdentityInput {
                    profile: V1_SOURCE_PROFILE,
                    sources: &sources,
                    vocabularies: &[],
                },
                limits(),
                &token
            )
            .unwrap_err(),
            IdentityError::InvalidInput
        );
    }
}

/// Malformed derivation/artifact context and aggregate work overruns fail closed.
#[test]
fn security_project_identity_malformed_context_and_aggregate_bounds() {
    let (capture, ir) = baseline();
    let token = CancellationToken::new();
    let data = vectors();
    let captured = capture
        .identity_transcript(limits(), &token)
        .unwrap()
        .identity();
    let logical = canonical_logical_project(&ir, limits(), &token)
        .unwrap()
        .identity();
    for field in 0..4 {
        let mut invalid = context(&data, captured, &capture, &ir);
        match field {
            0 => invalid.producer = "",
            1 => invalid.producer_version = "",
            2 => invalid.capture_limits[0] = 0,
            _ => invalid.project_limits.nodes = 0,
        }
        assert_eq!(
            derivation_identity(logical, &invalid, limits(), &token).unwrap_err(),
            IdentityError::InvalidInput
        );
    }
    let derived = derivation_identity(
        logical,
        &context(&data, captured, &capture, &ir),
        limits(),
        &token,
    )
    .unwrap()
    .identity();
    for format in ["", "non-ascii-é", "control\n"] {
        assert_eq!(
            artifact_identity(
                derived,
                &ArtifactIdentityInput {
                    kind: ArtifactKind::Project,
                    format,
                    roots: &[],
                    options: &[],
                },
                limits(),
                &token
            )
            .unwrap_err(),
            IdentityError::InvalidInput
        );
    }
    let oversized = "x".repeat(1025);
    assert_eq!(
        artifact_identity(
            derived,
            &ArtifactIdentityInput {
                kind: ArtifactKind::Project,
                format: &oversized,
                roots: &[],
                options: &[],
            },
            IdentityLimits {
                bytes: 1024,
                ..limits()
            },
            &token
        )
        .unwrap_err(),
        IdentityError::Limit
    );
    let mut excessive = ir.as_ref().clone();
    excessive.modules[0].imports = vec!["other".to_owned(); 60];
    let mut other = excessive.modules[0].clone();
    other.identity = LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "other");
    excessive.modules.push(other);
    assert_eq!(
        canonical_logical_project(
            &excessive,
            IdentityLimits {
                nodes: 100,
                ..limits()
            },
            &token
        )
        .unwrap_err(),
        IdentityError::Limit
    );
}

/// Cancellation rejects all four layers without publishing an authoritative partial transcript.
#[test]
fn security_project_identity_cancellation_all_layers() {
    let (capture, ir) = baseline();
    let token = CancellationToken::new();
    let logical = canonical_logical_project(&ir, limits(), &token).unwrap();
    let data = vectors();
    let captured = capture.identity_transcript(limits(), &token).unwrap();
    let derived = derivation_identity(
        logical.identity(),
        &context(&data, captured.identity(), &capture, &ir),
        limits(),
        &token,
    )
    .unwrap();
    token.cancel();
    assert_eq!(
        capture.identity_transcript(limits(), &token).unwrap_err(),
        IdentityError::Cancelled
    );
    assert_eq!(
        canonical_logical_project(&ir, limits(), &token).unwrap_err(),
        IdentityError::Cancelled
    );
    assert_eq!(
        derivation_identity(
            logical.identity(),
            &context(&data, captured.identity(), &capture, &ir),
            limits(),
            &token
        )
        .unwrap_err(),
        IdentityError::Cancelled
    );
    assert_eq!(
        artifact_identity(
            derived.identity(),
            &ArtifactIdentityInput {
                kind: ArtifactKind::Project,
                format: neutral_encoding::project::ENCODING,
                roots: &[],
                options: &[]
            },
            limits(),
            &token
        )
        .unwrap_err(),
        IdentityError::Cancelled
    );
}

/// Type recursion stops at the shared hard ceiling rather than exhausting the process stack.
#[test]
fn security_project_identity_type_depth_and_noncanonical_input() {
    let (_, ir) = baseline();
    let token = CancellationToken::new();
    let mut deep = ir.as_ref().clone();
    let mut ty = ProjectPublicType::String;
    for _ in 0..neutral_ir::project::PROJECT_MAX_DEPTH {
        ty = ProjectPublicType::List(Box::new(ty));
    }
    deep.declarations[0].signature = ProjectPublicSignature::Binding(ty.clone());
    assert!(canonical_logical_project(&deep, limits(), &token).is_ok());
    deep.declarations[0].signature =
        ProjectPublicSignature::Binding(ProjectPublicType::List(Box::new(ty)));
    assert_eq!(
        canonical_logical_project(&deep, limits(), &token).unwrap_err(),
        IdentityError::Limit
    );
    let mut duplicate = ir.as_ref().clone();
    duplicate
        .declarations
        .push(duplicate.declarations[0].clone());
    assert_eq!(
        canonical_logical_project(&duplicate, limits(), &token).unwrap_err(),
        IdentityError::InvalidInput
    );
    assert_ne!(LOGICAL_DOMAIN, CAPTURED_DOMAIN);
    assert_ne!(LOGICAL_DOMAIN, DERIVATION_DOMAIN);
    assert_ne!(DERIVATION_DOMAIN, ARTIFACT_DOMAIN);
    assert_ne!(
        SourceContentDigest::from_bytes(b"same").as_bytes(),
        SemanticDigest::from_nht(LOGICAL_DOMAIN, b"same")
            .unwrap()
            .as_bytes()
    );
}
