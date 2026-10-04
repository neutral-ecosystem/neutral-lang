// SPDX-License-Identifier: Apache-2.0

//! Typed configuration syntax and failure diagnostics.

use super::*;

/// TOML arrays preserve literal commas, escapes, comments, and multiline entries.
#[test]
fn automation_configuration_uses_standard_toml_syntax() {
    let source = "[tools]\nvalues = [\n 'one,two', # comma in string\n \"quoted\\\"value\",\n]\n";
    assert_eq!(
        configuration_array_from(source, "tools", "values").expect("array"),
        ["one,two", "quoted\"value"]
    );
    assert_eq!(
        quality_value_from(
            "[package]\nname='literal' # inline comment",
            "package",
            "name"
        ),
        Some("literal".to_owned())
    );
    assert!(configuration_array_from("[tools]\nvalues=[1]", "tools", "values").is_err());
    assert!(configuration_array_from("[tools]\nvalues=[\"\"]", "tools", "values").is_err());
    assert!(configuration_array_from("[tools]\nvalues=[]", "tools", "values").is_err());
    assert!(
        parse::<toml::Table>("broken=", "settings.toml")
            .unwrap_err()
            .contains("settings.toml")
    );
}

/// Closed schemas reject misspellings, missing required sections, and wrong value types.
#[test]
fn automation_configuration_rejects_unknown_and_missing_fields() {
    let source = crate::read_workspace_text(
        &crate::workspace_root().expect("root"),
        constants::AUTOMATION_CONFIG_FILE,
    )
    .expect("config");
    let _: Automation = parse(&source, "automation").expect("typed settings");
    let mut invalid: toml::Table = parse(&source, "automation").expect("settings document");
    let testing = invalid["testing"].as_table_mut().expect("testing settings");
    let profile = testing.remove("ci_profile").expect("CI profile");
    testing.insert("ci_profle".to_owned(), profile);
    assert!(
        parse::<Automation>(
            &toml::to_string(&invalid).expect("misspelled settings"),
            "automation"
        )
        .is_err()
    );
    let mut invalid: toml::Table = parse(&source, "automation").expect("settings document");
    invalid["testing"]["runner"] = toml::Value::String("unknown".to_owned());
    assert!(
        parse::<Automation>(
            &toml::to_string(&invalid).expect("invalid settings"),
            "automation"
        )
        .is_err()
    );
    assert!(parse::<Automation>("schema_version=1", "automation").is_err());
    let mut invalid: toml::Table = parse(&source, "automation").expect("settings document");
    invalid["quality"]["advisory_max_age_seconds"] = toml::Value::String("not-a-number".to_owned());
    assert!(
        parse::<Automation>(
            &toml::to_string(&invalid).expect("invalid settings"),
            "automation"
        )
        .is_err()
    );
    assert!(require_schema(constants::CONFIG_SCHEMA_VERSION + 1, "test config").is_err());
}
