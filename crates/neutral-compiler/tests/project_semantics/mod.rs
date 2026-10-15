// SPDX-License-Identifier: Apache-2.0

//! Semantic-project contract tests.

use super::*;
use crate::{
    CAPTURE_REQUEST_VERSION, CapturedCompositionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, CapturedVocabularyInput, ProjectCaptureControls,
    ProjectCaptureLimitValues, ProjectCaptureLimits, build_module_graph,
    capture_composition_project, capture_project,
};
use neutral_core::{
    CancellationToken, StructuralLimits, VocabularyContentDigest, profile::LanguageProfile,
};
use neutral_ir::composition::{ValueOriginKind, ValuePathSegment, project::CompositionAttribution};
use neutral_vocabulary::{VocabularyLimits, VocabularyLock, composition::CompositionLimits};
use std::fmt::Write;

/// Lexer failures retain the owning module, captured digest, and original byte span.
#[test]
fn root_parser_preserves_lexer_failure_location() {
    let bytes = b"num answer = \0";
    let digest = SourceContentDigest::from_bytes(bytes);
    let Err(failure) = parse_roots("api", digest, bytes) else {
        panic!("malformed source produced declaration roots");
    };
    assert_eq!(failure.diagnostics().len(), 1);
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code(), diagnostics::INACCESSIBLE_NAME);
    assert_eq!(diagnostic.module_id(), "api");
    assert_eq!(
        diagnostic.source_location(),
        SourceLocation::new(digest, ByteSpan::new(13, 14).unwrap())
    );
}

/// Malformed declarations and contextual shapes fail at the semantic grammar boundary.
#[test]
fn malformed_declaration_and_value_matrix_is_rejected() {
    for body in [
        "public",
        "record {}",
        "record Name []",
        "num answer",
        "num = 1",
        "num answer =",
        "num 7 = 1",
        "List num answer = []",
        "List<> answer = []",
        "List<num answer = []",
        "bool? ? answer = null",
        "url answer = 1",
        "path answer = true",
        "List<num> answer = [1 2]",
        "num answer = ref()",
        "num answer = ref(1)",
        "num answer = missing::",
        "num answer = [1,,2]",
        "record R { , }",
        "record R { num, }",
        "record R { public num count, }",
        "record R { num count =, }",
        "record R { num count = [1 2], }",
        "record R { num count extra, }",
        "record R { List<> items, }",
        "record R { num count, num count }",
        "record R { num count }\nR value = { count 1, }",
        "record R { num count }\nR value = { true: 1, }",
        "record R { num count }\nR value = { count: 1 }",
    ] {
        assert_eq!(
            first_code(&[("api", body)]),
            diagnostics::INVALID_SOURCE,
            "{body}"
        );
    }
}

/// A graph from different source bytes and cancellation never expose a semantic model.
#[test]
fn semantic_requests_reject_stale_graphs_and_cancellation() {
    let (captured, graph) = project(&[("api", "public num answer = 1")]);
    let (changed, _) = project(&[("api", "public num answer = 2")]);
    let failure =
        analyze_project_semantics(&changed, &graph, &CancellationToken::new()).unwrap_err();
    assert_eq!(failure.diagnostics()[0].code(), diagnostics::GRAPH_MISMATCH);
    let token = CancellationToken::new();
    token.cancel();
    let failure = analyze_project_semantics(&captured, &graph, &token).unwrap_err();
    assert_eq!(failure.diagnostics()[0].code(), diagnostics::CANCELLED);
}

