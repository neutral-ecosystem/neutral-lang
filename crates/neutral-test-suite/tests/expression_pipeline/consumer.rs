// SPDX-License-Identifier: Apache-2.0

//! Literal consumer reports over source-produced, independently decoded artifacts.

use super::*;
use neutral_ir::{expression::SelectorSegment, project_interface::ProjectPublicType as T};
use neutral_reader::consumer::{
    ConsumerContext, ConsumerLimits, ConsumerProtocol,
    expression::{ExpressionConsumerBoundary, ExpressionConsumerPolicy},
};
use neutral_semantics::{
    Binding, BindingKey, BindingState, EvaluationError, EvaluationRequest, EvaluationResult as R,
    OutputAssociation,
};

/// Registers explicit public result associations, not an executor or backend callback.
fn boundary<'p, 'a>(
    project: &'p ValidatedExpressionProject,
    report: &'a T,
    associations: &'a [OutputAssociation<'a>],
) -> ExpressionConsumerBoundary<'p, 'a> {
    ExpressionConsumerBoundary::new(
        project,
        ExpressionConsumerPolicy {
            protocol: ConsumerProtocol {
                report_contract: report,
                supported_features: profile::REQUIRED_FEATURES,
                diagnostic_codes: &["APP-001"],
            },
            associations,
        },
        ConsumerLimits::default(),
        &CancellationToken::new(),
    )
    .unwrap()
}

/// Expected ordinary data retains prerequisites as values, not implicit demanded output bindings.
fn plan_value(number: &str) -> V {
    V::Record(vec![
        (
            "prerequisites".into(),
            Some(V::List(vec![V::Reference(symbol("producer"))])),
        ),
        (
            "value".into(),
            Some(V::Number(
                ExactNumber::from_source(number, 128, 128).unwrap(),
            )),
        ),
    ])
}

/// Report context is opaque consumer data and never rewrites the immutable artifact.
fn context(revision: &str) -> ConsumerContext<'_> {
    ConsumerContext {
        adapter_revision: "adapter-1",
        target_revision: "target-1",
        input_revision: revision,
    }
}

/// Missing guards, missing selected outputs and skipped failed outputs have distinct literal results.
#[test]
fn integration_expression_reports_preserve_selected_demand_and_generation_identity() {
    let (_, project) = pipeline(include_bytes!("fixtures/guarded.neu"));
    let cancel = CancellationToken::new();
    let (entry, enabled, producer) = (symbol("entry"), symbol("enabled"), symbol("producer"));
    let result = T::Nominal(symbol("Result"));
    let report = T::Nominal(symbol("Report"));
    let plan = T::Nominal(symbol("Plan"));
    let associations = [OutputAssociation {
        target: &producer,
        result_type: &result,
    }];
    let boundary = boundary(&project, &report, &associations);
    let selector = [
        SelectorSegment::Field("values".into()),
        SelectorSegment::Index(1),
    ];
    let output = BindingKey::Output {
        target: &producer,
        result_type: &result,
        selector: &selector,
    };
    let enabled_key = BindingKey::Input(&enabled);
    let on = V::Bool(true);
    let off = V::Bool(false);
    for (guard, state, expected) in [
        (None, None, R::RequiredBindings(vec![enabled_key])),
        (Some(&on), None, R::RequiredBindings(vec![output])),
        (
            Some(&off),
            None,
            R::Value {
                ty: &plan,
                value: plan_value("0.125"),
            },
        ),
        (
            Some(&off),
            Some(BindingState::Failed),
            R::Value {
                ty: &plan,
                value: plan_value("0.125"),
            },
        ),
        (
            Some(&on),
            Some(BindingState::Unavailable),
            R::Unavailable(output),
        ),
        (
            Some(&on),
            Some(BindingState::Failed),
            R::Failure(EvaluationError::BindingFailed),
        ),
    ] {
        let mut bindings = Vec::new();
        if let Some(guard) = guard {
            bindings.push(Binding {
                key: enabled_key,
                ty: &T::Bool,
                state: BindingState::Value(guard),
            });
        }
        if let Some(state) = state {
            bindings.push(Binding {
                key: output,
                ty: &T::Num,
                state,
            });
        }
        let request = EvaluationRequest {
            selected: &entry,
            profiles: &project.ir().semantic_profiles,
            bindings: &bindings,
            associations: &associations,
            limits: ExpressionLimits::hard(),
        };
        boundary.validate_bindings(&request, &cancel).unwrap();
        let report = boundary
            .evaluate(&request, context("generation-1"), &cancel)
            .unwrap();
        assert_eq!(report.outcome, expected);
        assert_eq!(report.entry, &entry);
        assert_eq!(report.project_identity, boundary.project_identity());
        assert_eq!(report.context.input_revision, "generation-1");
        let changed = boundary
            .evaluate(&request, context("generation-2"), &cancel)
            .unwrap();
        assert_eq!(changed.project_identity, report.project_identity);
        assert_eq!(changed.outcome, report.outcome);
    }
}

