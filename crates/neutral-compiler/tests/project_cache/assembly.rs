// SPDX-License-Identifier: Apache-2.0

//! Assembly rejects incomplete checked inputs at the original declaration.

use super::super::{
    BTreeMap, BTreeSet, CancellationToken, graph, parse, validate_composition_bindings,
    validate_composition_scope,
};
use super::*;
use crate::{
    CapturedCompositionProject, CapturedCompositionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_composition_project,
};
use neutral_core::{StructuralLimits, profile::LanguageProfile};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

/// Captures source independently of the incomplete semantic inputs exercised below.
fn capture(body: &str) -> CapturedCompositionProject {
    let bounds = ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 4096,
        source_units: 4,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 4,
        vocabulary_bytes_per_unit: 4096,
        total_vocabulary_bytes: 4096,
        imports_per_module: 4,
        import_edges: 4,
        scc_units: 4,
        declarations: 16,
        diagnostics: 16,
        output_bytes: 16_384,
    };
    capture_composition_project(CapturedCompositionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "unit",
                "example",
                format!("neu \"1.0\"\nmodule example\n{body}\n").into_bytes(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(bounds),
                CancellationToken::new(),
            ),
        ),
        profile::REQUIRED_FEATURES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(4096, 16).unwrap(),
        )),
    ))
    .unwrap()
}

/// Neither a missing materialized binding nor a missing nominal definition can publish declarations.
#[test]
fn assembly_rejects_missing_checked_declarations_with_source_location() {
    for body in ["public num answer = 42", "public record Item { num value }"] {
        let captured = capture(body);
        let cancel = CancellationToken::new();
        let mut resolver = Resolver {
            captured: &captured,
            graph: graph::build(&captured, &cancel).unwrap(),
            roots: BTreeMap::new(),
            types: BTreeMap::new(),
            definitions: Vec::new(),
            values: BTreeMap::new(),
            active: BTreeSet::new(),
            provenance: Vec::new(),
            work: 0,
            cancel: &cancel,
        };
        let roots = parse(&captured.sources()[0], &captured, &cancel).unwrap();
        assert_eq!(roots.len(), 1);
        let location = roots[0].location;
        for root in roots {
            resolver
                .roots
                .insert(root.owner.try_clone().unwrap(), root)
                .unwrap();
        }
        let scope = Arc::try_new(
            validate_composition_scope(
                captured.catalogue(),
                Vec::new(),
                captured.composition_limits(),
                &cancel,
            )
            .unwrap(),
        )
        .unwrap();
        let bindings = validate_composition_bindings(
            scope.clone(),
            &[],
            captured.composition_limits(),
            &cancel,
        )
        .unwrap();
        let Err(failure) = declarations(&mut resolver, &scope, &bindings) else {
            panic!("assembly accepted missing checked declaration: {body}");
        };
        assert_eq!(failure.code, codes::INCOMPATIBLE_VALUE, "{body}");
        assert_eq!(failure.location, Some(location), "{body}");
        assert_eq!(resolver.provenance, Vec::new());
    }
}
