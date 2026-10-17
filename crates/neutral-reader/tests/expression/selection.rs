// SPDX-License-Identifier: Apache-2.0

//! Exact public selection and positive aggregate selection/projection controls.

use super::*;

/// Finds the inclusive cost boundary without assuming target-dependent wrapper sizes.
fn minimum(mut upper: u64, accepts: impl Fn(u64) -> bool) -> u64 {
    assert!(accepts(upper));
    let mut lower = 1;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        if accepts(middle) {
            upper = middle;
        } else {
            lower = middle + 1;
        }
    }
    upper
}

/// Explicit order is canonical, not request order; definitions and private candidates cannot enter.
#[test]
fn explicit_entry_order_and_private_rejections_preserve_accepted_selection() {
    let mut ir = two_entries_fixture();
    scalar(&mut ir, "m", "hidden", 150);
    ir.declarations.last_mut().unwrap().public = false;
    let project = authority(ir);
    let contract = T::Nominal(symbol("m", "Record"));
    let roots = [symbol("m", "b"), symbol("m", "a")];
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Explicit(&roots),
    };
    let cancel = CancellationToken::new();
    let select = |request: &EntryRequest<'_>| {
        project.entries(
            request,
            &project.ir().semantic_profiles,
            ExpressionViewLimits::default(),
            &cancel,
        )
    };
    let accepted = select(&request).unwrap();
    assert_eq!(
        accepted
            .bindings()
            .iter()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    for roots in [vec![symbol("m", "Record")], vec![symbol("m", "hidden")]] {
        assert_eq!(
            select(&EntryRequest {
                selection: EntrySelection::Explicit(&roots),
                ..request
            })
            .unwrap_err(),
            ExpressionEntryError::InvalidInput
        );
    }
    assert_eq!(
        accepted
            .derive_view(&cancel)
            .unwrap()
            .roots()
            .map(ModuleSymbolIdentity::declaration_name)
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(select(&request).unwrap().bindings().len(), 2);
}

/// A wildcard, subset, duplicate or unknown feature must never masquerade as a compatible tuple.
#[test]
fn selection_and_projection_reject_every_inexact_semantic_dimension() {
    let project = authority(two_entries_fixture());
    let roots = [symbol("m", "a")];
    let contract = T::Nominal(symbol("m", "Record"));
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Unique,
    };
    let cancel = CancellationToken::new();
    for fault in 0..10 {
        let mut profiles = SemanticProfiles::try_frozen().unwrap();
        match fault {
            0 => profiles.expression = "*".into(),
            1 => profiles.evaluator = "*".into(),
            2 => profiles.expansion = "*".into(),
            3 => profiles.decimal = "*".into(),
            4 => profiles.required_features.clear(),
            5 => {
                profiles.required_features.pop();
            }
            6 => profiles.required_features.push("unknown-feature".into()),
            7 => profiles
                .required_features
                .push(profiles.required_features[0].clone()),
            8 => profiles.required_features.reverse(),
            _ => profiles.expression = "neutral.expression/999".into(),
        }
        assert_eq!(
            project
                .entries(
                    &request,
                    &profiles,
                    ExpressionViewLimits::default(),
                    &cancel
                )
                .unwrap_err(),
            ExpressionEntryError::Unsupported,
            "fault {fault}"
        );
        assert_eq!(
            project
                .derive_view(
                    &ExpressionViewRequest {
                        schema: profile::PROJECT_VIEW_SCHEMA,
                        semantic_profiles: &profiles,
                        roots: &roots
                    },
                    &cancel
                )
                .unwrap_err(),
            E::Schema,
            "fault {fault}"
        );
    }
    let entries = project
        .entries(
            &EntryRequest {
                selection: EntrySelection::Explicit(&roots),
                ..request
            },
            &project.ir().semantic_profiles,
            ExpressionViewLimits::default(),
            &cancel,
        )
        .unwrap();
    assert_eq!(entries.bindings().len(), 1);
    assert_eq!(
        entries.derive_view(&cancel).unwrap().declarations().count(),
        2
    );
}

/// Every dimension applies to the combined request, rather than resetting after entry selection.
#[test]
fn selection_and_view_limits_are_positive_inclusive_and_shared() {
    type Control = fn(&mut ExpressionViewLimits) -> &mut u64;
    let controls: &[(&str, Option<u64>, Control)] = &[
        ("roots", Some(2), |p| &mut p.roots),
        ("closure edges", Some(4), |p| &mut p.closure_edges),
        ("work", None, |p| &mut p.work),
        ("output bytes", None, |p| &mut p.output_bytes),
    ];
    let project = authority(two_entries_fixture());
    let contract = T::Nominal(symbol("m", "Record"));
    let roots = [symbol("m", "b"), symbol("m", "a")];
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Explicit(&roots),
    };
    let view_request = ExpressionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA,
        semantic_profiles: &project.ir().semantic_profiles,
        roots: &roots,
    };
    let cancel = CancellationToken::new();
    let run = |policy| {
        project
            .entries(&request, &project.ir().semantic_profiles, policy, &cancel)?
            .derive_view(&cancel)
    };
    for &(name, exact, select) in controls {
        let mut policy = ExpressionViewLimits::default();
        let boundary = minimum(*select(&mut policy), |amount| {
            let mut policy = ExpressionViewLimits::default();
            *select(&mut policy) = amount;
            run(policy).is_ok()
        });
        assert!(boundary > 1, "{name} must constrain positive retained work");
        if let Some(exact) = exact {
            assert_eq!(boundary, exact, "{name}");
        }
        *select(&mut policy) = boundary;
        let accepted = run(policy).unwrap();
        *select(&mut policy) -= 1;
        assert_eq!(
            run(policy).unwrap_err(),
            ExpressionEntryError::Limit,
            "{name}"
        );
        *select(&mut policy) = boundary;
        assert_eq!(run(policy).unwrap().declarations().count(), 3);
        assert_eq!(
            accepted
                .roots()
                .map(ModuleSymbolIdentity::declaration_name)
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        if exact.is_none() {
            let direct = minimum(*select(&mut ExpressionViewLimits::default()), |amount| {
                let mut policy = ExpressionViewLimits::default();
                *select(&mut policy) = amount;
                project
                    .derive_view_with_limits(&view_request, policy, &cancel)
                    .is_ok()
            });
            assert!(
                boundary > direct,
                "{name}: selection must not reset the view budget"
            );
        }
    }
}

