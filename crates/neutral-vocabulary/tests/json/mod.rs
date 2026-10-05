// SPDX-License-Identifier: Apache-2.0

//! Strict Unicode, JSON syntax, and allocation boundary regressions.

use super::*;
use neutral_core::StructuralLimits;

/// Returns bounded parser limits independent of semantic bundle validation.
fn limits() -> VocabularyLimits {
    VocabularyLimits::from_structural(StructuralLimits::new(4096, 16).unwrap())
}

/// Explicitly cancellable parsing fails safely while frozen non-cancellable decoding remains unchanged.
#[test]
fn composition_json_cancellation_does_not_reinterpret_legacy_parsing() {
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    for text in ["null", "[]", r#""exact text""#, r#"{"label":"é🙂"}"#] {
        assert_eq!(
            parse_cancellable(text, limits(), &cancelled),
            Err(VocabularyError::Cancelled)
        );
        assert!(parse(text, limits()).is_ok());
    }
    let running = CancellationToken::new();
    assert_eq!(
        parse_cancellable("[null,true]", limits(), &running),
        parse("[null,true]", limits())
    );
}

/// All permitted escapes and both Unicode hex cases decode to exact scalar values.
#[test]
fn unicode_and_every_escape_preserve_exact_text() {
    assert_eq!(
        parse(
            r#""\"\\\/\b\f\n\r\t\u0041\u00e9\u00E9\uD83D\uDE00世界""#,
            limits()
        ),
        Ok(JsonValue::String(
            "\"\\/\u{8}\u{c}\n\r\tAéé😀世界".to_owned()
        ))
    );
    assert_eq!(
        parse(" \t\r\n[null,true,false,{},[]] \n", limits()),
        Ok(JsonValue::Array(vec![
            JsonValue::Null,
            JsonValue::Bool(true),
            JsonValue::Bool(false),
            JsonValue::Object(vec![]),
            JsonValue::Array(vec![])
        ]))
    );
}

/// Invalid separators, truncated escapes, raw controls and unpaired surrogates are rejected.
#[test]
fn malformed_json_and_surrogates_never_publish_a_tree() {
    for text in [
        "",
        "nul",
        "tru",
        "fals",
        "null x",
        "[true false]",
        "[true,]",
        "{true:null}",
        "{\"a\" null}",
        "{\"a\":null \"b\":false}",
        "{\"a\":null,}",
        "\"unterminated",
        "\"\\",
        "\"\n\"",
        r#""\x""#,
        r#""\u123""#,
        r#""\uGGGG""#,
        r#""\uD800""#,
        r#""\uD800x""#,
        r#""\uD800\x1234""#,
        r#""\uD800\u0041""#,
        r#""\uDC00""#,
    ] {
        assert_eq!(
            parse(text, limits()),
            Err(VocabularyError::MalformedJson),
            "{text:?}"
        );
    }
    for text in ["0", "-1", "1.5", "[42]"] {
        assert_eq!(
            parse(text, limits()),
            Err(VocabularyError::RawJsonNumberForbidden)
        );
    }
    assert_eq!(
        parse(r#"{"a":null,"\u0061":true}"#, limits()),
        Err(VocabularyError::DuplicateJsonMember)
    );
}

/// Limits measure decoded UTF-8 bytes and parsed nodes, including container roots.
#[test]
fn decoded_text_and_container_limits_are_exact() {
    let structural = StructuralLimits::new(4096, 16)
        .unwrap()
        .with_string_bytes(4)
        .unwrap()
        .with_nesting_depth(2)
        .unwrap()
        .with_traversal_nodes(3)
        .unwrap();
    let bounded = VocabularyLimits::from_structural(structural)
        .with_array_items(2)
        .unwrap()
        .with_object_members(1)
        .unwrap();
    assert_eq!(
        parse(r#""\uD83D\uDE00""#, bounded),
        Ok(JsonValue::String("😀".to_owned()))
    );
    for text in [
        r#""😀a""#,
        r#""\uD83D\uDE00a""#,
        "[[null]]",
        "[true,false,null]",
        r#"{"a":null,"b":null}"#,
    ] {
        assert_eq!(
            parse(text, bounded),
            Err(VocabularyError::JsonLimitExceeded),
            "{text}"
        );
    }
    assert!(parse("[true,false]", bounded).is_ok());
    assert!(parse(r#"{"a":null}"#, bounded).is_ok());
    let nodes = VocabularyLimits::from_structural(
        StructuralLimits::new(4096, 16)
            .unwrap()
            .with_traversal_nodes(2)
            .unwrap(),
    );
    assert_eq!(
        parse("[true,false]", nodes),
        Err(VocabularyError::JsonLimitExceeded)
    );
}