/// Actual output roots satisfy selector availability; malformed skipped bindings still reject preflight.
#[test]
fn integration_expression_output_roots_validate_before_demand_and_recover() {
    let (_, project) = pipeline(include_bytes!("fixtures/guarded.neu"));
    let cancel = CancellationToken::new();
    let (entry, enabled, producer) = (symbol("entry"), symbol("enabled"), symbol("producer"));
    let result = T::Nominal(symbol("Result"));
    let report = T::Nominal(symbol("Report"));
    let associations = [OutputAssociation {
        target: &producer,
        result_type: &result,
    }];
    let boundary = boundary(&project, &report, &associations);
    let selector = [
        SelectorSegment::Field("values".into()),
        SelectorSegment::Index(1),
    ];
    let output = BindingKey::Output {
        target: &producer,
        result_type: &result,
        selector: &selector,
    };
    let off = V::Bool(false);
    let invalid = V::Record(vec![("values".into(), Some(V::List(vec![])))]);
    let good = V::Record(vec![(
        "values".into(),
        Some(V::List(vec![
            V::Number(ExactNumber::from_source("1", 128, 128).unwrap()),
            V::Number(ExactNumber::from_source("2", 128, 128).unwrap()),
        ])),
    )]);
    let on = V::Bool(true);
    for (guard, root, accepted) in [
        (&off, &invalid, false),
        (&on, &invalid, false),
        (&on, &good, true),
    ] {
        let bindings = [
            Binding {
                key: BindingKey::Input(&enabled),
                ty: &T::Bool,
                state: BindingState::Value(guard),
            },
            Binding {
                key: output,
                ty: &T::Num,
                state: BindingState::OutputResult(root),
            },
        ];
        let request = EvaluationRequest {
            selected: &entry,
            profiles: &project.ir().semantic_profiles,
            bindings: &bindings,
            associations: &associations,
            limits: ExpressionLimits::hard(),
        };
        let outcome = boundary.evaluate(&request, context("generation"), &cancel);
        if accepted {
            assert_eq!(
                outcome.unwrap().outcome,
                R::Value {
                    ty: &T::Nominal(symbol("Plan")),
                    value: plan_value("2")
                }
            );
        } else {
            assert!(outcome.is_err(), "even skipped malformed roots must reject");
        }
    }
}

/// Captures the file-backed domain under one exact content lock and crosses the independent wire boundary.
fn vocabulary_project() -> ValidatedExpressionProject {
    use neutral_core::VocabularyContentDigest;
    use neutral_vocabulary::{VocabularyLock, composition};
    let bytes = include_bytes!("fixtures/domain.json");
    let lock = VocabularyLock::new(
        "PipelineDomain",
        "1.0.0",
        composition::ENCODING_VERSION,
        composition::SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        vec![composition::REQUIRED_FEATURE.to_owned()],
    )
    .unwrap();
    let captured = capture_with_vocabularies(
        include_bytes!("fixtures/vocabulary.neu"),
        vec![neutral_compiler::CapturedVocabularyInput::new(
            bytes.to_vec(),
            lock,
        )],
    );
    pipeline_from_capture(&captured).1
}

/// Vocabulary restrictions remain obligations over the wire and are enforced only on demanded values.
#[test]
fn integration_expression_vocabulary_restrictions_survive_wire_and_binding_revisions() {
    let project = vocabulary_project();
    let cancel = CancellationToken::new();
    let ty = |name: &str| T::VocabularyNominal {
        identity: "PipelineDomain".into(),
        version: "1.0.0".into(),
        name: name.into(),
    };
    let report = ty("Report");
    let result = ty("Result");
    let plan = ty("Plan");
    let (entry, enabled, producer) = (symbol("entry"), symbol("enabled"), symbol("producer"));
    let associations = [OutputAssociation {
        target: &producer,
        result_type: &result,
    }];
    let boundary = boundary(&project, &report, &associations);
    assert_eq!(project.model().restriction_obligations().len(), 1);
    let roots = [entry.clone()];
    let view = project
        .derive_view_with_limits(
            &ExpressionViewRequest {
                schema: profile::PROJECT_VIEW_SCHEMA,
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            ExpressionViewLimits::default(),
            &cancel,
        )
        .unwrap();
    assert_eq!(view.restriction_obligations().count(), 1);
    let selector = [
        SelectorSegment::Field("values".into()),
        SelectorSegment::Index(1),
    ];
    let output = BindingKey::Output {
        target: &producer,
        result_type: &result,
        selector: &selector,
    };
    for (enabled_value, number, accepted) in
        [(true, "2", true), (true, "4", false), (false, "4", true)]
    {
        let guard = V::Bool(enabled_value);
        let number_value = V::Number(ExactNumber::from_source(number, 128, 128).unwrap());
        let root = V::Record(vec![(
            "values".into(),
            Some(V::List(vec![number_value.clone(), number_value])),
        )]);
        let bindings = [
            Binding {
                key: BindingKey::Input(&enabled),
                ty: &T::Bool,
                state: BindingState::Value(&guard),
            },
            Binding {
                key: output,
                ty: &T::Num,
                state: BindingState::OutputResult(&root),
            },
        ];
        let request = EvaluationRequest {
            selected: &entry,
            profiles: &project.ir().semantic_profiles,
            bindings: &bindings,
            associations: &associations,
            limits: ExpressionLimits::hard(),
        };
        let report = boundary
            .evaluate(&request, context(number), &cancel)
            .unwrap();
        assert_eq!(report.project_identity, boundary.project_identity());
        if accepted {
            let expected = if enabled_value { number } else { "0.125" };
            assert_eq!(
                report.outcome,
                R::Value {
                    ty: &plan,
                    value: V::Record(vec![(
                        "value".into(),
                        Some(V::Number(
                            ExactNumber::from_source(expected, 128, 128).unwrap(),
                        ))
                    )])
                }
            );
        } else {
            assert_eq!(report.outcome, R::Failure(EvaluationError::Restriction));
        }
    }
}