/// Cancellation arriving during parsing is observed before another unit or resolution starts.
#[test]
fn semantic_cancellation_between_phases_never_publishes_partial_results() {
    for sources in [
        vec![("api", "public num answer = 1")],
        vec![
            ("api", "public num answer = 1"),
            ("other", "num hidden = 2"),
        ],
    ] {
        let (captured, graph) = project(&sources);
        let token = CancellationToken::new();
        let mut calls = 0;
        let result =
            analyze_project_semantics_with_parser(&captured, &graph, &token, &mut |source| {
                calls += 1;
                let roots = parse_roots(source.module_id(), source.digest(), source.bytes());
                token.cancel();
                roots
            });
        let Err(failure) = result else {
            panic!("cancelled request published a semantic model");
        };
        assert_eq!(calls, 1);
        assert_eq!(failure.diagnostics().len(), 1);
        assert_eq!(failure.diagnostics()[0].code(), diagnostics::CANCELLED);
        assert!(
            captured.sources().iter().any(
                |source| source.digest() == failure.diagnostics()[0].source_location().source()
            )
        );
    }
}

/// Declaration limits and duplicate bindings reject the complete request rather than truncating it.
#[test]
fn duplicate_roots_and_declaration_overflow_are_rejected() {
    assert_eq!(
        first_code(&[("api", "num value = 1\nnum value = 2")]),
        diagnostics::INACCESSIBLE_NAME
    );
    let mut body = String::new();
    for index in 0..65 {
        writeln!(body, "num item_{index} = {index}").unwrap();
    }
    assert_eq!(first_code(&[("api", &body)]), diagnostics::LIMIT_EXCEEDED);
}

/// Nominal types, reuse, and references cannot silently resolve to incompatible declaration kinds.
#[test]
fn semantic_kind_and_composite_type_mismatches_are_rejected() {
    for (body, expected) in [
        (
            "num value = 1\nvalue item = {}",
            diagnostics::INACCESSIBLE_NAME,
        ),
        (
            "record Item {}\nnum item = Item",
            diagnostics::INACCESSIBLE_NAME,
        ),
        (
            "record Item {}\nRef<Item> item = ref(Item)",
            diagnostics::INVALID_REFERENCE,
        ),
        (
            "num value = 1\nnum item = ref(value)",
            diagnostics::TYPE_MISMATCH,
        ),
        (
            "num value = 1\nRef<string> item = ref(value)",
            diagnostics::TYPE_MISMATCH,
        ),
        (
            "List<num> values = [1]\nList<string> items = values",
            diagnostics::TYPE_MISMATCH,
        ),
        (
            "num? value = null\nstring? item = value",
            diagnostics::TYPE_MISMATCH,
        ),
        (
            "url value = \"inert\"\npath item = value",
            diagnostics::INVALID_SOURCE,
        ),
        (
            "record Item {}\nrecord Other {}\nItem value = {}\nOther item = value",
            diagnostics::TYPE_MISMATCH,
        ),
        (
            "num value = missing::unknown",
            diagnostics::INACCESSIBLE_NAME,
        ),
    ] {
        assert_eq!(first_code(&[("api", body)]), expected, "{body}");
    }
}

/// Exact-number scale bounds apply during lowering even to compact source spellings.
#[test]
fn project_lowering_enforces_numeric_scale_bounds_at_original_source() {
    for spelling in ["1e4096", "1e-4096"] {
        let (captured, _) = project(&[("api", &format!("public num value = {spelling}"))]);
        let ir = crate::compile_project(&captured, &CancellationToken::new())
            .expect("exact scale boundary must compile");
        let Some(neutral_ir::project::ProjectValue::Number(number)) = &ir.declarations[0].value
        else {
            panic!("numeric declaration must retain its exact value");
        };
        assert_eq!(number.coefficient(), "1");
        assert_eq!(
            number.scale(),
            if spelling == "1e4096" { 4096 } else { -4096 }
        );
    }
    for spelling in ["1e4097", "1e-4097", "1e9223372036854775808"] {
        let (captured, _) = project(&[("api", &format!("public num value = {spelling}"))]);
        let crate::ProjectCompileFailure::Lowering {
            code,
            location: Some(location),
        } = crate::compile_project(&captured, &CancellationToken::new())
            .expect_err("over-limit exact number must not publish project IR")
        else {
            panic!("numeric bound must fail during contextual lowering");
        };
        assert_eq!(code, crate::project_lowering_diagnostics::INVALID_VALUE);
        let source = &captured.sources()[0];
        assert_eq!(location.source(), source.digest());
        assert_eq!(
            &source.bytes()[usize::try_from(location.span().start()).unwrap()
                ..usize::try_from(location.span().end()).unwrap()],
            spelling.as_bytes()
        );
    }
}

