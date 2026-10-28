// SPDX-License-Identifier: Apache-2.0

//! Stable checks execute the real fuzz support without libFuzzer or portable-plan files.

#[path = "../../../../fuzz/fuzz_targets/expression_support.rs"]
mod support;

/// A selected valid deferred seed must reach residual trees rather than scalar-only decoder rejection.
#[test]
fn fuzz_smoke_wire_seeds_reach_retained_expression_trees() {
    let bytes = support::mutated_seed(&[1]);
    let project = support::decode(&bytes).expect("registered seed decodes");
    assert!(project.ir().declarations.iter().any(|d| matches!(
        d.body,
        Some(neutral_ir::expression::ExpressionBody::Expression(_))
    )));
}

/// Every registered source is independently decodable and its formatter output remains compilable.
#[test]
fn fuzz_smoke_registered_expression_seeds_cover_source_wire_and_public_probe() {
    let cancel = neutral_core::CancellationToken::new();
    for (index, source) in support::SOURCE_SEEDS.iter().enumerate() {
        let captured = support::capture(source).expect("seed capture");
        let encoded = support::encode(&captured).expect("seed compiles and validates");
        let project = support::decode(&encoded).expect("seed independently decodes");
        support::inspect_public(&project);
        let selector = u8::try_from(index).expect("registered seeds fit the selector byte");
        assert_eq!(support::mutated_seed(&[selector]), encoded);
        assert_eq!(support::mutated_source(&[selector]), *source);
        let formatted =
            neutral_compiler::format_expression_source(&captured, support::SOURCE_MODULE, &cancel)
                .unwrap();
        let formatted = support::capture(&formatted).expect("formatted source captures");
        let formatted = support::decode(&support::encode(&formatted).unwrap()).unwrap();
        assert_eq!(project.ir().declarations, formatted.ir().declarations);
    }
    let bytes = support::mutated_seed(&[1]);
    let summary =
        support::probe(&bytes).expect("consumer seed reaches actual nominal entry discovery");
    assert!(summary.declarations > 0);
    assert!(support::probe(b"bad frame").is_none());
}

/// Offsets reach the final artifact section without disturbing unrelated bytes.
#[test]
fn fuzz_smoke_wire_mutations_reach_beyond_the_header() {
    let original = support::mutated_seed(&[1]);
    let offset = original.len() - 1;
    let mut input = vec![1];
    input.extend_from_slice(&offset.to_le_bytes());
    input.push(original[offset] ^ 1);
    let mutated = support::mutated_seed(&input);
    assert_eq!(&mutated[..offset], &original[..offset]);
    assert_eq!(mutated[offset], original[offset] ^ 1);
    let _ = support::decode(&mutated);
}

/// Explicit consumer requests reach selected/skipped demand, values and safe envelope failures.
#[test]
fn fuzz_smoke_consumer_seed_reaches_binding_and_availability_reports() {
    use neutral_ir::composition::BindingValue as V;
    use neutral_semantics::{BindingKey, EvaluationError, EvaluationResult as R};
    let project = support::decode(&support::mutated_seed(&[1])).unwrap();
    let run = |bytes: &[u8]| support::consumer_support::consume(&project, bytes);
    let skipped = run(&[0, 2, 1]).expect("failed output is not demanded on skipped branch");
    let R::Value {
        value: V::Record(fields),
        ..
    } = skipped.outcome
    else {
        panic!("complete plan value")
    };
    let value = fields
        .iter()
        .find(|(name, _)| name == "value")
        .unwrap()
        .1
        .as_ref()
        .unwrap();
    assert_eq!(
        value,
        &V::Number(neutral_ir::ExactNumber::from_source("0.125", 16, 16).unwrap())
    );
    assert!(matches!(
        run(&[1, 2, 0]).unwrap().outcome,
        R::Unavailable(BindingKey::Output { .. })
    ));
    assert!(matches!(
        run(&[1, 2, 1]).unwrap().outcome,
        R::Failure(EvaluationError::BindingFailed)
    ));
    assert!(matches!(run(&[1, 2, 2]).unwrap().outcome, R::Value { .. }));
    assert!(matches!(
        run(&[1, 2, 5]).unwrap().outcome,
        R::RequiredBindings(_)
    ));
    for invalid in [&[3, 2, 2][..], &[5, 2, 2], &[1, 2, 3], &[1, 2, 4]] {
        assert!(run(invalid).is_err(), "{invalid:?}");
    }
    assert!(
        run(&[1, 2, 2]).is_ok(),
        "bad requests do not poison a later generation"
    );
}
