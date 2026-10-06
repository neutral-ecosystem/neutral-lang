// SPDX-License-Identifier: Apache-2.0

//! Compiler-independent integration tests for the new explicit composition boundary.

use neutral_core::{CancellationToken, StructuralLimits, VocabularyContentDigest};
use neutral_ir::{
    ExactNumber,
    composition::{ClosedValue as V, CompositionBody, FieldPresence},
    project_interface::ProjectPublicType as T,
};
use neutral_vocabulary::{
    VocabularyError, VocabularyLimits, VocabularyLock,
    composition::{
        CapturedCompositionBundle, CompositionError as E, CompositionLimits, ENCODING_VERSION,
        REQUIRED_FEATURE, SCHEMA_VERSION, ValidatedComposition, validate_composition_closure,
    },
};

/// Runtime-owned complete example; it never reads an active/archived portable directory.
const EXAMPLE: &[u8] = include_bytes!("composition/fixtures/bundle.json");
/// Exact test semantic revision, independent of the workspace package version.
const REVISION: &str = "1.0.0";

/// Returns a generous finite independent budget policy for semantic tests.
fn limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ))
}

/// Constructs exact locked composition facts from captured bytes.
fn lock(identity: &str, bytes: &[u8]) -> VocabularyLock {
    VocabularyLock::new(
        identity,
        REVISION,
        ENCODING_VERSION,
        SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        vec![REQUIRED_FEATURE.to_owned()],
    )
    .unwrap()
}

/// Builds pretty test envelopes from literal definition and dependency entries.
fn bundle(identity: &str, dependencies: &str, definitions: &str) -> Vec<u8> {
    format!(
        r#"{{
  "format": "neutral-vocabulary-bundle",
  "encoding_version": "{ENCODING_VERSION}",
  "schema_version": "{SCHEMA_VERSION}",
  "identity": "{identity}",
  "version": "{REVISION}",
  "required_features": ["{REQUIRED_FEATURE}"],
  "dependencies": [{dependencies}],
  "types": [{definitions}]
}}
"#
    )
    .into_bytes()
}

/// Validates an isolated exact captured bundle without weakening dependency checks.
fn single(definitions: &str) -> Result<ValidatedComposition, E> {
    let bytes = bundle("Fixture", "", definitions);
    let lock = lock("Fixture", &bytes);
    validate_composition_closure(
        &[CapturedCompositionBundle {
            bytes: &bytes,
            lock: &lock,
        }],
        &[("Fixture", REVISION)],
        limits(),
        &CancellationToken::new(),
    )
}

/// Constructs one defaulted field around a literal type/value/restriction triple.
fn default_field(ty: &str, value: &str, restrictions: &str) -> String {
    format!(
        r#"{{
  "kind": "record", "name": "Item", "public": true,
  "fields": [{{
    "name": "value", "type": {ty}, "presence": "defaulted",
    "restrictions": {restrictions}, "default": {value}
  }}]
}}"#
    )
}

/// Extracts one single-field default from a successfully validated catalogue.
fn default_value(catalogue: &ValidatedComposition) -> &V {
    let CompositionBody::Record(fields) = &catalogue.bundles()[0].definitions[0].body else {
        panic!("record expected")
    };
    fields[0].default.as_ref().unwrap()
}

/// The complete example yields canonical nominal/ref types and validated restricted defaults.
#[test]
fn composition_complete_bundle_retains_contracts_and_defaults() {
    let lock = lock("ExampleDomain", EXAMPLE);
    let catalogue = validate_composition_closure(
        &[CapturedCompositionBundle {
            bytes: EXAMPLE,
            lock: &lock,
        }],
        &[("ExampleDomain", REVISION)],
        limits(),
        &CancellationToken::new(),
    )
    .unwrap();
    let bundle = &catalogue.bundles()[0];
    assert_eq!(
        bundle.identity.content_digest(),
        VocabularyContentDigest::from_bytes(EXAMPLE)
    );
    assert_eq!(bundle.definitions[0].name, "Outcome");
    let CompositionBody::Variant(alternatives) = &bundle.definitions[0].body else {
        panic!("variant expected")
    };
    assert_eq!(
        alternatives
            .iter()
            .map(|a| a.tag.as_str())
            .collect::<Vec<_>>(),
        ["failure", "success"]
    );
    let CompositionBody::Record(fields) = &bundle.definitions[1].body else {
        panic!("record expected")
    };
    assert_eq!(
        fields[0].default,
        Some(V::Number(ExactNumber::from_unsigned_integer("3").unwrap()))
    );
    assert_eq!(fields[1].presence, FieldPresence::Optional);
    assert_eq!(fields[1].default, None);
    assert!(matches!(fields[2].default, Some(V::List(_))));
    assert!(matches!(fields[3].ty, T::Ref(_)));
}