/// Public semantic accessors retain exact root categories and reference occurrence spans.
#[test]
fn semantic_public_facts_cover_record_and_binding_origins() {
    let (captured, graph) = project(&[(
        "api",
        "public record Item { num count = 1 }\npublic Item item = {}\npublic Ref<Item> pointer = ref(item)\n",
    )]);
    let model = analyze_project_semantics(&captured, &graph, &CancellationToken::new()).unwrap();
    for root in model.symbols() {
        assert!(root.is_public());
        assert_eq!(
            root.source_location().source(),
            captured.sources()[0].digest()
        );
        assert_eq!(
            root.kind(),
            if root.identity().declaration_name() == "Item" {
                ProjectSymbolKind::Record
            } else {
                ProjectSymbolKind::Binding
            }
        );
    }
    for edge in model.dependencies() {
        assert_eq!(
            edge.source_location().source(),
            captured.sources()[0].digest()
        );
        assert!(edge.source_location().span().start() < edge.source_location().span().end());
    }
}

/// Captures and graph-validates a small exact source closure.
pub(super) fn project(sources: &[(&str, &str)]) -> (CapturedProject, ModuleGraph) {
    let limits = ProjectCaptureLimitValues {
        total_source_bytes: 16_384,
        source_bytes_per_unit: 4096,
        source_units: 8,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 1,
        vocabulary_bytes_per_unit: 1024,
        total_vocabulary_bytes: 1024,
        imports_per_module: 8,
        import_edges: 16,
        scc_units: 8,
        declarations: 64,
        diagnostics: 16,
        output_bytes: 16_384,
    };
    let input = sources
        .iter()
        .enumerate()
        .map(|(index, (module, body))| {
            CapturedSourceInput::new(
                format!("source:{index}"),
                *module,
                format!("neu \"1.0\"\nmodule {module}\n{body}").into_bytes(),
            )
        })
        .collect();
    let captured = capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        input,
        Vec::new(),
        ProjectCaptureControls::new(ProjectCaptureLimits::new(limits), CancellationToken::new()),
    ))
    .expect("semantic fixture must capture");
    let graph = build_module_graph(&captured, &CancellationToken::new())
        .expect("semantic fixture graph must be valid");
    (captured, graph)
}

/// Returns the first semantic code for one rejected project.
fn first_code(sources: &[(&str, &str)]) -> &'static str {
    let (captured, graph) = project(sources);
    analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect_err("project must be rejected")
        .diagnostics()[0]
        .code()
}

/// Captures one source module using an exact vocabulary-owned default.
fn composition_project_with_default() -> crate::CapturedCompositionProject {
    let bytes = br#"{
  "format": "neutral-vocabulary-bundle",
  "encoding_version": "1.0",
  "schema_version": "2.0",
  "identity": "ExampleDomain",
  "version": "1.0.0",
  "required_features": ["vocabulary-composition-v2"],
  "dependencies": [],
  "types": [{
    "kind": "record",
    "name": "Item",
    "public": true,
    "fields": [{
      "name": "count",
      "type": { "kind": "num" },
      "presence": "defaulted",
      "restrictions": {},
      "default": { "kind": "num", "value": "3" }
    }]
  }]
}
"#
    .to_vec();
    let lock = VocabularyLock::new(
        "ExampleDomain",
        "1.0.0",
        neutral_vocabulary::composition::ENCODING_VERSION,
        neutral_ir::composition::profile::VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&bytes),
        vec![neutral_ir::composition::profile::VOCABULARY_COMPOSITION_FEATURE.to_owned()],
    )
    .unwrap();
    let source = b"neu \"1.0\"\nmodule example\nuse ExampleDomain as domain\ndomain::Item original = {}\ndomain::Item copied = original\n".to_vec();
    let capture_limits = ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 4096,
        source_units: 1,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 1,
        vocabulary_bytes_per_unit: 4096,
        total_vocabulary_bytes: 4096,
        imports_per_module: 4,
        import_edges: 4,
        scc_units: 1,
        declarations: 16,
        diagnostics: 16,
        output_bytes: 16_384,
    });
    capture_composition_project(CapturedCompositionProjectRequest::new(
        CapturedProjectRequest::new(
            neutral_ir::composition::profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "source:example",
                "example",
                source,
            )],
            vec![CapturedVocabularyInput::new(bytes, lock)],
            ProjectCaptureControls::new(capture_limits, CancellationToken::new()),
        ),
        neutral_ir::composition::profile::REQUIRED_FEATURES
            .iter()
            .map(|feature| (*feature).to_owned())
            .collect(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(65_536, 64).unwrap(),
        )),
    ))
    .expect("valid defaulted vocabulary request should capture")
}