/// Private source/reuse coordinates are available in complete authority, but never in public edges/origins.
#[test]
fn public_reuse_projection_redacts_retained_private_evidence() {
    let mut ir = empty();
    scalar(&mut ir, "m", "hidden", 10);
    ir.declarations[0].public = false;
    scalar(&mut ir, "m", "exposed", 30);
    reuse(&mut ir, "exposed", "hidden", 31);
    ir.origins[1].attribution = Some(CompositionAttribution::Source(loc("m", 11, 12)));
    let project = authority(ir);
    assert_eq!(project.ir().provenance.len(), 1);
    assert!(
        project
            .ir()
            .origins
            .iter()
            .any(|o| o.binding == symbol("m", "exposed")
                && o.attribution == Some(CompositionAttribution::Source(loc("m", 11, 12))))
    );
    let roots = [symbol("m", "exposed")];
    let cancel = CancellationToken::new();
    let view = project
        .derive_view(
            &ExpressionViewRequest {
                schema: profile::PROJECT_VIEW_SCHEMA,
                semantic_profiles: &project.ir().semantic_profiles,
                roots: &roots,
            },
            &cancel,
        )
        .unwrap();
    let declarations = view.declarations().collect::<Vec<_>>();
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].identity, symbol("m", "exposed"));
    assert_eq!(declarations[0].body, Some(ExpressionBody::Value(one())));
    assert_eq!(
        view.origins()
            .map(|o| (o.owner, o.path, o.kind))
            .collect::<Vec<_>>(),
        [(&roots[0], [].as_slice(), ValueOriginKind::Supplied)]
    );
    assert_eq!(view.structural_edges().count(), 0);
    assert_eq!(view.expression_attributions().count(), 0);
    assert_eq!(
        view.dependencies(
            &symbol("m", "hidden"),
            ExpressionInspectionLimits::default(),
            &cancel
        )
        .unwrap_err(),
        E::Semantic
    );
}

/// Entry selection and its projection share one atomic request across every observed allocation/cancellation.
#[test]
fn selection_faults_and_cancellation_preserve_prior_public_authority() {
    use neutral_core::allocation::testing;
    let project = authority(two_entries_fixture());
    let contract = T::Nominal(symbol("m", "Record"));
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Enumerate,
    };
    let run = |cancel: &CancellationToken| {
        project
            .entries(
                &request,
                &project.ir().semantic_profiles,
                ExpressionViewLimits::default(),
                cancel,
            )?
            .derive_view(cancel)
    };
    let cancel = CancellationToken::new();
    let (accepted, checkpoints) = testing::observe(None, || run(&cancel));
    let accepted = accepted.unwrap();
    assert!(checkpoints > 0);
    for index in 0..checkpoints {
        let (failure, observed) = testing::observe(Some(index), || run(&cancel));
        assert!(observed > index);
        assert_eq!(
            failure.unwrap_err(),
            ExpressionEntryError::Limit,
            "allocation {index}"
        );
        let signal = CancellationToken::new();
        let (failure, observed) = testing::observe_cancellation(index, &signal, || run(&signal));
        assert!(observed > index && signal.is_cancelled());
        assert_eq!(
            failure.unwrap_err(),
            ExpressionEntryError::Cancelled,
            "cancellation {index}"
        );
        assert_eq!(accepted.declarations().count(), 3);
        assert_eq!(run(&cancel).unwrap().declarations().count(), 3);
    }
}

/// Inspection accounts for owned guard/path bytes independently of the number of returned edges.
#[test]
fn dependency_positive_limits_fail_one_over_and_recover() {
    type Control = fn(&mut ExpressionInspectionLimits) -> &mut u64;
    let controls: &[(&str, Control)] = &[
        ("edges", |p| &mut p.edges),
        ("work", |p| &mut p.work),
        ("output bytes", |p| &mut p.output_bytes),
    ];
    let project = guarded_project(BinaryOperator::And);
    let owner = symbol("m", "result");
    let cancel = CancellationToken::new();
    for &(name, select) in controls {
        let mut policy = ExpressionInspectionLimits::default();
        let boundary = minimum(*select(&mut policy), |amount| {
            let mut policy = ExpressionInspectionLimits::default();
            *select(&mut policy) = amount;
            project.dependencies(&owner, policy, &cancel).is_ok()
        });
        assert!(boundary > 1, "{name}");
        *select(&mut policy) = boundary;
        let accepted = project.dependencies(&owner, policy, &cancel).unwrap();
        *select(&mut policy) -= 1;
        assert_eq!(
            project.dependencies(&owner, policy, &cancel).unwrap_err(),
            E::Limit,
            "{name}"
        );
        *select(&mut policy) = boundary;
        assert_eq!(
            project.dependencies(&owner, policy, &cancel).unwrap(),
            accepted
        );
    }
}
