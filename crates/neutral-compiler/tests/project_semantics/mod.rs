// SPDX-License-Identifier: Apache-2.0

//! Stage 4 semantic-project contract tests.

use super::*;
use crate::{
    CAPTURE_REQUEST_VERSION, CapturedProjectRequest, CapturedSourceInput, ProjectCaptureControls,
    ProjectCaptureLimitValues, ProjectCaptureLimits, build_module_graph, capture_project,
};
use neutral_core::profile::LanguageProfile;

/// Captures and graph-validates a small exact source closure.
fn project(sources: &[(&str, &str)]) -> (CapturedProject, ModuleGraph) {
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