/// Reusing a materialized vocabulary default does not invent a second default occurrence.
#[test]
fn reused_vocabulary_default_keeps_original_default_attribution() {
    let captured = composition_project_with_default();
    let ir = compile_composition_project(&captured, &CancellationToken::new())
        .expect("valid default reuse should compile");
    let path = [ValuePathSegment::Field("count".to_owned())];
    let default = ir
        .origins
        .iter()
        .find(|origin| {
            origin.binding.declaration_name() == "original"
                && origin.path == path
                && origin.kind == ValueOriginKind::Defaulted
        })
        .expect("source omission must retain the original default occurrence");
    assert!(matches!(
        &default.attribution,
        Some(CompositionAttribution::Vocabulary {
            identity,
            version,
            type_name,
            field_name,
            ..
        }) if identity == "ExampleDomain"
            && version == "1.0.0"
            && type_name == "Item"
            && field_name == "count"
    ));
    let reused = ir
        .origins
        .iter()
        .find(|origin| {
            origin.binding.declaration_name() == "copied"
                && origin.path.is_empty()
                && origin.kind == ValueOriginKind::Supplied
        })
        .expect("copied materialized binding root remains supplied");
    assert!(matches!(
        &reused.attribution,
        Some(CompositionAttribution::Source(_))
    ));
}

#[test]
/// Public nominal signatures cannot expose a private local type.
fn private_type_closure_is_rejected() {
    assert_eq!(
        first_code(&[(
            "api",
            "record Hidden {\n string name,\n}\npublic record Leaky {\n Hidden inner,\n}\n"
        )]),
        diagnostics::PRIVATE_PUBLIC_TYPE
    );
}

#[test]
/// A public identity reference cannot expose a private target.
fn private_ref_target_is_rejected() {
    assert_eq!(
        first_code(&[(
            "api",
            "num hidden = 7\npublic Ref<num> exposed = ref(hidden)\n"
        )]),
        diagnostics::PRIVATE_REFERENCE
    );
}

#[test]
/// Imported public reuse retains alias-independent module-symbol edges.
fn cross_module_reuse_has_stable_identity() {
    let (captured, graph) = project(&[
        (
            "service::consumer",
            "import service::source as source\npublic num copied = source::answer\n",
        ),
        (
            "service::source",
            "num seed = 42\npublic num answer = seed\n",
        ),
    ]);
    let model = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("valid reuse");
    let edge = model
        .dependencies()
        .iter()
        .find(|edge| edge.from().declaration_name() == "copied")
        .expect("cross-module edge");
    assert_eq!(edge.kind(), ProjectDependencyKind::Value);
    assert_eq!(edge.from().module().module_name(), "service::consumer");
    assert_eq!(edge.to().module().module_name(), "service::source");
    assert_eq!(edge.to().declaration_name(), "answer");
}

