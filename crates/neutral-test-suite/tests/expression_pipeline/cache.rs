// SPDX-License-Identifier: Apache-2.0

//! Real source-cache generations cross transport and independent public inspection.

use super::*;
use neutral_compiler::{ExpressionCompilationCache, ProjectCacheLimits};
use neutral_core::{VocabularyContentDigest, allocation::Shared};
use neutral_ir::expression::project::ExpressionProjectIr;
use neutral_ir::project_identity::{IdentityLimits, canonical_expression_project};
use neutral_vocabulary::{VocabularyLock, composition};

/// Every produced generation must independently validate and encode identically to clean compilation.
pub(super) fn checked(ir: Shared<ExpressionProjectIr>) -> ValidatedExpressionProject {
    let signal = CancellationToken::new();
    let limits = SemanticLimits {
        project: ir.limits,
        composition: composition_limits(),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    };
    let authority = ValidatedExpressionProject::from_ir(ir, limits, &signal).unwrap();
    let bytes = encode_expression_project(&authority, &signal).unwrap();
    let decoded = decode_expression_project(&bytes, DecodeLimits::hard(), limits, &signal).unwrap();
    assert_eq!(authority.ir(), decoded.ir());
    decoded
}

/// Two modules exercise cached signatures, template defaults, matrix syntax and fresh source evidence.
#[test]
#[allow(clippy::too_many_lines)] // One generation sequence compares clean, warm, changed and restored authority.
fn integration_expression_cache_clean_warm_changed_units_and_source_evidence_agree() {
    let main = include_bytes!("fixtures/cache-main.neu");
    let types = include_bytes!("fixtures/cache-types.neu");
    let make = |types: &[u8], id: &str| {
        capture_sources(
            vec![
                CapturedSourceInput::new("source:main", "pipeline", main.to_vec()),
                CapturedSourceInput::new(id, "pipeline::types", types.to_vec()),
            ],
            vec![],
        )
    };
    let captured = make(types, "source:types");
    let signal = CancellationToken::new();
    let mut cache = ExpressionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: 1_048_576,
    })
    .unwrap();
    let (cold, stats) = cache.compile_expression(&captured, &signal).unwrap();
    assert_eq!((stats.parsed_units, stats.reused_units), (2, 0));
    assert_eq!(
        cold,
        compile_expression_project(&captured, &signal).unwrap()
    );
    let original = checked(cold);
    assert_eq!(
        value(&original, "entry"),
        &V::Record(vec![(
            "count".into(),
            Some(V::Number(ExactNumber::from_source("8", 64, 64).unwrap()))
        )])
    );
    let (warm, stats) = cache.compile_expression(&captured, &signal).unwrap();
    assert_eq!((stats.parsed_units, stats.reused_units), (0, 2));
    assert_eq!(checked(warm).ir(), original.ir());
    let changed_bytes = std::str::from_utf8(types)
        .unwrap()
        .replace("value + 1", "value + 2");
    let changed = make(changed_bytes.as_bytes(), "source:new-types");
    let (actual, stats) = cache.compile_expression(&changed, &signal).unwrap();
    assert_eq!(
        (
            stats.parsed_units,
            stats.reused_units,
            stats.rejected_entries
        ),
        (1, 1, 1)
    );
    assert_eq!(
        actual,
        compile_expression_project(&changed, &signal).unwrap()
    );
    let changed = checked(actual);
    assert_eq!(
        value(&changed, "entry"),
        &V::Record(vec![(
            "count".into(),
            Some(V::Number(ExactNumber::from_source("9", 64, 64).unwrap()))
        )])
    );
    let renamed = make(types, "source:renamed");
    let identity_limits = IdentityLimits {
        bytes: 1_048_576,
        nodes: 1_048_576,
    };
    assert_ne!(
        renamed
            .identity_transcript(identity_limits, &signal)
            .unwrap()
            .identity(),
        captured
            .identity_transcript(identity_limits, &signal)
            .unwrap()
            .identity()
    );
    let (renamed, stats) = cache.compile_expression(&renamed, &signal).unwrap();
    assert_eq!(stats.reused_units, 1);
    assert_eq!(
        renamed
            .sources
            .iter()
            .find(|s| s.module == "pipeline::types")
            .unwrap()
            .source_id,
        "source:renamed"
    );
    let limits = IdentityLimits {
        bytes: 1_048_576,
        nodes: 1_048_576,
    };
    assert_eq!(
        canonical_expression_project(&renamed, limits, &signal)
            .unwrap()
            .identity(),
        canonical_expression_project(original.ir(), limits, &signal)
            .unwrap()
            .identity()
    );
    let (_, stats) = cache
        .compile_expression(&make(types, "source:again"), &signal)
        .unwrap();
    assert_eq!(stats.reused_units, 2);
}

