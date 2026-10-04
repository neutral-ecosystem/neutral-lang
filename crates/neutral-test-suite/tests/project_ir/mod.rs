// SPDX-License-Identifier: Apache-2.0

//! Complete typed project publication, malformed artifacts, and root-independent views.

use crate::project_capture::{parse_fixture, request_fixture, required_string};
use neutral_compiler::{
    CapturedProjectRequest, CapturedSourceInput, ProjectCompileFailure, capture_project,
    compile_project,
};
use neutral_core::{ByteSpan, CancellationToken, SourceLocation};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    project::*,
    project_interface::{ProjectPublicSignature, ProjectPublicType},
};
use neutral_reader::{ProjectReadError, ValidatedProject};
use std::sync::Arc;

/// Reviewed complete three-module request, shared with portable conformance.
const COMPLETE: &str = include_str!("complete.toml");

/// Forged source ownership and omitted type edges cannot cross the reader boundary.
#[test]
fn security_project_ir_dependency_companions_are_complete() {
    let ir = complete();
    let mut corrupt = ir.as_ref().clone();
    let other = corrupt
        .sources
        .iter()
        .find(|source| source.digest != corrupt.provenance[0].location.source())
        .unwrap();
    corrupt.provenance[0].location =
        SourceLocation::new(other.digest, ByteSpan::new(0, 1).unwrap());
    assert_eq!(
        ValidatedProject::from_ir(Arc::new(corrupt), ir.limits, &CancellationToken::new())
            .unwrap_err(),
        ProjectReadError::Companion
    );
    let mut corrupt = ir.as_ref().clone();
    let position = corrupt
        .provenance
        .iter()
        .position(|edge| edge.kind == neutral_ir::project_interface::ProjectPublicEdgeKind::Type)
        .unwrap();
    corrupt.provenance.remove(position);
    assert_eq!(
        ValidatedProject::from_ir(Arc::new(corrupt), ir.limits, &CancellationToken::new())
            .unwrap_err(),
        ProjectReadError::Companion
    );
}

/// Locked schemas with an embedded self cycle are independently rejected.
#[test]
fn security_project_ir_vocabulary_schema_cycles_fail_closed() {
    let captured = capture_project(request_fixture(include_str!(
        "../public_semantics/fixtures/positive/vocabulary-multiple-alias.toml"
    )))
    .unwrap();
    let ir = compile_project(&captured, &CancellationToken::new()).unwrap();
    let mut corrupt = ir.as_ref().clone();
    let record = &mut corrupt.vocabulary_records[0];
    record.fields.push((
        "nested".to_owned(),
        ProjectPublicType::VocabularyNominal {
            identity: record.identity.clone(),
            version: record.version.clone(),
            name: record.name.clone(),
        },
    ));
    assert_eq!(
        ValidatedProject::from_ir(Arc::new(corrupt), ir.limits, &CancellationToken::new())
            .unwrap_err(),
        ProjectReadError::Declaration
    );
}

/// All supplied private-only modules remain complete even with an empty public index.
#[test]
fn conformance_project_ir_private_only_fixture() {
    let captured = capture_project(request_fixture(include_str!("private-only.toml"))).unwrap();
    let ir = compile_project(&captured, &CancellationToken::new()).unwrap();
    let reader =
        ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &CancellationToken::new()).unwrap();
    assert_eq!(ir.declarations.len(), 2);
    let view = reader
        .derive_view(
            &ViewRequest {
                schema: PROJECT_VIEW_SCHEMA.to_owned(),
                roots: Vec::new(),
            },
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(view.exports(), []);
    assert_eq!(view.values(), []);
}

/// Builds a fresh exact request with explicit fixture controls and no ambient input.
fn request() -> CapturedProjectRequest {
    request_fixture(COMPLETE)
}

/// Compiles the complete reviewed fixture without any host acquisition.
fn complete() -> Arc<ProjectIr> {
    let captured = capture_project(request()).expect("captured fixture");
    compile_project(&captured, &CancellationToken::new()).expect("complete project")
}

/// Constructs one exact module-symbol identity using the shared profile contract.
fn symbol(module: &str, name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(neutral_core::profile::V1_SOURCE_PROFILE, module),
        name,
    )
}

