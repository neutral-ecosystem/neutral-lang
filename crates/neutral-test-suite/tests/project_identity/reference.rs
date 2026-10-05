// SPDX-License-Identifier: Apache-2.0

//! Test-only typed-fact projection into a separately maintained Python encoder.

use super::{
    CapturedProject, IdentityLimits, ModuleSymbolIdentity, ProjectIr, ProjectPublicSignature,
    ProjectPublicType, Value, hex, limits, text, vectors,
};
use neutral_ir::project::ProjectValue;
use serde_json::json;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

/// Projects a stable symbol tuple, without producing any canonical transcript bytes.
pub(super) fn symbol(value: &ModuleSymbolIdentity) -> Value {
    json!([
        value.module().language_behavior_version(),
        value.module().module_name(),
        value.declaration_name()
    ])
}

/// Projects resolved type facts independently of the production transcript encoder.
fn ty(value: &ProjectPublicType) -> Value {
    match value {
        ProjectPublicType::Num => json!({"kind": "num"}),
        ProjectPublicType::String => json!({"kind": "string"}),
        ProjectPublicType::Bool => json!({"kind": "bool"}),
        ProjectPublicType::Url => json!({"kind": "url"}),
        ProjectPublicType::Path => json!({"kind": "path"}),
        ProjectPublicType::Nominal(value) => json!({"kind": "nominal", "symbol": symbol(value)}),
        ProjectPublicType::VocabularyNominal {
            identity,
            version,
            name,
        } => json!({"kind": "vocabulary", "identity": identity, "version": version, "name": name}),
        ProjectPublicType::List(value) => json!({"kind": "List", "inner": ty(value)}),
        ProjectPublicType::Ref(value) => json!({"kind": "Ref", "inner": ty(value)}),
        ProjectPublicType::Nullable(value) => json!({"kind": "nullable", "inner": ty(value)}),
    }
}

/// Projects materialized typed values, preserving lists and canonical record field order.
fn value(value: &ProjectValue) -> Value {
    match value {
        ProjectValue::Number(value) => {
            json!({"kind": "num", "negative": value.is_negative(), "coefficient": value.coefficient(), "scale": value.scale()})
        }
        ProjectValue::String(value) => json!({"kind": "string", "value": value}),
        ProjectValue::Url(value) => json!({"kind": "url", "value": value}),
        ProjectValue::Path(value) => json!({"kind": "path", "value": value}),
        ProjectValue::Bool(value) => json!({"kind": "bool", "value": value}),
        ProjectValue::Null => json!({"kind": "null"}),
        ProjectValue::Reference(value) => json!({"kind": "Ref", "value": symbol(value)}),
        ProjectValue::List(values) => {
            json!({"kind": "List", "value": values.iter().map(self::value).collect::<Vec<_>>()})
        }
        ProjectValue::Record(values) => {
            json!({"kind": "record", "value": values.iter().map(|(name, v)| json!([name, self::value(v)])).collect::<Vec<_>>()})
        }
    }
}

/// Projects explicit project bounds in the frozen independent-reference tuple order.
fn project_limits(ir: &ProjectIr) -> [u64; 6] {
    let value = ir.limits;
    [
        value.modules,
        value.declarations,
        value.import_edges,
        value.nodes,
        value.text_bytes,
        value.artifact_bytes,
    ]
}

