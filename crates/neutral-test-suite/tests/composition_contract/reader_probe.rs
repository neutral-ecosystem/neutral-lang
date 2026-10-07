// SPDX-License-Identifier: Apache-2.0

//! Successor independent consumer closure, precise attribution and real executable checks.

use super::*;
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{ValuePathSegment as P, project::CompositionAttribution as A},
};
use neutral_probe::composition::{inspect_composition_encoded, render_composition_summary_json};
use neutral_reader::composition::CompositionViewRequest;

/// Selects an exact reviewed runtime source owner, never an alias or host path.
fn owner(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(neutral_core::profile::V1_SOURCE_PROFILE, "example"),
        name,
    )
}
/// Creates a versioned post-validation root selection.
fn selection(names: &[&str]) -> CompositionViewRequest {
    CompositionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA.to_owned(),
        roots: names.iter().map(|n| owner(n)).collect(),
    }
}

/// Unselected variant alternatives retain interpretive reference types, while selected refs retain actual targets.
#[test]
fn integration_composition_view_closes_all_types_and_actual_references() {
    let case = source_case(
        "neu \"1.0\"\nmodule example\npublic record Node { num id }\npublic variant Choice { num ready, Ref<Node> linked }\npublic Node target = { id: 42 }\npublic Choice ready = { tag: \"ready\", payload: 1 }\npublic Choice linked = { tag: \"linked\", payload: ref(target) }\nnum secret = 7\npublic num visible = secret\n",
    );
    let project = compile(&case);
    let cancel = CancellationToken::new();
    let ready = project
        .derive_view(&selection(&["ready"]), &cancel)
        .unwrap();
    assert_eq!(
        ready
            .declarations()
            .iter()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["Choice", "Node", "ready"]
    );
    assert_eq!(ready.references().len(), 0);
    let linked = project
        .derive_view(&selection(&["linked"]), &cancel)
        .unwrap();
    assert!(
        linked
            .declarations()
            .iter()
            .any(|d| d.identity == owner("target"))
    );
    assert_eq!(linked.references()[0].1.target, owner("target"));
    let visible = project
        .derive_view(&selection(&["visible"]), &cancel)
        .unwrap();
    assert_eq!(visible.declarations().len(), 1);
    assert!(visible.origins().iter().all(|o| o.attribution.is_none()));
    assert!(!format!("{visible:?}").contains("secret"));
    for request in [
        selection(&["secret"]),
        selection(&["absent"]),
        selection(&["ready", "ready"]),
        CompositionViewRequest {
            schema: "unknown".to_owned(),
            roots: Vec::new(),
        },
    ] {
        assert!(project.derive_view(&request, &cancel).is_err());
    }
    let empty = project.derive_view(&selection(&[]), &cancel).unwrap();
    assert_eq!(empty.declarations().len(), 0);
    assert_eq!(empty.vocabularies().len(), 0);
    cancel.cancel();
    assert!(
        project
            .derive_view(&selection(&["ready"]), &cancel)
            .is_err()
    );
}

/// Cyclic identity references use iterative closure rather than recursive binding expansion.
#[test]
fn integration_composition_view_bounds_long_reference_closure() {
    let count = 96;
    let mut text =
        String::from("neu \"1.0\"\nmodule example\npublic record Node { Ref<Node> next }\n");
    for index in 0..count {
        writeln!(
            text,
            "public Node n{index} = {{ next: ref(n{}) }}",
            (index + 1) % count
        )
        .unwrap();
    }
    let case = source_case(&text);
    let mut controls = limits_for(&case);
    controls.work = profile::MAX_ITEMS;
    let captured = capture_composition_project(request_with_limits(&case, controls)).unwrap();
    let ir = compile_composition_project(&captured, &CancellationToken::new()).unwrap();
    let project = ValidatedCompositionProject::from_ir(
        Arc::clone(&ir),
        ir.limits,
        controls,
        &CancellationToken::new(),
    )
    .unwrap();
    let view = project
        .derive_view(&selection(&["n0"]), &CancellationToken::new())
        .unwrap();
    assert_eq!(view.declarations().len(), count + 1);
    assert_eq!(view.references().len(), count);
    let oversized = owner(&"x".repeat(1_000_001));
    assert!(
        project
            .derive_view(
                &CompositionViewRequest {
                    schema: profile::PROJECT_VIEW_SCHEMA.to_owned(),
                    roots: vec![oversized],
                },
                &CancellationToken::new()
            )
            .is_err()
    );
}