/// Compiles a single source using the same explicit reviewed controls.
fn single(body: &str) -> Result<Arc<ProjectIr>, ProjectCompileFailure> {
    let source = CapturedSourceInput::new(
        "unit:example",
        "example",
        format!("neu \"1.0\"\nmodule example\n{body}").into_bytes(),
    );
    let limits = capture_project(request())
        .expect("fixture controls")
        .limits();
    let captured = capture_project(CapturedProjectRequest::new(
        neutral_compiler::CAPTURE_REQUEST_VERSION,
        neutral_core::profile::LanguageProfile::V1_0,
        vec![source],
        Vec::new(),
        neutral_compiler::ProjectCaptureControls::new(limits, CancellationToken::new()),
    ))
    .expect("single request");
    compile_project(&captured, &CancellationToken::new())
}

/// Checks complete/private/disconnected membership and materialized closed defaults.
#[test]
fn conformance_project_ir_complete_private_public_fixture() {
    let ir = complete();
    let reader = ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &CancellationToken::new())
        .expect("independent reader");
    assert_eq!(reader.complete_ir().modules.len(), 3);
    assert_eq!(ir.resources.source_units, 3);
    assert_eq!(ir.resources.declarations, ir.declarations.len() as u64);
    assert!(
        ir.declarations
            .iter()
            .any(|decl| !decl.public && decl.identity.module().module_name() == "app::orphan")
    );
    let api = ir
        .declarations
        .iter()
        .find(|decl| decl.identity == symbol("app::shared", "api"))
        .unwrap();
    let ProjectValue::Record(fields) = api.value.as_ref().unwrap() else {
        panic!("materialized record");
    };
    assert_eq!(
        fields
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["label", "ports"]
    );
    assert_eq!(
        fields[0].1,
        ProjectValue::String("materialized private value".to_owned())
    );
    assert!(matches!(&fields[1].1, ProjectValue::List(values) if values.len() == 2));
}

/// Views retain interpretive types/value/reference targets while omitting private roots.
#[test]
fn integration_project_ir_views_preserve_closure_and_complete_meaning() {
    let ir = complete();
    let before = ir.as_ref().clone();
    let reader =
        ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &CancellationToken::new()).unwrap();
    let request = ViewRequest {
        schema: PROJECT_VIEW_SCHEMA.to_owned(),
        roots: vec![symbol("app::consumer", "pointer")],
    };
    let view = reader
        .derive_view(&request, &CancellationToken::new())
        .unwrap();
    let names = view
        .exports()
        .iter()
        .map(|export| export.identity().declaration_name())
        .collect::<Vec<_>>();
    assert_eq!(names, ["pointer", "Service", "api"]);
    assert!(!names.contains(&"hidden"));
    assert!(!names.contains(&"unused"));
    assert_eq!(before, *ir);
    assert!(before.logical_eq(&ir));
    let private = ViewRequest {
        schema: PROJECT_VIEW_SCHEMA.to_owned(),
        roots: vec![symbol("app::shared", "hidden")],
    };
    assert_eq!(
        reader.derive_view(&private, &CancellationToken::new()),
        Err(ProjectReadError::View)
    );
    let duplicated = ViewRequest {
        roots: vec![request.roots[0].clone(), request.roots[0].clone()],
        ..request
    };
    assert_eq!(
        reader.derive_view(&duplicated, &CancellationToken::new()),
        Err(ProjectReadError::View)
    );
}

/// Contextual mismatches and invalid unused defaults cannot publish a project.
#[test]
fn security_project_ir_contextual_failures_are_atomic() {
    for source in [
        "public num value = \"wrong\"\n",
        "record Item { num value, }\nItem item = {}\n",
        "record Item { num value, }\nItem item = { unknown: 1, }\n",
        "record Item { num value, }\nItem item = { value: 1, value: 2, }\n",
        "record Item { num value = \"wrong\", }\n",
        "num seed = 1\nrecord Item { num value = seed, }\n",
        "record Item { Ref<num> value = ref(seed), }\nnum seed = 1\n",
        "public List<num> values = [1, \"wrong\"]\n",
        "public num value = null\n",
    ] {
        assert!(single(source).is_err(), "{source}");
    }
}