/// Builds complete test facts without consulting production hash or framing functions.
pub(super) fn request(capture: &CapturedProject, ir: &ProjectIr, bounds: IdentityLimits) -> Value {
    let data = vectors();
    json!({
        "limits": {"bytes": bounds.bytes, "nodes": bounds.nodes},
        "producer": text(&data["input"], "producer"),
        "producer_version": text(&data["input"], "producer_version"),
        "capture_limits": capture.identity_capture_limits(),
        "project_limits": project_limits(ir),
        "capture": {
            "profile": capture.profile().source_version(),
            "sources": capture.sources().iter().map(|source| json!({"module": source.module_id(), "source_id": source.source_id(), "digest": hex(&source.digest().as_bytes()), "byte_len": source.bytes().len()})).collect::<Vec<_>>(),
            "vocabularies": capture.vocabularies().iter().map(|v| {
                let lock = v.lock();
                json!({"identity": lock.identity(), "version": lock.version(), "encoding_version": lock.encoding_version(), "schema_version": lock.schema_version(), "digest": hex(&lock.content_digest().as_bytes()), "byte_len": v.bytes().len(), "features": lock.required_features()})
            }).collect::<Vec<_>>()
        },
        "logical": {
            "schema": ir.schema,
            "modules": ir.modules.iter().map(|m| json!({"profile": m.identity.language_behavior_version(), "name": m.identity.module_name(), "imports": m.imports})).collect::<Vec<_>>(),
            "declarations": ir.declarations.iter().map(|d| json!({
                "symbol": symbol(&d.identity), "public": d.public,
                "signature": match &d.signature {
                    ProjectPublicSignature::Binding(v) => json!({"kind": "binding", "type": ty(v)}),
                    ProjectPublicSignature::Record(fields) => json!({"kind": "record", "fields": fields.iter().map(|f| json!([f.name(), ty(f.ty())])).collect::<Vec<_>>()}),
                },
                "value": d.value.as_ref().map_or_else(|| json!({"kind": "absent"}), value),
                "defaults": d.defaults.iter().map(|(name, v)| json!([name, value(v)])).collect::<Vec<_>>()
            })).collect::<Vec<_>>(),
            "records": ir.vocabulary_records.iter().map(|r| json!({"identity": r.identity, "version": r.version, "name": r.name, "public": r.public, "fields": r.fields.iter().map(|(name, v)| json!([name, ty(v)])).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "catalogues": ir.public_interface.vocabularies().iter().map(|v| json!({"identity": v.identity(), "version": v.version(), "public_types": v.public_types()})).collect::<Vec<_>>(),
            "edges": ir.public_interface.edges().iter().map(|e| json!({"from": symbol(e.from()), "to": symbol(e.to()), "kind": e.kind().spelling()})).collect::<Vec<_>>()
        },
        "artifacts": [
            {"layer": "artifact-project", "kind": "project", "format": text(&data["input"], "artifact_format"), "roots": [], "options": []},
            {"layer": "artifact-view", "kind": "view", "format": text(&data["input"], "view_format"), "roots": ir.public_interface.exports().first().map(|e| vec![symbol(e.identity())]).unwrap_or_default(), "options": []},
            {"layer": "artifact-format", "kind": "project", "format": text(&data["input"], "alternate_format"), "roots": [], "options": []}
        ]
    })
}

/// Executes the independent standard-library encoder, failing rather than skipping missing tools.
pub(super) fn inspect(request: &Value) -> Value {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/project_identity/reference.py");
    let configured = std::env::var("NEUTRAL_IDENTITY_REFERENCE_PYTHON").ok();
    let programs = configured
        .as_deref()
        .map_or_else(|| vec!["python3", "python"], |value| vec![value]);
    let mut child = programs.iter().find_map(|program| {
        match Command::new(program).arg("-B").arg(&script).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(child) => Some(child),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("[error] cannot run identity reference: {error}"),
        }
    }).expect("[error] install Python 3 or set NEUTRAL_IDENTITY_REFERENCE_PYTHON; independent vectors cannot be skipped");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "[error] reference encoder: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// The independent implementation reproduces all six immutable published vectors exactly.
#[test]
fn conformance_project_identity_second_implementation_literal_vectors() {
    let (capture, ir) = super::baseline();
    let actual = inspect(&request(&capture, &ir, limits()));
    for expected in vectors()["vectors"].as_array().unwrap() {
        let layer = text(expected, "layer");
        for field in ["transcript_hex", "sha256", "transcript_bytes"] {
            assert_eq!(actual[layer][field], expected[field], "{layer}/{field}");
        }
    }
}

/// Compares complete bytes and digests for every identity layer in one accepted case.
pub(super) fn compare(capture: &CapturedProject, ir: &ProjectIr) {
    let token = neutral_core::CancellationToken::new();
    let reader = neutral_reader::ValidatedProject::from_ir(
        std::sync::Arc::new(ir.clone()),
        ir.limits,
        &token,
    )
    .unwrap();
    let ids = super::integration::identities(&reader, capture);
    let input = request(capture, ir, limits());
    let output = inspect(&input);
    for (layer, bytes, digest) in [
        (
            "captured",
            ids.captured().bytes(),
            ids.captured().identity().to_string(),
        ),
        (
            "logical",
            ids.logical().bytes(),
            ids.logical().identity().to_string(),
        ),
        (
            "derivation",
            ids.derivation().bytes(),
            ids.derivation().identity().to_string(),
        ),
    ] {
        assert_eq!(output[layer]["transcript_hex"], hex(bytes), "{layer}");
        assert_eq!(output[layer]["sha256"], digest, "{layer}");
        assert_eq!(
            output[layer]["transcript_bytes"],
            bytes.len() as u64,
            "{layer}"
        );
    }
    for artifact in input["artifacts"].as_array().unwrap() {
        let roots = if artifact["kind"] == "view" {
            ir.public_interface
                .exports()
                .first()
                .map(|e| vec![e.identity().clone()])
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let actual = ids
            .artifact(
                &super::ArtifactIdentityInput {
                    kind: if artifact["kind"] == "view" {
                        super::ArtifactKind::View
                    } else {
                        super::ArtifactKind::Project
                    },
                    format: artifact["format"].as_str().unwrap(),
                    roots: &roots,
                    options: &[],
                },
                limits(),
                &token,
            )
            .unwrap();
        let layer = artifact["layer"].as_str().unwrap();
        assert_eq!(
            output[layer]["transcript_hex"],
            hex(actual.bytes()),
            "{layer}"
        );
        assert_eq!(
            output[layer]["sha256"],
            actual.identity().to_string(),
            "{layer}"
        );
    }
}

/// The independent encoder covers locked vocabulary schemas, cycles, private bodies, and every value/type family.
#[test]
fn conformance_project_identity_second_implementation_complete_corpus() {
    for fixture in [
        include_str!("../project_ir/complete.toml"),
        include_str!("../project_ir/private-only.toml"),
        include_str!("../module_graph/fixtures/positive/valid-cycle.toml"),
        include_str!("../public_semantics/fixtures/positive/vocabulary-multiple-alias.toml"),
        include_str!("../public_semantics/fixtures/positive/reuse-provenance.toml"),
        include_str!("../public_semantics/fixtures/positive/location-values.toml"),
    ] {
        let capture =
            neutral_compiler::capture_project(crate::project_capture::request_fixture(fixture))
                .unwrap();
        let ir =
            neutral_compiler::compile_project(&capture, &neutral_core::CancellationToken::new())
                .unwrap();
        compare(&capture, &ir);
    }
    for body in [
        "record Item { num count = -1.25e2, string? note = null, }\npublic num? absent = null\npublic List<List<num>> values = [[1,2], []]\npublic bool active = true\npublic url site = \"https://example.invalid\"\npublic path location = \"../inert\"\nItem item = {}\n",
        "public string value = \"emoji: 🦀 and h\\u{65}llo\"\n",
        "public string value = \"abc\"\npublic string other = \"ab:c\"\n",
    ] {
        let source = format!(
            "neu \"{}\"\nmodule example\n{body}",
            super::V1_SOURCE_PROFILE
        );
        let capture = super::captured("unit:reference", &source, super::capture_limits());
        let ir =
            neutral_compiler::compile_project(&capture, &neutral_core::CancellationToken::new())
                .unwrap();
        compare(&capture, &ir);
    }
}

/// Both encoders reject noncanonical logical order, duplicate imports, schema changes, and depth violations.
#[test]
fn security_project_identity_second_implementation_adversarial_logical_vectors() {
    let token = neutral_core::CancellationToken::new();
    let capture = neutral_compiler::capture_project(crate::project_capture::request_fixture(
        include_str!("../project_ir/complete.toml"),
    ))
    .unwrap();
    let original = neutral_compiler::compile_project(&capture, &token).unwrap();
    for case in 0..6 {
        let mut ir = original.as_ref().clone();
        match case {
            0 => ir.schema = "unrecognized-schema".to_owned(),
            1 => ir.modules.reverse(),
            2 => ir.declarations.reverse(),
            3 => {
                let import = ir.modules[0].imports[0].clone();
                ir.modules[0].imports.push(import);
            }
            4 => {
                let fields = ir
                    .declarations
                    .iter_mut()
                    .find_map(|d| match &mut d.signature {
                        ProjectPublicSignature::Record(fields) => Some(fields),
                        ProjectPublicSignature::Binding(_) => None,
                    })
                    .unwrap();
                fields.reverse();
            }
            _ => {
                ir.modules.clear();
            }
        }
        assert_eq!(
            super::canonical_logical_project(&ir, limits(), &token).unwrap_err(),
            super::IdentityError::InvalidInput,
            "case {case}"
        );
        assert_eq!(
            inspect(&request(&capture, &ir, limits()))["logical"]["error"],
            "InvalidInput",
            "case {case}"
        );
    }
    let (_, minimal) = super::baseline();
    let mut deep = minimal.as_ref().clone();
    let mut ty = ProjectPublicType::Num;
    for _ in 0..=neutral_ir::project::PROJECT_MAX_DEPTH {
        ty = ProjectPublicType::List(Box::new(ty));
    }
    deep.declarations[0].signature = ProjectPublicSignature::Binding(ty);
    assert_eq!(
        super::canonical_logical_project(&deep, limits(), &token).unwrap_err(),
        super::IdentityError::Limit
    );
    assert_eq!(
        inspect(&request(&capture, &deep, limits()))["logical"]["error"],
        "Limit"
    );
}

/// Independent cancellation, byte/node limits, and malformed artifact/context classifications agree.
#[test]
fn security_project_identity_second_implementation_bounds_and_context() {
    let (capture, ir) = super::baseline();
    let token = neutral_core::CancellationToken::new();
    let captured_bytes = capture
        .identity_transcript(limits(), &token)
        .unwrap()
        .bytes()
        .len() as u64;
    for bounds in [
        IdentityLimits {
            bytes: 0,
            ..limits()
        },
        IdentityLimits {
            nodes: 0,
            ..limits()
        },
        IdentityLimits {
            bytes: captured_bytes - 1,
            ..limits()
        },
        IdentityLimits {
            nodes: 1,
            ..limits()
        },
    ] {
        let result = inspect(&request(&capture, &ir, bounds));
        assert_eq!(
            capture.identity_transcript(bounds, &token).unwrap_err(),
            super::IdentityError::Limit
        );
        assert_eq!(result["captured"]["error"], "Limit");
        assert_eq!(
            super::canonical_logical_project(&ir, bounds, &token).unwrap_err(),
            super::IdentityError::Limit
        );
        assert_eq!(result["logical"]["error"], "Limit");
    }
    let mut cancelled = request(&capture, &ir, limits());
    cancelled["cancelled"] = json!(true);
    assert_eq!(inspect(&cancelled)["captured"]["error"], "Cancelled");
    assert_eq!(inspect(&cancelled)["logical"]["error"], "Cancelled");
}

/// Malformed derivation context and format identifiers agree across both implementations.
#[test]
fn security_project_identity_second_implementation_derivation_context() {
    let (capture, ir) = super::baseline();
    let token = neutral_core::CancellationToken::new();
    let captured = capture.identity_transcript(limits(), &token).unwrap();
    let logical = super::canonical_logical_project(&ir, limits(), &token).unwrap();
    let data = vectors();
    for field in 0..4 {
        let mut invalid = request(&capture, &ir, limits());
        let mut context = super::context(&data, captured.identity(), &capture, &ir);
        match field {
            0 => {
                invalid["producer"] = json!("");
                context.producer = "";
            }
            1 => {
                invalid["producer_version"] = json!("");
                context.producer_version = "";
            }
            2 => {
                invalid["capture_limits"][0] = json!(0);
                context.capture_limits[0] = 0;
            }
            _ => {
                invalid["project_limits"][3] = json!(0);
                context.project_limits.nodes = 0;
            }
        }
        assert_eq!(
            super::derivation_identity(logical.identity(), &context, limits(), &token).unwrap_err(),
            super::IdentityError::InvalidInput
        );
        assert_eq!(inspect(&invalid)["derivation"]["error"], "InvalidInput");
    }
    let derived = super::derivation_identity(
        logical.identity(),
        &super::context(&data, captured.identity(), &capture, &ir),
        limits(),
        &token,
    )
    .unwrap();
    for format in ["", "invalid-é", "control\n"] {
        let mut invalid = request(&capture, &ir, limits());
        invalid["artifacts"][0]["format"] = json!(format);
        assert_eq!(
            super::artifact_identity(
                derived.identity(),
                &super::ArtifactIdentityInput {
                    kind: super::ArtifactKind::Project,
                    format,
                    roots: &[],
                    options: &[]
                },
                limits(),
                &token
            )
            .unwrap_err(),
            super::IdentityError::InvalidInput
        );
        assert_eq!(
            inspect(&invalid)["artifact-project"]["error"],
            "InvalidInput"
        );
    }
}

/// Tagged tuple boundaries distinguish inputs that collide under delimiter-free concatenation.
#[test]
fn property_project_identity_framing_collision_paths_remain_distinct() {
    let token = neutral_core::CancellationToken::new();
    let make = |module: &str, name: &str| {
        let source = format!(
            "neu \"{}\"\nmodule {module}\npublic string {name} = \"hello\"\n",
            super::V1_SOURCE_PROFILE
        );
        let capture =
            neutral_compiler::capture_project(neutral_compiler::CapturedProjectRequest::new(
                neutral_compiler::CAPTURE_REQUEST_VERSION,
                neutral_core::profile::LanguageProfile::V1_0,
                vec![neutral_compiler::CapturedSourceInput::new(
                    "unit:collision",
                    module,
                    source.into_bytes(),
                )],
                Vec::new(),
                neutral_compiler::ProjectCaptureControls::new(
                    super::capture_limits(),
                    token.clone(),
                ),
            ))
            .unwrap();
        let ir = neutral_compiler::compile_project(&capture, &token).unwrap();
        compare(&capture, &ir);
        super::canonical_logical_project(&ir, limits(), &token).unwrap()
    };
    assert_eq!(format!("{}{}", "ab", "c"), format!("{}{}", "a", "bc"));
    assert_ne!(make("ab", "c"), make("a", "bc"));
}

/// Every independent layer accepts exact byte/frame bounds and rejects one below them.
#[test]
fn security_project_identity_second_implementation_exact_layer_bounds() {
    let (capture, ir) = super::baseline();
    let token = neutral_core::CancellationToken::new();
    let mut request = request(&capture, &ir, limits());
    let all = inspect(&request);
    request["upstream"] = json!({"captured": all["captured"]["sha256"], "logical": all["logical"]["sha256"], "derivation": all["derivation"]["sha256"]});
    request["artifact"] = request["artifacts"][0].clone();
    let captured = capture.identity_transcript(limits(), &token).unwrap();
    let logical = super::canonical_logical_project(&ir, limits(), &token).unwrap();
    let data = vectors();
    let context = super::context(&data, captured.identity(), &capture, &ir);
    let derived =
        super::derivation_identity(logical.identity(), &context, limits(), &token).unwrap();
    for (operation, vector) in [
        ("captured", "captured"),
        ("logical", "logical"),
        ("derivation", "derivation"),
        ("artifact", "artifact-project"),
    ] {
        request["operation"] = json!(operation);
        let bytes = all[vector]["transcript_bytes"].as_u64().unwrap();
        let nodes = all[vector]["frames"].as_u64().unwrap();
        for bounds in [
            IdentityLimits { bytes, nodes },
            IdentityLimits {
                bytes: bytes - 1,
                nodes,
            },
            IdentityLimits {
                bytes,
                nodes: nodes - 1,
            },
            IdentityLimits { bytes: 0, nodes },
            IdentityLimits { bytes, nodes: 0 },
        ] {
            request["limits"] = json!({"bytes": bounds.bytes, "nodes": bounds.nodes});
            let expected = inspect(&request);
            let actual = match operation {
                "captured" => capture
                    .identity_transcript(bounds, &token)
                    .map(|v| (hex(v.bytes()), v.identity().to_string())),
                "logical" => super::canonical_logical_project(&ir, bounds, &token)
                    .map(|v| (hex(v.bytes()), v.identity().to_string())),
                "derivation" => {
                    super::derivation_identity(logical.identity(), &context, bounds, &token)
                        .map(|v| (hex(v.bytes()), v.identity().to_string()))
                }
                _ => super::artifact_identity(
                    derived.identity(),
                    &super::ArtifactIdentityInput {
                        kind: super::ArtifactKind::Project,
                        format: text(&data["input"], "artifact_format"),
                        roots: &[],
                        options: &[],
                    },
                    bounds,
                    &token,
                )
                .map(|v| (hex(v.bytes()), v.identity().to_string())),
            };
            match actual {
                Ok((transcript, digest)) => {
                    assert_eq!(expected["transcript_hex"], transcript);
                    assert_eq!(expected["sha256"], digest);
                }
                Err(error) => {
                    assert_eq!(error, super::IdentityError::Limit);
                    assert_eq!(expected["error"], "Limit");
                }
            }
        }
        request["limits"] = json!({"bytes": limits().bytes, "nodes": limits().nodes});
        request["cancelled"] = json!(true);
        assert_eq!(inspect(&request)["error"], "Cancelled");
        request["cancelled"] = json!(false);
    }
}

/// Both implementations reject duplicate, unordered, empty, and control-bearing capture facts.
#[test]
fn security_project_identity_second_implementation_captured_adversarial_vectors() {
    let (capture, ir) = super::baseline();
    let token = neutral_core::CancellationToken::new();
    for case in 0..7 {
        let mut request = request(&capture, &ir, limits());
        request["operation"] = json!("captured");
        match case {
            0 => {
                request["capture"]["sources"] = json!([]);
            }
            1 => {
                request["capture"]["profile"] = json!("");
            }
            2 => {
                request["capture"]["sources"][0]["module"] = json!("");
            }
            3 => {
                request["capture"]["sources"][0]["source_id"] = json!("");
            }
            4 => {
                request["capture"]["sources"][0]["source_id"] = json!("unit:\u{0085}");
            }
            5 => {
                let duplicate = request["capture"]["sources"][0].clone();
                request["capture"]["sources"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            _ => {
                let mut other = request["capture"]["sources"][0].clone();
                other["module"] = json!("aaa");
                other["source_id"] = json!("unit:other");
                request["capture"]["sources"]
                    .as_array_mut()
                    .unwrap()
                    .push(other);
            }
        }
        let sources = request["capture"]["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| super::CapturedIdentitySource {
                module: source["module"].as_str().unwrap(),
                source_id: source["source_id"].as_str().unwrap(),
                digest: capture.sources()[0].digest(),
                byte_len: source["byte_len"].as_u64().unwrap(),
            })
            .collect::<Vec<_>>();
        let actual = super::captured_closure(
            &super::CapturedIdentityInput {
                profile: request["capture"]["profile"].as_str().unwrap(),
                sources: &sources,
                vocabularies: &[],
            },
            limits(),
            &token,
        );
        assert_eq!(
            actual.unwrap_err(),
            super::IdentityError::InvalidInput,
            "case {case}"
        );
        assert_eq!(inspect(&request)["error"], "InvalidInput", "case {case}");
    }
}

/// Duplicate/unsorted artifact options and roots are rejected by both encoders, not merged.
#[test]
fn security_project_identity_second_implementation_artifact_adversarial_vectors() {
    let (capture, ir) = super::baseline();
    let token = neutral_core::CancellationToken::new();
    let reader =
        neutral_reader::ValidatedProject::from_ir(std::sync::Arc::clone(&ir), ir.limits, &token)
            .unwrap();
    let ids = super::integration::identities(&reader, &capture);
    for (roots, options, kind) in [
        (
            vec![ir.declarations[0].identity.clone(); 2],
            Vec::new(),
            super::ArtifactKind::View,
        ),
        (
            Vec::new(),
            vec![("same", "a"), ("same", "b")],
            super::ArtifactKind::Project,
        ),
        (
            Vec::new(),
            vec![("z", "a"), ("a", "b")],
            super::ArtifactKind::Project,
        ),
        (
            Vec::new(),
            vec![("", "empty")],
            super::ArtifactKind::Project,
        ),
        (
            vec![ir.declarations[0].identity.clone()],
            Vec::new(),
            super::ArtifactKind::Project,
        ),
    ] {
        let mut request = request(&capture, &ir, limits());
        request["artifacts"][0]["kind"] = json!(if kind == super::ArtifactKind::View {
            "view"
        } else {
            "project"
        });
        request["artifacts"][0]["roots"] = json!(roots.iter().map(symbol).collect::<Vec<_>>());
        request["artifacts"][0]["options"] = json!(options);
        let actual = super::artifact_identity(
            ids.derivation().identity(),
            &super::ArtifactIdentityInput {
                kind,
                format: text(&vectors()["input"], "artifact_format"),
                roots: &roots,
                options: &options,
            },
            limits(),
            &token,
        );
        assert_eq!(actual.unwrap_err(), super::IdentityError::InvalidInput);
        assert_eq!(
            inspect(&request)["artifact-project"]["error"],
            "InvalidInput"
        );
    }
}

/// Independent artifact framing agrees for multiple reordered roots and explicit options.
#[test]
fn property_project_identity_second_implementation_artifact_selections() {
    let token = neutral_core::CancellationToken::new();
    let capture = neutral_compiler::capture_project(crate::project_capture::request_fixture(
        include_str!("../project_ir/complete.toml"),
    ))
    .unwrap();
    let ir = neutral_compiler::compile_project(&capture, &token).unwrap();
    let reader =
        neutral_reader::ValidatedProject::from_ir(std::sync::Arc::clone(&ir), ir.limits, &token)
            .unwrap();
    let ids = super::integration::identities(&reader, &capture);
    let roots = ir
        .public_interface
        .exports()
        .iter()
        .map(|e| e.identity().clone())
        .collect::<Vec<_>>();
    let reversed = roots.iter().rev().cloned().collect::<Vec<_>>();
    let options = [("encoding", "canonical"), ("presentation", "compact")];
    let mut outputs = Vec::new();
    for selection in [&roots[..], &reversed[..], &[][..], &roots[..1]] {
        let mut input = request(&capture, &ir, limits());
        input["artifacts"][0]["kind"] = json!("view");
        input["artifacts"][0]["roots"] = json!(selection.iter().map(symbol).collect::<Vec<_>>());
        input["artifacts"][0]["options"] = json!(options);
        let actual = ids
            .artifact(
                &super::ArtifactIdentityInput {
                    kind: super::ArtifactKind::View,
                    format: text(&vectors()["input"], "artifact_format"),
                    roots: selection,
                    options: &options,
                },
                limits(),
                &token,
            )
            .unwrap();
        let expected = inspect(&input);
        assert_eq!(
            expected["artifact-project"]["transcript_hex"],
            hex(actual.bytes())
        );
        assert_eq!(
            expected["artifact-project"]["sha256"],
            actual.identity().to_string()
        );
        outputs.push(actual);
    }
    assert_eq!(outputs[0], outputs[1]);
    assert_ne!(outputs[0], outputs[2]);
    assert_ne!(outputs[0], outputs[3]);
}
