// SPDX-License-Identifier: Apache-2.0

//! Literal semantic expectations for accepted source programs after independent reading.

use super::*;
use neutral_ir::composition::BindingValue as V;

#[test]
fn integration_scalars_keep_meaning_when_source_order_and_number_spelling_change() {
    for source in [
        source!("positive", "scalars", "api"),
        source!("positive", "scalars-reordered", "api"),
    ] {
        let (_, _, project) = pipeline(&[source]);
        assert_eq!(materialized(&project, "api", "fraction"), &number("-125"));
        assert_eq!(materialized(&project, "api", "exponent"), &number("42"));
        assert_eq!(materialized(&project, "api", "enabled"), &V::Bool(true));
        assert_eq!(materialized(&project, "api", "disabled"), &V::Bool(false));
        assert_eq!(
            materialized(&project, "api", "label"),
            &V::String("neutral".into())
        );
    }
}

#[test]
fn integration_forward_reuse_materializes_the_literal_value() {
    let (_, _, project) = pipeline(&[source!("positive", "forward-reuse", "api")]);
    assert_eq!(materialized(&project, "api", "final_value"), &number("42"));
}

#[test]
fn integration_cli_nested_reuse_materializes_the_copied_record_value() {
    let (_, _, project) = pipeline(&[source!("cli", "nested-reuse", "service")]);
    let expected = V::Record(vec![(
        "item".into(),
        Some(V::Record(vec![("id".into(), Some(number("1")))])),
    )]);
    assert_eq!(materialized(&project, "service", "original"), &expected);
    assert_eq!(materialized(&project, "service", "copied"), &expected);
}

#[test]
fn integration_nullable_nested_lists_and_empty_lists_keep_shape() {
    let (_, _, project) = pipeline(&[source!("positive", "nullable-lists", "api")]);
    assert_eq!(materialized(&project, "api", "missing"), &V::Null);
    assert_eq!(
        materialized(&project, "api", "present"),
        &V::String("yes".into())
    );
    assert_eq!(
        materialized(&project, "api", "matrix"),
        &V::List(vec![
            V::List(vec![number("1"), number("2")]),
            V::List(vec![]),
            V::List(vec![number("3")]),
        ])
    );
    assert_eq!(materialized(&project, "api", "empty"), &V::List(vec![]));
}

#[test]
fn integration_inert_location_literals_preserve_their_exact_spelling() {
    let (_, _, project) = pipeline(&[source!("positive", "locations", "api")]);
    assert_eq!(
        materialized(&project, "api", "endpoint"),
        &V::Url("HTTPS://Example.invalid/A%2fb".into())
    );
    assert_eq!(
        materialized(&project, "api", "file"),
        &V::Path("../A/../b".into())
    );
}

#[test]
fn integration_record_defaults_and_overrides_match_multiline_and_single_line_forms() {
    for (name, source) in [
        ("records", source!("positive", "records", "api")),
        (
            "records-no-trailing-comma",
            source!("positive", "records-no-trailing-comma", "api"),
        ),
        (
            "single-line-fields",
            source!("positive", "single-line-fields", "api"),
        ),
        (
            "single-line-fields-comma",
            source!("positive", "single-line-fields-comma", "api"),
        ),
    ] {
        let (_, _, project) = pipeline(&[source]);
        match name {
            "records" | "records-no-trailing-comma" => assert_eq!(
                materialized(&project, "api", "service"),
                &V::Record(vec![
                    (
                        "child".into(),
                        Some(V::Record(vec![("count".into(), Some(number("42")))]))
                    ),
                    ("note".into(), Some(V::Null)),
                    (
                        "ports".into(),
                        Some(V::List(vec![number("80"), number("443")]))
                    ),
                ])
            ),
            _ => {
                assert_eq!(
                    materialized(&project, "api", "defaults"),
                    &V::Record(vec![
                        ("count".into(), Some(number("42"))),
                        ("r".into(), Some(number("3")))
                    ])
                );
                assert_eq!(
                    materialized(&project, "api", "overridden"),
                    &V::Record(vec![
                        ("count".into(), Some(number("99"))),
                        ("r".into(), Some(number("3")))
                    ])
                );
            }
        }
    }
}

#[test]
fn integration_strings_preserve_unicode_and_escaped_scalars() {
    let (_, _, project) = pipeline(&[source!("positive", "unicode", "api")]);
    assert_eq!(
        materialized(&project, "api", "label"),
        &V::String("café\n世界\t\"quoted\"".into())
    );
}

#[test]
fn integration_nominal_reference_cycle_keeps_exact_target_identities() {
    let (_, _, project) = pipeline(&[source!("positive", "reference-cycle", "api")]);
    assert_eq!(
        materialized(&project, "api", "first"),
        &V::Record(vec![(
            "next".into(),
            Some(V::Reference(symbol("api", "second")))
        )])
    );
    assert_eq!(
        materialized(&project, "api", "second"),
        &V::Record(vec![(
            "next".into(),
            Some(V::Reference(symbol("api", "first")))
        )])
    );
}

#[test]
fn integration_vocabulary_aliases_materialize_values_from_exact_captured_bundles() {
    use neutral_compiler::CapturedVocabularyInput;
    use neutral_core::VocabularyContentDigest;
    use neutral_vocabulary::{VocabularyLock, composition};

    let captured = [
        include_bytes!("fixtures/positive/alpha.json").as_slice(),
        include_bytes!("fixtures/positive/beta.json").as_slice(),
    ]
    .into_iter()
    .map(|bytes| {
        let bundle: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        CapturedVocabularyInput::new(
            bytes.to_vec(),
            VocabularyLock::new(
                bundle["identity"].as_str().unwrap(),
                bundle["version"].as_str().unwrap(),
                composition::ENCODING_VERSION,
                composition::SCHEMA_VERSION,
                VocabularyContentDigest::from_bytes(bytes),
                vec![composition::REQUIRED_FEATURE.to_owned()],
            )
            .unwrap(),
        )
    })
    .collect();
    let (_, _, project) =
        pipeline_with_vocabularies(&[source!("positive", "vocabulary", "api")], captured);
    assert_eq!(
        materialized(&project, "api", "original"),
        &V::Record(vec![("label".into(), Some(V::String("typed".into())))])
    );
    assert_eq!(
        materialized(&project, "api", "copied"),
        &V::Record(vec![("label".into(), Some(V::String("typed".into())))])
    );
    assert_eq!(
        materialized(&project, "api", "other"),
        &V::Record(vec![("label".into(), Some(V::String("other".into())))])
    );
}
