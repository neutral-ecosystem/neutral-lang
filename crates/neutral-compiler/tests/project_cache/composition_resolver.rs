// SPDX-License-Identifier: Apache-2.0

//! Resolver diagnostics and bounded copies at their private semantic boundaries.

use super::super::{CapturedCompositionProject, parse};
use super::*;
use crate::{
    CapturedCompositionProjectRequest, CapturedProjectRequest, CapturedSourceInput,
    ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_composition_project,
};
use neutral_core::{StructuralLimits, profile::LanguageProfile};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

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

fn resolver<'a>(
    captured: &'a CapturedCompositionProject,
    cancel: &'a CancellationToken,
) -> Resolver<'a> {
    let mut resolver = Resolver {
        captured,
        graph: graph::build(captured, cancel).unwrap(),
        roots: BTreeMap::new(),
        types: BTreeMap::new(),
        definitions: Vec::new(),
        values: BTreeMap::new(),
        active: super::super::BTreeSet::new(),
        provenance: Vec::new(),
        work: 0,
        cancel,
    };
    for source in captured.sources() {
        for root in parse(source, captured, cancel).unwrap() {
            resolver.roots.insert(root.owner.clone(), root).unwrap();
        }
    }
    let roots = resolver.retained_roots().unwrap();
    resolver.signatures(&roots).unwrap();
    resolver
}

#[test]
fn missing_import_alias_reports_the_original_occurrence() {
    let captured = capture("public num answer = 42");
    let cancel = CancellationToken::new();
    let mut resolver = resolver(&captured, &cancel);
    let root = resolver.retained_roots().unwrap().pop().unwrap();
    let span = root.tokens[0].span;
    assert_eq!(
        resolver.name(&root, Some("missing"), "answer", span, false),
        Err(fail(
            codes::INCOMPATIBLE_VALUE,
            Some(SourceLocation::new(root.location.source(), span))
        )),
    );
}

#[test]
fn unknown_record_field_reports_the_object_location() {
    let captured =
        capture("public record Item { num amount }\npublic Item answer = { amount: 1, extra: 2 }");
    let cancel = CancellationToken::new();
    let mut resolver = resolver(&captured, &cancel);
    let root = resolver
        .retained_roots()
        .unwrap()
        .into_iter()
        .find(|root| root.ty.is_some())
        .unwrap();
    let ty = resolver.types[&root.owner].clone();
    assert_eq!(
        resolver.record_value(&root, &ty, &root.tokens, false, 0),
        Err(fail(
            codes::INCOMPATIBLE_VALUE,
            Some(SourceLocation::new(
                root.location.source(),
                root.tokens[0].span
            ))
        )),
    );
}

#[test]
fn overflowing_work_preserves_the_previous_total_and_diagnostic() {
    let captured = capture("public num answer = 42");
    let cancel = CancellationToken::new();
    let mut resolver = resolver(&captured, &cancel);
    let location = resolver.retained_roots().unwrap()[0].location;
    resolver.work = u64::MAX;
    assert_eq!(
        resolver.step(1, Some(location)),
        Err(fail(codes::LIMIT, Some(location)))
    );
    assert_eq!(resolver.work, u64::MAX);
}

#[test]
fn clone_work_counts_nested_defaults_choices_and_numeric_bounds() {
    let cancel = CancellationToken::new();
    let number = ExactNumber::from_source("12", 16, 16).unwrap();
    let value: V = V::Record(vec![
        ("missing".to_owned(), None),
        (
            "present".to_owned(),
            Some(V::List(vec![V::Number(number.clone()), V::Bool(true)])),
        ),
    ]);
    assert_eq!(value_clone_work(&value, &cancel, symbol_work).unwrap(), 24);
    let body = B::Record(vec![CompositionField {
        name: "amount".to_owned(),
        ty: T::Nullable(Box::new(T::List(Box::new(T::Num)))),
        presence: FieldPresence::Defaulted,
        restrictions: FieldRestrictions {
            minimum: Some(number.clone()),
            maximum: Some(number.clone()),
            choices: Some(vec![CompositionValue::Number(number.clone())]),
            ..FieldRestrictions::default()
        },
        default: Some(CompositionValue::Number(number)),
    }]);
    assert_eq!(body_clone_work(&body, &cancel).unwrap(), 21);
    cancel.cancel();
    assert_eq!(
        body_clone_work(&body, &cancel),
        Err(fail(codes::CANCELLED, None))
    );
}