/// Independent artifact checks reject forged resource facts, values, maps, and schema.
#[test]
fn security_project_ir_malformed_artifacts_fail_closed() {
    let ir = complete();
    let mut corrupt = ir.as_ref().clone();
    corrupt.resources.value_nodes += 1;
    assert!(matches!(
        ValidatedProject::from_ir(Arc::new(corrupt), ir.limits, &CancellationToken::new()),
        Err(ProjectReadError::Resources)
    ));
    let mut corrupt = ir.as_ref().clone();
    corrupt.source_maps.pop();
    assert!(matches!(
        ValidatedProject::from_ir(Arc::new(corrupt), ir.limits, &CancellationToken::new()),
        Err(ProjectReadError::Companion)
    ));
    let mut corrupt = ir.as_ref().clone();
    corrupt.source_maps[0].location = SourceLocation::new(
        corrupt.source_maps[0].location.source(),
        ByteSpan::new(0, u64::MAX).unwrap(),
    );
    assert!(
        ValidatedProject::from_ir(Arc::new(corrupt), ir.limits, &CancellationToken::new()).is_err()
    );
    let mut corrupt = ir.as_ref().clone();
    let binding = corrupt
        .declarations
        .iter_mut()
        .find(|decl| decl.value.is_some())
        .unwrap();
    binding.value = Some(ProjectValue::Bool(false));
    assert!(matches!(
        ValidatedProject::from_ir(Arc::new(corrupt), ir.limits, &CancellationToken::new()),
        Err(ProjectReadError::Declaration)
    ));
    let mut corrupt = ir.as_ref().clone();
    corrupt.schema = "unknown".to_owned();
    assert!(matches!(
        ValidatedProject::from_ir(Arc::new(corrupt), ir.limits, &CancellationToken::new()),
        Err(ProjectReadError::Schema)
    ));
}

/// Cancellation and each publication bound prevent partial reader/view success.
#[test]
fn security_project_ir_limits_and_cancellation_are_bounded() {
    let ir = complete();
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(matches!(
        ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &cancelled),
        Err(ProjectReadError::Cancelled)
    ));
    let captured = capture_project(request()).unwrap();
    assert!(compile_project(&captured, &cancelled).is_err());
    for limits in [
        ProjectLimits {
            modules: 2,
            ..ir.limits
        },
        ProjectLimits {
            declarations: 1,
            ..ir.limits
        },
        ProjectLimits {
            nodes: 1,
            ..ir.limits
        },
        ProjectLimits {
            text_bytes: 1,
            ..ir.limits
        },
        ProjectLimits {
            import_edges: 0,
            ..ir.limits
        },
    ] {
        assert!(
            ValidatedProject::from_ir(Arc::clone(&ir), limits, &CancellationToken::new()).is_err()
        );
    }
    let reader =
        ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &CancellationToken::new()).unwrap();
    assert_eq!(
        reader.derive_view(
            &ViewRequest {
                schema: PROJECT_VIEW_SCHEMA.to_owned(),
                roots: Vec::new()
            },
            &cancelled
        ),
        Err(ProjectReadError::Cancelled)
    );
}

/// Nullable, exact numeric, record default, nested list, and inert scalar values are typed.
#[test]
fn unit_project_ir_complete_contextual_value_shapes() {
    let ir = single("record Item { num count = 1e1, string? note = null, }\npublic num? absent = null\npublic List<List<num>> values = [[1,2], []]\npublic url site = \"https://example.invalid\"\npublic path location = \"../inert\"\nItem item = {}\n").unwrap();
    ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &CancellationToken::new()).unwrap();
    assert!(
        ir.declarations
            .iter()
            .any(|decl| decl.signature == ProjectPublicSignature::Binding(ProjectPublicType::Url))
    );
    assert!(
        ir.declarations
            .iter()
            .any(|decl| decl.signature == ProjectPublicSignature::Binding(ProjectPublicType::Path))
    );
}

/// The portable oracle retains the expected complete/public membership counts.
#[test]
fn conformance_project_ir_oracle_membership() {
    let oracle = parse_fixture(include_str!("oracle.toml"));
    assert_eq!(
        oracle.root["modules"].parse::<usize>().unwrap(),
        complete().modules.len()
    );
}

/// Root descriptors are reviewed conformance data, not arbitrary acquisition instructions.
#[test]
fn conformance_project_ir_view_descriptors() {
    let ir = complete();
    let reader =
        ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &CancellationToken::new()).unwrap();
    for text in [
        include_str!("public-view.toml"),
        include_str!("private-view.toml"),
    ] {
        let fixture = parse_fixture(text);
        let request = ViewRequest {
            schema: PROJECT_VIEW_SCHEMA.to_owned(),
            roots: vec![symbol(
                &required_string(&fixture.root, "root_module"),
                &required_string(&fixture.root, "root_name"),
            )],
        };
        let outcome = reader.derive_view(&request, &CancellationToken::new());
        if required_string(&fixture.root, "outcome") == "public-view" {
            let view = outcome.unwrap();
            assert_eq!(
                view.exports()
                    .iter()
                    .map(|export| export.identity().declaration_name())
                    .collect::<Vec<_>>()
                    .join(","),
                required_string(&fixture.root, "expected_exports")
            );
        } else {
            assert_eq!(
                format!("{:?}", outcome.unwrap_err()),
                required_string(&fixture.root, "outcome")
            );
        }
    }
}