#[test]
/// Import SCCs remain valid graphs but ordinary value cycles are rejected.
fn cross_scc_value_cycle_is_rejected() {
    assert_eq!(
        first_code(&[
            (
                "cycle::left",
                "import cycle::right as right\npublic num value = right::value\n"
            ),
            (
                "cycle::right",
                "import cycle::left as left\npublic num value = left::value\n"
            ),
        ]),
        diagnostics::SEMANTIC_CYCLE
    );
}

#[test]
/// An import SCC with acyclic semantic dependencies remains valid.
fn import_scc_without_semantic_cycle_is_accepted() {
    let (captured, graph) = project(&[
        (
            "cycle::right",
            "import cycle::left as left\npublic num answer = left::seed\n",
        ),
        (
            "cycle::left",
            "import cycle::right as right\npublic num seed = 42\n",
        ),
    ]);
    let model = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("imports alone do not create a semantic cycle");
    assert_eq!(model.public_interface().exports().len(), 2);
}

#[test]
/// Multiple missing-name diagnostics follow module and original-byte order.
fn semantic_diagnostic_order_is_capture_invariant() {
    let bodies = [
        ("zeta", "num second = absent\nnum first = missing\n"),
        ("alpha", "num value = unknown\n"),
    ];
    let (first_capture, first_graph) = project(&bodies);
    let (second_capture, second_graph) = project(&[bodies[1], bodies[0]]);
    let failures = [
        analyze_project_semantics(&first_capture, &first_graph, &CancellationToken::new())
            .expect_err("missing names"),
        analyze_project_semantics(&second_capture, &second_graph, &CancellationToken::new())
            .expect_err("missing names after reorder"),
    ];
    for failure in failures {
        let ordered = failure
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.module_id().to_owned(),
                    diagnostic.source_location().span().start(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(ordered.len(), 3);
        assert_eq!(ordered[0].0, "alpha");
        assert!(ordered[1].1 < ordered[2].1);
        assert_eq!(ordered[1].0, "zeta");
        assert_eq!(ordered[2].0, "zeta");
    }
}

#[test]
/// Private imported names and absent imports have the same public failure.
fn private_imported_name_is_not_resolved() {
    assert_eq!(
        first_code(&[
            (
                "api::consumer",
                "import api::source as source\nnum value = source::hidden\n"
            ),
            ("api::source", "num hidden = 7\n"),
        ]),
        diagnostics::INACCESSIBLE_NAME
    );
}

#[test]
/// Identical record shapes in distinct modules do not share nominal identity.
fn cross_module_nominal_mismatch_is_rejected() {
    assert_eq!(
        first_code(&[
            (
                "types::alpha",
                "public record Shape {\n string label,\n}\npublic Shape item = { label: \"same\", }\n"
            ),
            (
                "types::beta",
                "import types::alpha as alpha\nrecord Shape {\n string label,\n}\nShape item = alpha::item\n"
            ),
        ]),
        diagnostics::TYPE_MISMATCH
    );
}

#[test]
/// Qualified type aliases compare by full owning module identity.
fn alias_spelling_does_not_change_nominal_type() {
    let (captured, graph) = project(&[
        (
            "types::alpha",
            "public record Shape {\n string label,\n}\npublic Shape item = { label: \"same\", }\n",
        ),
        (
            "types::beta",
            "import types::alpha as imported\nimported::Shape item = imported::item\n",
        ),
    ]);
    analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("same nominal identity");
}

#[test]
/// Reused imported types retain their identity through different module aliases.
fn reused_imported_nominal_type_has_stable_identity() {
    let (captured, graph) = project(&[
        (
            "types::alpha",
            "public record Shape {\n string label,\n}\npublic Shape item = { label: \"same\", }\n",
        ),
        (
            "types::beta",
            "import types::alpha as original\npublic original::Shape item = original::item\n",
        ),
        (
            "types::gamma",
            "import types::alpha as shape\nimport types::beta as reused\nshape::Shape item = reused::item\n",
        ),
    ]);
    analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("reused imported type has the same nominal identity");
}

#[test]
/// A public value cannot indirectly expose a private identity reference.
fn private_reference_reachable_through_reuse_is_rejected() {
    assert_eq!(
        first_code(&[(
            "api",
            "num hidden = 7\nRef<num> local = ref(hidden)\npublic Ref<num> exposed = local\n"
        )]),
        diagnostics::PRIVATE_REFERENCE
    );
}

#[test]
/// Ref type edges do not make embedded-record cycles.
fn reference_type_does_not_embed_its_target() {
    let (captured, graph) = project(&[("api", "public record Node {\n Ref<Node> next,\n}\n")]);
    analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect("identity-only type cycle");
}

#[test]
/// Invalid trailing tokens cannot produce an authoritative semantic model.
fn malformed_value_is_rejected_before_name_resolution() {
    assert_eq!(
        first_code(&[("api", "num broken = 1 2\n")]),
        diagnostics::INVALID_SOURCE
    );
}

#[test]
/// Duplicate public modifiers have their own stable failure category.
fn duplicate_public_modifier_is_rejected() {
    assert_eq!(
        first_code(&[("api", "public public num value = 1\n")]),
        diagnostics::INVALID_PUBLIC
    );
}

#[test]
/// A graph from another captured byte closure cannot be reused.
fn graph_from_different_capture_is_rejected() {
    let (captured, _) = project(&[("api", "num value = 1\n")]);
    let (_, wrong_graph) = project(&[("api", "num value = 2\n")]);
    let failure = analyze_project_semantics(&captured, &wrong_graph, &CancellationToken::new())
        .expect_err("foreign graph must fail closed");
    assert_eq!(failure.diagnostics()[0].code(), diagnostics::GRAPH_MISMATCH);
}

#[test]
/// Alias renaming changes source provenance but not target module-symbol keys.
fn import_alias_renaming_preserves_dependency_identity() {
    let source = ("shared", "public num answer = 42\n");
    let (first_capture, first_graph) = project(&[
        source,
        (
            "consumer",
            "import shared as first\nnum copy = first::answer\n",
        ),
    ]);
    let (second_capture, second_graph) = project(&[
        source,
        (
            "consumer",
            "import shared as second\nnum copy = second::answer\n",
        ),
    ]);
    let first = analyze_project_semantics(&first_capture, &first_graph, &CancellationToken::new())
        .expect("first alias resolves");
    let second =
        analyze_project_semantics(&second_capture, &second_graph, &CancellationToken::new())
            .expect("second alias resolves");
    let left = &first.dependencies()[0];
    let right = &second.dependencies()[0];
    assert_eq!(left.from(), right.from());
    assert_eq!(left.to(), right.to());
    assert_eq!(left.kind(), right.kind());
}

/// A digest-matched malformed vocabulary fails before source declaration analysis.
#[test]
fn invalid_vocabulary_preserves_semantic_failure_location() {
    let (fixture, _) = project(&[("api", "")]);
    let bytes = b"{}".to_vec();
    let lock = VocabularyLock::new(
        "ExampleDomain",
        "1.0.0",
        neutral_vocabulary::PROJECT_VOCABULARY_ENCODING_VERSION,
        neutral_vocabulary::PROJECT_VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&bytes),
        Vec::new(),
    )
    .unwrap();
    let captured = capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![CapturedSourceInput::new(
            "source:api",
            "api",
            b"neu \"1.0\"\nmodule api\nuse ExampleDomain as domain\n".to_vec(),
        )],
        vec![CapturedVocabularyInput::new(bytes, lock)],
        ProjectCaptureControls::new(fixture.limits(), CancellationToken::new()),
    ))
    .expect("capture checks exact vocabulary integrity before schema validation");
    let graph = build_module_graph(&captured, &CancellationToken::new()).unwrap();
    let failure = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect_err("invalid exact vocabulary must fail closed");
    assert_eq!(failure.diagnostics().len(), 1);
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code(), diagnostics::INVALID_VOCABULARY);
    assert_eq!(diagnostic.module_id(), "api");
    assert_eq!(
        diagnostic.source_location(),
        SourceLocation::new(captured.sources()[0].digest(), zero_span())
    );
}