/// Source scalar/default/reuse spans retain original bytes; public projections never retain those spans.
#[test]
fn integration_composition_attribution_matches_source_subexpressions() {
    let text = include_str!("source-defaults.neu");
    let project = compile(&source_case(text));
    let ir = project.complete_ir();
    for binding in ["original", "copied"] {
        let origin = ir
            .origins
            .iter()
            .find(|o| {
                o.binding == owner(binding)
                    && o.path == [P::Field("inner".to_owned()), P::Field("count".to_owned())]
            })
            .unwrap();
        let Some(A::Source(location)) = origin.attribution else {
            panic!("source default must have real attribution");
        };
        assert_eq!(
            &text[usize::try_from(location.span().start()).unwrap()
                ..usize::try_from(location.span().end()).unwrap()],
            "42"
        );
    }
    let view = project
        .derive_view(&selection(&["states"]), &CancellationToken::new())
        .unwrap();
    assert!(
        view.origins()
            .iter()
            .all(|o| !matches!(o.attribution, Some(A::Source(_))))
    );
    let mut hostile = (**ir).clone();
    let origin = hostile
        .origins
        .iter_mut()
        .find(|o| matches!(o.attribution, Some(A::Source(_))))
        .unwrap();
    origin.attribution = Some(A::Source(neutral_core::SourceLocation::new(
        SourceContentDigest::from_bytes(b"not captured"),
        neutral_core::ByteSpan::new(0, 1).unwrap(),
    )));
    assert!(
        ValidatedCompositionProject::from_ir(
            Arc::new(hostile),
            ir.limits,
            limits_for(&source_case(text)),
            &CancellationToken::new()
        )
        .is_err()
    );
}

/// Canonical vocabulary owners survive views, but captured source IDs and bundle spans are redacted.
#[test]
fn integration_composition_probe_preserves_contracts_and_root_invariance() {
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    for case in fixture["cases"].as_array().unwrap().iter().filter(|c| {
        c["project_outcome"] == "accepted"
            && !c["capture"]["sources"].as_array().unwrap().is_empty()
    }) {
        let project = compile(case);
        let ir = project.complete_ir();
        assert!(
            ir.origins
                .iter()
                .filter(|o| o.kind == neutral_ir::composition::ValueOriginKind::Defaulted)
                .any(|o| matches!(o.attribution, Some(A::Vocabulary { .. })))
                || ir.vocabularies.is_empty()
                || ir
                    .origins
                    .iter()
                    .all(|o| o.kind != neutral_ir::composition::ValueOriginKind::Defaulted)
        );
        let bytes = encode_composition_project(&project, &CancellationToken::new()).unwrap();
        let all = inspect_composition_encoded(
            &bytes,
            DecodeLimits::hard(),
            ir.limits,
            limits_for(case),
            None,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(all.view.declarations().iter().all(|d| d.public));
        assert!(
            all.view
                .vocabularies()
                .iter()
                .flat_map(|b| &b.definitions)
                .all(|d| d.public)
        );
        assert!(all.view.origins().iter().all(|o| !matches!(
            o.attribution,
            Some(A::Source(_) | A::Vocabulary { span: Some(_), .. })
        )));
        let empty = inspect_composition_encoded(
            &bytes,
            DecodeLimits::hard(),
            ir.limits,
            limits_for(case),
            Some(&[]),
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(all.logical_identity, empty.logical_identity);
        assert_eq!(empty.view.declarations().len(), 0);
        let rendered = render_composition_summary_json(&all);
        assert!(serde_json::from_str::<Value>(&rendered).is_ok());
        for source in &ir.sources {
            assert!(!rendered.contains(&source.source_id));
        }
        assert_eq!(
            encode_composition_project(&project, &CancellationToken::new()).unwrap(),
            bytes
        );
        let mut hostile = (**ir).clone();
        if let Some(origin) = hostile
            .origins
            .iter_mut()
            .find(|o| matches!(o.attribution, Some(A::Vocabulary { .. })))
        {
            if let Some(A::Vocabulary { field_name, .. }) = &mut origin.attribution {
                *field_name = "unknown".to_owned();
            }
            assert!(
                ValidatedCompositionProject::from_ir(
                    Arc::new(hostile),
                    ir.limits,
                    limits_for(case),
                    &CancellationToken::new()
                )
                .is_err()
            );
        }
    }
}

/// A real standalone reader-only binary inspects successor bytes without compiler linkage or fallback.
#[test]
fn system_composition_standalone_probe_decodes_successor_artifacts() {
    let executable = crate::source_pipeline::executable("neutral-probe");
    let project = compile(&source_case(include_str!("source-defaults.neu")));
    let bytes = encode_composition_project(&project, &CancellationToken::new()).unwrap();
    let path = std::env::temp_dir().join(format!(
        "neutral-composition-probe-{}.nir",
        std::process::id()
    ));
    std::fs::write(&path, &bytes).unwrap();
    let output = std::process::Command::new(&executable)
        .args(["--json", "--root", "example::states"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema"], profile::PROJECT_IR_SCHEMA);
    assert_eq!(json["roots"], json!(["example::states"]));
    let private = std::process::Command::new(&executable)
        .args(["--json", "--root", "example::variant"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!private.status.success());
    assert_eq!(private.stdout, [] as [u8; 0]);
    assert!(!String::from_utf8_lossy(&private.stderr).contains("example::variant"));
    std::fs::write(&path, &bytes[..bytes.len() - 1]).unwrap();
    let corrupt = std::process::Command::new(&executable)
        .arg("--json")
        .arg(&path)
        .output()
        .unwrap();
    assert!(!corrupt.status.success());
    assert_eq!(corrupt.stdout, [] as [u8; 0]);
    std::fs::remove_file(&path).unwrap();
    let manifest = include_str!("../../../neutral-probe/Cargo.toml");
    assert!(!manifest.contains("neutral-compiler"));
}