/// Every registered malformed artifact selector has an independently checked rejection.
#[test]
fn conformance_project_ir_malformed_descriptor() {
    let fixture = parse_fixture(include_str!("malformed.toml"));
    let ir = complete();
    for mutation in &fixture.arrays["mutations"] {
        let mut corrupt = ir.as_ref().clone();
        match required_string(mutation, "selector").as_str() {
            "resource-count" => corrupt.resources.value_nodes += 1,
            "missing-map" => {
                corrupt.source_maps.pop();
            }
            "out-of-range-map" => {
                corrupt.source_maps[0].location = SourceLocation::new(
                    corrupt.source_maps[0].location.source(),
                    ByteSpan::new(0, u64::MAX).unwrap(),
                );
            }
            "incompatible-value" => {
                corrupt
                    .declarations
                    .iter_mut()
                    .find(|decl| decl.value.is_some())
                    .unwrap()
                    .value = Some(ProjectValue::Bool(false));
            }
            "unknown-schema" => corrupt.schema = "unknown".to_owned(),
            unknown => panic!("unreviewed mutation {unknown}"),
        }
        let failure =
            ValidatedProject::from_ir(Arc::new(corrupt), ir.limits, &CancellationToken::new())
                .unwrap_err();
        assert_eq!(format!("{failure:?}"), required_string(mutation, "outcome"));
    }
}

/// Materialized private reuse still retains required public identity-reference targets.
#[test]
fn security_project_ir_inherited_references_keep_public_dependency_closure() {
    let ir = single(
        "public num seed = 1\nRef<num> hidden = ref(seed)\npublic Ref<num> shown = hidden\n",
    )
    .unwrap();
    let reader =
        ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &CancellationToken::new()).unwrap();
    let view = reader
        .derive_view(
            &ViewRequest {
                schema: PROJECT_VIEW_SCHEMA.to_owned(),
                roots: vec![symbol("example", "shown")],
            },
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(
        view.exports()
            .iter()
            .map(|export| export.identity().declaration_name())
            .collect::<Vec<_>>(),
        ["seed", "shown"]
    );
    assert_eq!(view.values().len(), 2);
}

/// Full locked vocabulary schemas are retained and views select only required public types.
#[test]
fn integration_project_ir_vocabulary_interpretive_closure() {
    let captured = capture_project(request_fixture(include_str!(
        "../public_semantics/fixtures/positive/vocabulary-multiple-alias.toml"
    )))
    .unwrap();
    let ir = compile_project(&captured, &CancellationToken::new()).unwrap();
    let reader =
        ValidatedProject::from_ir(Arc::clone(&ir), ir.limits, &CancellationToken::new()).unwrap();
    let view = reader
        .derive_view(
            &ViewRequest {
                schema: PROJECT_VIEW_SCHEMA.to_owned(),
                roots: vec![symbol("api", "first")],
            },
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(view.vocabulary_records().len(), 1);
    assert_eq!(view.vocabulary_records()[0].identity, "Alpha");
    assert_eq!(ir.resources.vocabulary_units, 2);
}

/// Capture order and host source IDs do not change complete logical project meaning.
#[test]
fn property_project_ir_capture_and_host_order_are_nonsemantic() {
    let captured = capture_project(request()).unwrap();
    let sources = captured
        .sources()
        .iter()
        .rev()
        .map(|source| {
            CapturedSourceInput::new(
                format!("other:{}", source.module_id()),
                source.module_id(),
                source.bytes().to_vec(),
            )
        })
        .collect();
    let reordered = capture_project(CapturedProjectRequest::new(
        neutral_compiler::CAPTURE_REQUEST_VERSION,
        captured.profile(),
        sources,
        Vec::new(),
        neutral_compiler::ProjectCaptureControls::new(captured.limits(), CancellationToken::new()),
    ))
    .unwrap();
    let first = compile_project(&captured, &CancellationToken::new()).unwrap();
    let second = compile_project(&reordered, &CancellationToken::new()).unwrap();
    assert!(first.logical_eq(&second));
    assert_ne!(first.sources, second.sources);
    let handles = (0..3)
        .map(|_| {
            let captured = captured.clone();
            std::thread::spawn(move || {
                compile_project(&captured, &CancellationToken::new()).unwrap()
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        assert_eq!(first, handle.join().unwrap());
    }
}