/// Changed exact lock/default/restriction data must rebuild contracts despite unchanged source syntax.
#[test]
fn integration_expression_cache_refreshes_vocabulary_defaults_and_restrictions() {
    let source = b"neu \"1.0\"\nmodule pipeline\nuse PipelineDomain as domain\npublic domain::Plan entry = {}\n";
    let original = std::str::from_utf8(include_bytes!("fixtures/domain.json")).unwrap();
    let bundle = |number: &str, maximum: &str| {
        original.replace("\"presence\": \"required\",\n          \"restrictions\": {\"minimum\": \"0\", \"maximum\": \"3\"}", &format!("\"presence\": \"defaulted\",\n          \"default\": {{\"kind\": \"num\", \"value\": \"{number}\"}},\n          \"restrictions\": {{\"minimum\": \"0\", \"maximum\": \"{maximum}\"}}"))
    };
    let make = |source: &[u8], bytes: &str| {
        let lock = VocabularyLock::new(
            "PipelineDomain",
            "1.0.0",
            composition::ENCODING_VERSION,
            composition::SCHEMA_VERSION,
            VocabularyContentDigest::from_bytes(bytes.as_bytes()),
            vec![composition::REQUIRED_FEATURE.into()],
        )
        .unwrap();
        capture_with_vocabularies(
            source,
            vec![neutral_compiler::CapturedVocabularyInput::new(
                bytes.as_bytes().to_vec(),
                lock,
            )],
        )
    };
    let mut cache = ExpressionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: 1_048_576,
    })
    .unwrap();
    let signal = CancellationToken::new();
    let mut identities = Vec::new();
    for (index, (number, maximum)) in [("1", "3"), ("2", "4"), ("1", "3")].into_iter().enumerate() {
        let captured = make(source, &bundle(number, maximum));
        let (actual, stats) = cache.compile_expression(&captured, &signal).unwrap();
        assert_eq!(stats.reused_units, u64::from(index != 0));
        assert_eq!(
            actual,
            compile_expression_project(&captured, &signal).unwrap()
        );
        let actual = checked(actual);
        assert_eq!(
            value(&actual, "entry"),
            &V::Record(vec![(
                "value".into(),
                Some(V::Number(ExactNumber::from_source(number, 64, 64).unwrap()))
            )])
        );
        identities.push(
            canonical_expression_project(
                actual.ir(),
                IdentityLimits {
                    bytes: 1_048_576,
                    nodes: 1_048_576,
                },
                &signal,
            )
            .unwrap()
            .identity(),
        );
    }
    assert_ne!(identities[0], identities[1]);
    assert_eq!(identities[0], identities[2]);
    let supplied = std::str::from_utf8(source)
        .unwrap()
        .replace("entry = {}", "entry = { value: 2 }");
    let good = make(supplied.as_bytes(), &bundle("1", "3"));
    let bad = make(supplied.as_bytes(), &bundle("1", "1"));
    let (expected, _) = cache.compile_expression(&good, &signal).unwrap();
    assert_eq!(
        cache.compile_expression(&bad, &signal).unwrap_err(),
        compile_expression_project(&bad, &signal).unwrap_err()
    );
    let (recovered, stats) = cache.compile_expression(&good, &signal).unwrap();
    assert_eq!(stats.reused_units, 1);
    assert_eq!(recovered, expected);
}