/// Unknown members and executable shapes reject at every composition schema boundary.
#[test]
fn security_composition_closed_schema_rejects_unknown_and_executable_members() {
    for (definitions, expected) in [
        (
            r#"{ "kind": "record", "name": "Item", "public": true, "fields": [], "metadata": {} }"#,
            E::Vocabulary(VocabularyError::UnknownMember),
        ),
        (
            r#"{ "kind": "variant", "name": "Item", "public": true, "alternatives": [], "script": "safe" }"#,
            E::Vocabulary(VocabularyError::ExecutableShapeForbidden),
        ),
        (
            r#"{ "kind": "variant", "name": "Item", "public": true, "alternatives": [] }"#,
            E::InvalidContract,
        ),
        (
            r#"{ "kind": "variant", "name": "Item", "public": true, "alternatives": [{ "tag": "ok", "type": { "kind": "string" } }, { "tag": "ok", "type": { "kind": "num" } }] }"#,
            E::InvalidContract,
        ),
    ] {
        assert_eq!(single(definitions), Err(expected));
    }
}

/// Type composition rejects ambiguous nullability, unsupported kinds and non-nominal refs.
#[test]
fn composition_invalid_type_shapes_fail_without_partial_catalogues() {
    for ty in [
        r#"{ "kind": "nullable", "inner": { "kind": "nullable", "inner": { "kind": "string" } } }"#,
        r#"{ "kind": "ref", "target": { "kind": "num" } }"#,
        r#"{ "kind": "map" }"#,
        r#"{ "kind": "external", "identity": "Fixture", "version": "1.0.0", "name": "Item" }"#,
    ] {
        assert_eq!(
            single(&default_field(ty, r#"{ "kind": "null" }"#, "{}")),
            Err(E::InvalidContract)
        );
    }
}

/// Numeric restrictions use exact inclusive bounds and normalize finite choices before comparison.
#[test]
fn composition_exact_numeric_bounds_and_choices_are_validated() {
    let ty = r#"{ "kind": "num" }"#;
    let value = r#"{ "kind": "num", "value": "0.1" }"#;
    for restrictions in [
        r#"{ "minimum": "0.1", "maximum": "0.1" }"#,
        r#"{ "choices": [{ "kind": "num", "value": "1e-1" }] }"#,
    ] {
        assert!(single(&default_field(ty, value, restrictions)).is_ok());
    }
    for (restrictions, expected) in [
        (r#"{ "minimum": "0.2" }"#, E::InvalidDefault),
        (r#"{ "maximum": "0.09" }"#, E::InvalidDefault),
        (
            r#"{ "minimum": "2", "maximum": "1" }"#,
            E::InvalidRestrictions,
        ),
        (r#"{ "choices": [] }"#, E::InvalidRestrictions),
        (
            r#"{ "choices": [{ "kind": "num", "value": "0.1" }, { "kind": "num", "value": "1e-1" }] }"#,
            E::DuplicateChoice,
        ),
        (
            r#"{ "choices": [{ "kind": "num", "value": "2" }], "maximum": "1" }"#,
            E::InvalidRestrictions,
        ),
        (r#"{ "max_length": "3" }"#, E::InvalidRestrictions),
    ] {
        assert_eq!(
            single(&default_field(ty, value, restrictions)),
            Err(expected)
        );
    }
}

/// Unicode length counts decoded scalars, not bytes/graphemes; length strings are canonical.
#[test]
fn composition_text_length_is_scalar_based_and_bounds_are_canonical() {
    let ty = r#"{ "kind": "string" }"#;
    let value = r#"{ "kind": "string", "value": "é🙂" }"#;
    assert!(
        single(&default_field(
            ty,
            value,
            r#"{ "min_length": "2", "max_length": "2" }"#
        ))
        .is_ok()
    );
    for (restriction, expected) in [
        (r#"{ "max_length": "1" }"#, E::InvalidDefault),
        (r#"{ "min_length": "3" }"#, E::InvalidDefault),
        (
            r#"{ "min_length": "2", "max_length": "1" }"#,
            E::InvalidRestrictions,
        ),
        (r#"{ "max_length": "01" }"#, E::InvalidRestrictions),
        (
            r#"{ "max_length": "18446744073709551616" }"#,
            E::InvalidRestrictions,
        ),
        (
            r#"{ "max_length": 2 }"#,
            E::Vocabulary(VocabularyError::RawJsonNumberForbidden),
        ),
        (r#"{ "minimum": "0" }"#, E::InvalidRestrictions),
    ] {
        assert_eq!(
            single(&default_field(ty, value, restriction)),
            Err(expected)
        );
    }
}

/// Null requires nullable typing but remains exempt from non-null restrictions.
#[test]
fn composition_null_is_explicit_and_never_an_implicit_default() {
    let value = r#"{ "kind": "null" }"#;
    let nullable = r#"{ "kind": "nullable", "inner": { "kind": "string" } }"#;
    assert_eq!(
        default_value(
            &single(&default_field(nullable, value, r#"{ "min_length": "1" }"#)).unwrap()
        ),
        &V::Null
    );
    assert_eq!(
        single(&default_field(r#"{ "kind": "string" }"#, value, "{}")),
        Err(E::InvalidDefault)
    );
    let nullable_ref = r#"{ "kind": "nullable", "inner": { "kind": "ref", "target": { "kind": "nominal", "name": "Item" } } }"#;
    assert!(single(&default_field(nullable_ref, value, "{}")).is_ok());
}

/// Closed default tags never acquire source lookup, reference, expression or executable channels.
#[test]
fn security_composition_defaults_are_closed_and_type_exact() {
    for value in [
        r#"{ "kind": "ref", "target": "answer" }"#,
        r#"{ "kind": "expression", "value": "safe" }"#,
        r#"{ "kind": "record", "fields": [] }"#,
        r#"{ "kind": "bool", "value": true }"#,
    ] {
        assert_eq!(
            single(&default_field(r#"{ "kind": "string" }"#, value, "{}")),
            Err(E::InvalidDefault)
        );
    }
    assert_eq!(
        single(&default_field(
            r#"{ "kind": "url" }"#,
            r#"{ "kind": "string", "value": "https://example.invalid" }"#,
            "{}"
        )),
        Err(E::InvalidDefault)
    );
}

/// Embedding traverses list/nullable wrappers and every unselected variant branch.
#[test]
fn security_composition_embedding_and_public_closure_cover_all_alternatives() {
    let recursive = r#"{
      "kind": "variant", "name": "Item", "public": true,
      "alternatives": [{ "tag": "safe", "type": { "kind": "string" } },
        { "tag": "recursive", "type": { "kind": "list", "element": { "kind": "nullable", "inner": { "kind": "nominal", "name": "Item" } } } }]
    }"#;
    assert_eq!(single(recursive), Err(E::EmbeddedCycle));
    let private = r#"{
      "kind": "variant", "name": "Item", "public": true,
      "alternatives": [{ "tag": "safe", "type": { "kind": "string" } },
        { "tag": "hidden", "type": { "kind": "nominal", "name": "Internal" } }]
    }, { "kind": "record", "name": "Internal", "public": false, "fields": [] }"#;
    assert_eq!(single(private), Err(E::PrivateType));
}

/// Invalid variant defaults reject unknown tags, wrong payloads and kind mismatches.
#[test]
fn composition_variant_defaults_validate_selected_tag_and_payload() {
    let variant = r#"{ "kind": "variant", "name": "Outcome", "public": true,
      "alternatives": [{ "tag": "ok", "type": { "kind": "string" } }] }"#;
    for (value, valid) in [
        (
            r#"{ "kind": "variant", "tag": "ok", "payload": { "kind": "string", "value": "done" } }"#,
            true,
        ),
        (
            r#"{ "kind": "variant", "tag": "bad", "payload": { "kind": "string", "value": "done" } }"#,
            false,
        ),
        (
            r#"{ "kind": "variant", "tag": "ok", "payload": { "kind": "num", "value": "3" } }"#,
            false,
        ),
        (r#"{ "kind": "record", "fields": [] }"#, false),
    ] {
        let record = default_field(r#"{ "kind": "nominal", "name": "Outcome" }"#, value, "{}");
        let result = single(&format!("{record},{variant}"));
        if valid {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(E::InvalidDefault));
        }
    }
}

/// Required-nullable omission rejects; optional absence and explicit null survive distinctly.
#[test]
fn composition_record_defaults_preserve_absent_null_and_defaulted_fields() {
    let item = r#"{
      "kind": "record", "name": "Child", "public": true,
      "fields": [
        { "name": "label", "type": { "kind": "nullable", "inner": { "kind": "string" } }, "presence": "optional", "restrictions": {} },
        { "name": "count", "type": { "kind": "num" }, "presence": "defaulted", "restrictions": {}, "default": { "kind": "num", "value": "3" } }
      ]
    }"#;
    let ty = r#"{ "kind": "nominal", "name": "Child" }"#;
    let empty = default_field(ty, r#"{ "kind": "record", "fields": [] }"#, "{}");
    let catalogue = single(&format!("{empty},{item}")).unwrap();
    let CompositionBody::Record(fields) = &catalogue.bundles()[0].definitions[1].body else {
        panic!("record expected")
    };
    let Some(V::Record(values)) = &fields[0].default else {
        panic!("default expected")
    };
    assert_eq!(values[0].0, "count");
    assert_eq!(values[1], ("label".to_owned(), None));
    let explicit = default_field(
        ty,
        r#"{ "kind": "record", "fields": [{ "name": "label", "value": { "kind": "null" } }] }"#,
        "{}",
    );
    let catalogue = single(&format!("{explicit},{item}")).unwrap();
    let CompositionBody::Record(fields) = &catalogue.bundles()[0].definitions[1].body else {
        panic!("record expected")
    };
    let Some(V::Record(values)) = &fields[0].default else {
        panic!("default expected")
    };
    assert_eq!(values[1], ("label".to_owned(), Some(V::Null)));
    let required = item.replace("\"presence\": \"optional\"", "\"presence\": \"required\"");
    assert_eq!(
        single(&format!("{empty},{required}")),
        Err(E::InvalidDefault)
    );
}

/// Builds an exactly locked dependency entry for graph test inputs.
fn dependency(identity: &str) -> String {
    format!(r#"{{ "identity": "{identity}", "version": "{REVISION}" }}"#)
}

/// Builds a public record using an external type through a non-embedding reference edge.
fn external_record(identity: &str) -> String {
    format!(
        r#"{{ "kind": "record", "name": "Item", "public": true,
      "fields": [{{ "name": "target", "type": {{ "kind": "ref", "target": {{
        "kind": "external", "identity": "{identity}", "version": "{REVISION}", "name": "Item"
      }} }}, "presence": "required", "restrictions": {{}} }}] }}"#
    )
}

/// Validates multiple runtime-generated captured inputs with exact root requirements.
fn closure(
    inputs: &[(&str, Vec<u8>)],
    roots: &[(&str, &str)],
    limits: CompositionLimits,
) -> Result<ValidatedComposition, E> {
    let locks = inputs
        .iter()
        .map(|(identity, bytes)| lock(identity, bytes))
        .collect::<Vec<_>>();
    let captured = inputs
        .iter()
        .zip(&locks)
        .map(|((_, bytes), lock)| CapturedCompositionBundle { bytes, lock })
        .collect::<Vec<_>>();
    validate_composition_closure(&captured, roots, limits, &CancellationToken::new())
}

/// Exact transitive cover allows shared leaf ownership, rejects missing/extra/conflicting closure.
#[test]
fn composition_cross_bundle_dependencies_are_exact_and_public() {
    let leaf = bundle(
        "Leaf",
        "",
        r#"{ "kind": "record", "name": "Item", "public": true, "fields": [] }"#,
    );
    let root = bundle("Root", &dependency("Leaf"), &external_record("Leaf"));
    let inputs = [("Root", root.clone()), ("Leaf", leaf.clone())];
    let catalogue = closure(&inputs, &[("Root", REVISION)], limits()).unwrap();
    assert_eq!(
        catalogue
            .bundles()
            .iter()
            .map(|b| b.identity.identity())
            .collect::<Vec<_>>(),
        ["Leaf", "Root"]
    );
    assert_eq!(
        closure(&inputs[..1], &[("Root", REVISION)], limits()),
        Err(E::MissingDependency)
    );
    assert_eq!(
        closure(&inputs, &[("Leaf", REVISION)], limits()),
        Err(E::ExtraBundle)
    );
    assert_eq!(
        closure(
            &[("Root", root.clone()), ("Root", root)],
            &[("Root", REVISION)],
            limits()
        ),
        Err(E::DuplicateBundle)
    );
    let private = String::from_utf8(leaf)
        .unwrap()
        .replace("\"public\": true", "\"public\": false")
        .into_bytes();
    assert_eq!(
        closure(
            &[("Root", inputs[0].1.clone()), ("Leaf", private)],
            &[("Root", REVISION)],
            limits()
        ),
        Err(E::PrivateType)
    );
}

/// Direct dependency cycles, unused requirements and self-dependencies reject independently of refs.
#[test]
fn composition_dependency_cycles_and_unused_declarations_fail_closed() {
    let a = bundle("Alpha", &dependency("Beta"), &external_record("Beta"));
    let b = bundle("Beta", &dependency("Alpha"), &external_record("Alpha"));
    assert_eq!(
        closure(
            &[("Alpha", a), ("Beta", b)],
            &[("Alpha", REVISION)],
            limits()
        ),
        Err(E::DependencyCycle)
    );
    let unused = bundle("Alpha", &dependency("Beta"), "");
    let leaf = bundle("Beta", "", "");
    assert_eq!(
        closure(
            &[("Alpha", unused), ("Beta", leaf)],
            &[("Alpha", REVISION)],
            limits()
        ),
        Err(E::InvalidDependency)
    );
    let self_dependency = bundle("Alpha", &dependency("Alpha"), "");
    assert_eq!(
        closure(
            &[("Alpha", self_dependency)],
            &[("Alpha", REVISION)],
            limits()
        ),
        Err(E::InvalidDependency)
    );
}

/// Cancellation and digest tampering abort without yielding even a partially valid catalogue.
#[test]
fn security_composition_cancellation_and_exact_digests_are_checked() {
    let good = lock("ExampleDomain", EXAMPLE);
    let captured = [CapturedCompositionBundle {
        bytes: EXAMPLE,
        lock: &good,
    }];
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        validate_composition_closure(
            &captured,
            &[("ExampleDomain", REVISION)],
            limits(),
            &cancelled
        ),
        Err(E::Cancelled)
    );
    let wrong = lock("ExampleDomain", b"different");
    assert_eq!(
        validate_composition_closure(
            &[CapturedCompositionBundle {
                bytes: EXAMPLE,
                lock: &wrong
            }],
            &[("ExampleDomain", REVISION)],
            limits(),
            &CancellationToken::new()
        ),
        Err(E::Vocabulary(VocabularyError::DigestMismatch))
    );
    let mut zero = limits();
    zero.total_alternatives = 0;
    assert_eq!(
        validate_composition_closure(
            &captured,
            &[("ExampleDomain", REVISION)],
            zero,
            &CancellationToken::new()
        ),
        Err(E::InvalidLimits)
    );
}

/// Separate alternatives/choices/type-depth/aggregate bytes enforce exact and one-over boundaries.
#[test]
fn security_composition_independent_limits_are_enforced_at_boundaries() {
    let lock = lock("ExampleDomain", EXAMPLE);
    let captured = [CapturedCompositionBundle {
        bytes: EXAMPLE,
        lock: &lock,
    }];
    let roots = [("ExampleDomain", REVISION)];
    let mut exact = limits();
    exact.alternatives_per_type = 2;
    exact.total_alternatives = 2;
    exact.choices_per_field = 2;
    exact.total_choices = 2;
    exact.total_fields = 4;
    exact.total_types = 2;
    exact.captured_bytes = EXAMPLE.len() as u64;
    assert!(
        validate_composition_closure(&captured, &roots, exact, &CancellationToken::new()).is_ok()
    );
    for short in [
        CompositionLimits {
            alternatives_per_type: 1,
            ..exact
        },
        CompositionLimits {
            total_alternatives: 1,
            ..exact
        },
        CompositionLimits {
            choices_per_field: 1,
            ..exact
        },
        CompositionLimits {
            total_choices: 1,
            ..exact
        },
        CompositionLimits {
            total_fields: 3,
            ..exact
        },
        CompositionLimits {
            total_types: 1,
            ..exact
        },
        CompositionLimits {
            captured_bytes: EXAMPLE.len() as u64 - 1,
            ..exact
        },
        CompositionLimits {
            type_depth: 1,
            ..exact
        },
        CompositionLimits {
            value_depth: 1,
            ..exact
        },
        CompositionLimits { work: 1, ..exact },
    ] {
        assert_eq!(
            validate_composition_closure(&captured, &roots, short, &CancellationToken::new()),
            Err(E::Limit)
        );
    }
}

/// Builds public reference fields for all explicit direct dependencies in graph fixtures.
fn external_records(targets: &[&str]) -> String {
    let fields = targets
        .iter()
        .enumerate()
        .map(|(index, identity)| {
            format!(
                r#"{{
      "name": "target_{index}", "type": {{ "kind": "ref", "target": {{
        "kind": "external", "identity": "{identity}", "version": "{REVISION}", "name": "Item"
      }} }}, "presence": "required", "restrictions": {{}}
    }}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(r#"{{ "kind": "record", "name": "Item", "public": true, "fields": [{fields}] }}"#)
}

/// Diamond closure shares a leaf but accounts for every edge and the longest dependency path.
#[test]
fn composition_diamond_dependency_limits_are_independent_and_order_invariant() {
    let root = bundle(
        "Root",
        &format!("{},{}", dependency("Left"), dependency("Right")),
        &external_records(&["Left", "Right"]),
    );
    let left = bundle("Left", &dependency("Leaf"), &external_record("Leaf"));
    let right = bundle("Right", &dependency("Leaf"), &external_record("Leaf"));
    let leaf = bundle("Leaf", "", &external_records(&[]));
    let inputs = [
        ("Root", root),
        ("Left", left),
        ("Right", right),
        ("Leaf", leaf),
    ];
    let exact = CompositionLimits {
        bundles: 4,
        dependencies_per_bundle: 2,
        dependency_edges: 4,
        dependency_depth: 3,
        ..limits()
    };
    let first = closure(&inputs, &[("Root", REVISION)], exact).unwrap();
    let mut reversed = inputs.clone();
    reversed.reverse();
    assert_eq!(
        closure(&reversed, &[("Root", REVISION)], exact).unwrap(),
        first
    );
    for short in [
        CompositionLimits {
            bundles: 3,
            ..exact
        },
        CompositionLimits {
            dependencies_per_bundle: 1,
            ..exact
        },
        CompositionLimits {
            dependency_edges: 3,
            ..exact
        },
        CompositionLimits {
            dependency_depth: 2,
            ..exact
        },
    ] {
        assert_eq!(
            closure(&inputs, &[("Root", REVISION)], short),
            Err(E::Limit)
        );
    }
}

/// A shallow first visit cannot hide a deeper path to the same dependency leaf.
#[test]
fn security_composition_shared_leaf_depth_uses_longest_path() {
    let root = bundle(
        "Root",
        &format!("{},{}", dependency("Leaf"), dependency("Middle")),
        &external_records(&["Leaf", "Middle"]),
    );
    let middle = bundle("Middle", &dependency("Next"), &external_record("Next"));
    let next = bundle("Next", &dependency("Leaf"), &external_record("Leaf"));
    let leaf = bundle("Leaf", "", &external_records(&[]));
    let inputs = [
        ("Root", root),
        ("Middle", middle),
        ("Next", next),
        ("Leaf", leaf),
    ];
    assert!(
        closure(
            &inputs,
            &[("Root", REVISION)],
            CompositionLimits {
                dependency_depth: 4,
                ..limits()
            }
        )
        .is_ok()
    );
    assert_eq!(
        closure(
            &inputs,
            &[("Root", REVISION)],
            CompositionLimits {
                dependency_depth: 3,
                ..limits()
            }
        ),
        Err(E::Limit)
    );
}

/// New composition bundles can depend on an exactly locked frozen-schema public leaf.
#[test]
fn compatibility_composition_accepts_existing_project_schema_leaf_without_reinterpretation() {
    let root = bundle("Root", &dependency("Leaf"), &external_record("Leaf"));
    let leaf = format!(r#"{{
      "format": "neutral-vocabulary-bundle", "encoding_version": "{}", "schema_version": "{}",
      "identity": "Leaf", "version": "{REVISION}", "required_features": [],
      "types": [{{ "name": "Item", "public": true, "fields": [{{ "name": "count", "type": "num" }}] }}]
    }}"#, neutral_vocabulary::PROJECT_VOCABULARY_ENCODING_VERSION, neutral_vocabulary::PROJECT_VOCABULARY_SCHEMA_VERSION).into_bytes();
    let root_lock = lock("Root", &root);
    let leaf_lock = VocabularyLock::new(
        "Leaf",
        REVISION,
        neutral_vocabulary::PROJECT_VOCABULARY_ENCODING_VERSION,
        neutral_vocabulary::PROJECT_VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(&leaf),
        Vec::new(),
    )
    .unwrap();
    let inputs = [
        CapturedCompositionBundle {
            bytes: &root,
            lock: &root_lock,
        },
        CapturedCompositionBundle {
            bytes: &leaf,
            lock: &leaf_lock,
        },
    ];
    let catalogue = validate_composition_closure(
        &inputs,
        &[("Root", REVISION)],
        limits(),
        &CancellationToken::new(),
    )
    .unwrap();
    let CompositionBody::Record(fields) = &catalogue.bundles()[0].definitions[0].body else {
        panic!("record expected")
    };
    assert_eq!(fields[0].presence, FieldPresence::Required);
    assert_eq!(fields[0].default, None);
    assert_eq!(fields[0].ty, T::Num);
}

/// Declared dependency revisions and feature sets must match exact supplied locks.
#[test]
fn security_composition_versions_and_feature_selection_fail_closed() {
    let root = bundle(
        "Root",
        &dependency("Leaf").replace(REVISION, "2.0.0"),
        &external_record("Leaf"),
    );
    let leaf = bundle("Leaf", "", &external_records(&[]));
    assert_eq!(
        closure(
            &[("Root", root), ("Leaf", leaf)],
            &[("Root", REVISION)],
            limits()
        ),
        Err(E::MissingDependency)
    );
    let valid = String::from_utf8(bundle("Fixture", "", "")).unwrap();
    for replacement in ["unknown-feature", "", "vocabulary-composition-v2,other"] {
        let bytes = valid.replace(REQUIRED_FEATURE, replacement).into_bytes();
        let locked = lock("Fixture", &bytes);
        assert_eq!(
            validate_composition_closure(
                &[CapturedCompositionBundle {
                    bytes: &bytes,
                    lock: &locked
                }],
                &[("Fixture", REVISION)],
                limits(),
                &CancellationToken::new()
            ),
            Err(E::Vocabulary(VocabularyError::UnknownRequiredFeature))
        );
    }
}

/// Lists enforce immediate-element length and exact contextual element types.
#[test]
fn composition_list_defaults_are_homogeneous_and_length_restricted() {
    let ty = r#"{ "kind": "list", "element": { "kind": "bool" } }"#;
    let value = r#"{ "kind": "list", "items": [{ "kind": "bool", "value": true }, { "kind": "bool", "value": false }] }"#;
    assert!(
        single(&default_field(
            ty,
            value,
            r#"{ "min_length": "2", "max_length": "2" }"#
        ))
        .is_ok()
    );
    assert_eq!(
        single(&default_field(ty, value, r#"{ "max_length": "1" }"#)),
        Err(E::InvalidDefault)
    );
    assert_eq!(
        single(&default_field(
            ty,
            &value.replace(
                "\"kind\": \"bool\", \"value\": false",
                "\"kind\": \"string\", \"value\": \"false\""
            ),
            "{}"
        )),
        Err(E::InvalidDefault)
    );
}

/// Inert locations are preserved exactly; finite choices never coerce URL/path/string kinds.
#[test]
fn composition_location_defaults_are_inert_and_choices_are_exactly_typed() {
    let ty = r#"{ "kind": "path" }"#;
    let value = r#"{ "kind": "path", "value": "../example.invalid/%2F/./" }"#;
    assert_eq!(
        default_value(&single(&default_field(ty, value, "{}")).unwrap()),
        &V::Path("../example.invalid/%2F/./".to_owned())
    );
    let wrong = r#"{ "choices": [{ "kind": "string", "value": "../example.invalid/%2F/./" }] }"#;
    assert_eq!(
        single(&default_field(ty, value, wrong)),
        Err(E::InvalidRestrictions)
    );
}

/// Invalid defaults on private unused types still reject the entire captured catalogue.
#[test]
fn security_composition_unused_private_contracts_are_fully_validated() {
    let private = default_field(
        r#"{ "kind": "string" }"#,
        r#"{ "kind": "bool", "value": false }"#,
        "{}",
    )
    .replace("\"public\": true", "\"public\": false");
    assert_eq!(single(&private), Err(E::InvalidDefault));
}

/// Caller policies cannot enlarge the schema's hard per-object duplicate-inspection bound.
#[test]
fn security_composition_json_object_work_has_a_hard_schema_bound() {
    let bytes = bundle(
        "Fixture",
        "",
        r#"{ "kind": "record", "name": "Item", "public": true, "fields": [],
      "extra_a": true, "extra_b": true, "extra_c": true, "extra_d": true, "extra_e": true }"#,
    );
    let lock = lock("Fixture", &bytes);
    let policy = CompositionLimits {
        json: limits().json.with_object_members(u64::MAX).unwrap(),
        ..limits()
    };
    assert_eq!(
        validate_composition_closure(
            &[CapturedCompositionBundle {
                bytes: &bytes,
                lock: &lock
            }],
            &[("Fixture", REVISION)],
            policy,
            &CancellationToken::new()
        ),
        Err(E::Vocabulary(VocabularyError::JsonLimitExceeded))
    );
}

/// Repeated/concurrent requests retain identical immutable facts with no cross-request counters.
#[test]
fn composition_catalogues_are_deterministic_across_concurrent_requests() {
    let expected = single(&external_records(&[])).unwrap();
    let workers = (0..8)
        .map(|_| std::thread::spawn(|| single(&external_records(&[])).unwrap()))
        .collect::<Vec<_>>();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), expected);
    }
}

/// Every public composition classification has a distinct stable safe diagnostic code.
#[test]
fn composition_diagnostic_codes_are_stable_and_unique() {
    let errors = [
        E::Vocabulary(VocabularyError::UnknownMember),
        E::InvalidLimits,
        E::Limit,
        E::Cancelled,
        E::DuplicateBundle,
        E::MissingDependency,
        E::ExtraBundle,
        E::InvalidDependency,
        E::DependencyCycle,
        E::PrivateType,
        E::UnknownType,
        E::EmbeddedCycle,
        E::InvalidContract,
        E::InvalidRestrictions,
        E::DuplicateChoice,
        E::InvalidDefault,
        E::InvalidValue,
        E::Allocation,
    ];
    let mut codes = std::collections::BTreeSet::new();
    for (index, error) in errors.iter().enumerate() {
        assert_eq!(error.diagnostic_code(), format!("NEU-COM-{:03}", index + 1));
        assert!(codes.insert(error.diagnostic_code()));
    }
}