/// Nullable wrappers count toward the public interface depth bound.
#[test]
fn public_interface_depth_overflow_preserves_semantic_failure_location() {
    let body = format!(
        "public record Item {{ {}num?{} values, }}",
        "List<".repeat(MAX_PROJECT_INTERFACE_TYPE_DEPTH),
        ">".repeat(MAX_PROJECT_INTERFACE_TYPE_DEPTH),
    );
    let (captured, graph) = project(&[("api", &body)]);
    let failure = analyze_project_semantics(&captured, &graph, &CancellationToken::new())
        .expect_err("public interface cannot publish an over-depth nullable type");
    assert_eq!(failure.diagnostics().len(), 1);
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code(), diagnostics::LIMIT_EXCEEDED);
    assert_eq!(diagnostic.module_id(), "api");
    assert_eq!(
        diagnostic.source_location(),
        SourceLocation::new(captured.sources()[0].digest(), zero_span())
    );
}

/// Attribution follows the original bytes through reuse, containers and local defaults.
#[test]
fn composition_attribution_preserves_nested_source_spans() {
    let policy = composition_project_with_default();
    let source = b"neu \"1.0\"\nmodule example\nrecord Item { num count = 17 }\nvariant Choice { List<Item> ready, string failed }\nChoice? original = { tag: \"ready\", payload: [{ count: 23 }, {}] }\nChoice? copied = original\n";
    let captured = capture_composition_project(CapturedCompositionProjectRequest::new(
        CapturedProjectRequest::new(
            neutral_ir::composition::profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "source:example",
                "example",
                source.to_vec(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(policy.limits(), CancellationToken::new()),
        ),
        neutral_ir::composition::profile::REQUIRED_FEATURES
            .iter()
            .map(|feature| (*feature).to_owned())
            .collect(),
        policy.composition_limits(),
    ))
    .unwrap();
    let ir = compile_composition_project(&captured, &CancellationToken::new()).unwrap();
    for (path, kind, expected) in [
        (
            vec![ValuePathSegment::Payload],
            ValueOriginKind::Supplied,
            "[{ count: 23 }, {}]",
        ),
        (
            vec![
                ValuePathSegment::Payload,
                ValuePathSegment::Element(0),
                ValuePathSegment::Field("count".to_owned()),
            ],
            ValueOriginKind::Supplied,
            "23",
        ),
        (
            vec![
                ValuePathSegment::Payload,
                ValuePathSegment::Element(1),
                ValuePathSegment::Field("count".to_owned()),
            ],
            ValueOriginKind::Defaulted,
            "17",
        ),
    ] {
        let origin = ir
            .origins
            .iter()
            .find(|origin| {
                origin.binding.declaration_name() == "original"
                    && origin.path == path
                    && origin.kind == kind
            })
            .expect("nested value retains its origin");
        let Some(CompositionAttribution::Source(location)) = origin.attribution else {
            panic!("source value must have a source attribution");
        };
        let span = location.span();
        assert_eq!(
            &source[usize::try_from(span.start()).unwrap()..usize::try_from(span.end()).unwrap()],
            expected.as_bytes()
        );
    }
    let copied = ir
        .origins
        .iter()
        .find(|origin| origin.binding.declaration_name() == "copied" && origin.path.is_empty())
        .expect("copied value retains its origin");
    let Some(CompositionAttribution::Source(location)) = copied.attribution else {
        panic!("copied source value must have source attribution");
    };
    let span = location.span();
    assert_eq!(
        &source[usize::try_from(span.start()).unwrap()..usize::try_from(span.end()).unwrap()],
        b"{ tag: \"ready\", payload: [{ count: 23 }, {}] }"
    );
}
