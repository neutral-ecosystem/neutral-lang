// SPDX-License-Identifier: Apache-2.0

//! Test-only conversion of frozen resolved facts, never an oracle or a semantic reader.

use super::*;
use neutral_ir::{
    ExactNumber, VocabularyIdentity,
    composition::project::{CompositionDeclaration, CompositionProjectIr, CompositionSignature},
    composition::{
        CompositionAlternative, CompositionBody, CompositionBundle, CompositionDefinition,
        CompositionDependency, CompositionField, CompositionValue, FieldPresence,
        FieldRestrictions,
    },
    project::{ProjectModule, ProjectProvenance},
    project_interface::{ProjectPublicEdgeKind, ProjectPublicType},
};

/// Reads an immutable fixture string; malformed fixtures must fail, not silently normalize.
fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}
/// Iterates an explicitly registered ordered fixture collection.
fn list(v: &Value) -> std::slice::Iter<'_, Value> {
    v.as_array().unwrap().iter()
}
/// Retains literal string collections without sorting or canonicalizing them.
fn strings(v: &Value) -> Vec<String> {
    list(v).map(|v| text(v).to_owned()).collect()
}
/// Constructs the complete stable symbol tuple from registered semantic facts.
pub(super) fn symbol(v: &Value) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(text(&v[0]), text(&v[1])),
        text(&v[2]),
    )
}
/// Builds typed contracts from literal facts, preserving invalid wrapper shapes for framing tests.
fn ty(v: &Value) -> ProjectPublicType {
    use ProjectPublicType as T;
    match text(&v["kind"]) {
        "num" => T::Num,
        "string" => T::String,
        "bool" => T::Bool,
        "url" => T::Url,
        "path" => T::Path,
        "nominal" => T::Nominal(symbol(&v["symbol"])),
        "vocabulary" => T::VocabularyNominal {
            identity: text(&v["identity"]).to_owned(),
            version: text(&v["version"]).to_owned(),
            name: text(&v["name"]).to_owned(),
        },
        "List" => T::List(Box::new(ty(&v["inner"]))),
        "Ref" => T::Ref(Box::new(ty(&v["inner"]))),
        "nullable" => T::Nullable(Box::new(ty(&v["inner"]))),
        other => panic!("unregistered type {other}"),
    }
}
/// Uses the public normalized-parts trust boundary, never source spelling normalization.
fn number(v: &Value) -> ExactNumber {
    ExactNumber::from_normalized_parts(
        v["negative"].as_bool().unwrap(),
        text(&v["coefficient"]),
        v["scale"].as_i64().unwrap(),
        65_536,
        u64::MAX,
    )
    .unwrap()
}
/// Builds values with an explicit reference constructor; closed defaults cannot acquire a reference.
fn value<R>(v: &Value, reference: fn(&Value) -> R) -> CompositionValue<R> {
    use CompositionValue as V;
    match text(&v["kind"]) {
        "num" => V::Number(number(v)),
        "string" => V::String(text(&v["value"]).to_owned()),
        "bool" => V::Bool(v["value"].as_bool().unwrap()),
        "null" => V::Null,
        "url" => V::Url(text(&v["value"]).to_owned()),
        "path" => V::Path(text(&v["value"]).to_owned()),
        "Ref" => V::Reference(reference(&v["value"])),
        "List" => V::List(list(&v["value"]).map(|v| value(v, reference)).collect()),
        "record" => V::Record(
            list(&v["value"])
                .map(|f| {
                    (
                        text(&f[0]).to_owned(),
                        (f[1]["kind"] != "omitted").then(|| value(&f[1], reference)),
                    )
                })
                .collect(),
        ),
        "variant" => V::Variant {
            tag: text(&v["tag"]).to_owned(),
            payload: Box::new(value(&v["payload"], reference)),
        },
        other => panic!("unrepresentable typed value {other}"),
    }
}
/// Fails malformed fixtures attempting to place a binding reference inside a closed default.
fn no_reference(_: &Value) -> std::convert::Infallible {
    panic!("closed fixture contains a reference")
}
/// Reads nullable contract bounds without inventing a default restriction.
fn optional_number(v: &Value) -> Option<ExactNumber> {
    (!v.is_null()).then(|| number(v))
}
/// Retains every semantic field fact, including unused defaults and restrictions.
fn field(v: &Value) -> CompositionField {
    let r = &v["restrictions"];
    CompositionField {
        name: text(&v["name"]).to_owned(),
        ty: ty(&v["type"]),
        presence: match text(&v["presence"]) {
            "required" => FieldPresence::Required,
            "optional" => FieldPresence::Optional,
            "defaulted" => FieldPresence::Defaulted,
            other => panic!("unknown presence {other}"),
        },
        restrictions: FieldRestrictions {
            choices: (!r["choices"].as_array().unwrap().is_empty()).then(|| {
                list(&r["choices"])
                    .map(|v| value(v, no_reference))
                    .collect()
            }),
            minimum: optional_number(&r["minimum"]),
            maximum: optional_number(&r["maximum"]),
            min_length: r["min_length"].as_u64(),
            max_length: r["max_length"].as_u64(),
        },
        default: (!v["default"].is_null()).then(|| value(&v["default"], no_reference)),
    }
}
/// Shares one resolved record/variant model for both nominal origins in the test adapter.
fn body(v: &Value) -> CompositionBody {
    match text(&v["kind"]) {
        "record" => CompositionBody::Record(list(&v["fields"]).map(field).collect()),
        "variant" => CompositionBody::Variant(
            list(&v["alternatives"])
                .map(|a| CompositionAlternative {
                    tag: text(&a["tag"]).to_owned(),
                    ty: ty(&a["type"]),
                })
                .collect(),
        ),
        other => panic!("unknown definition {other}"),
    }
}
/// Replaces only meaning in caller-owned raw IR; inherited companions deliberately confer no authority.
pub(super) fn project(base: &CompositionProjectIr, v: &Value) -> CompositionProjectIr {
    let mut ir = base.clone();
    ir.schema = text(&v["schema"]).to_owned();
    ir.modules = list(&v["modules"])
        .map(|m| ProjectModule {
            identity: LogicalModuleIdentity::new(text(&v["profile"]), text(&m["name"])),
            imports: strings(&m["imports"]),
        })
        .collect();
    ir.declarations = list(&v["declarations"])
        .map(|d| CompositionDeclaration {
            identity: symbol(&d["symbol"]),
            public: d["public"].as_bool().unwrap(),
            signature: if d["signature"]["kind"] == "binding" {
                CompositionSignature::Binding(ty(&d["signature"]["type"]))
            } else {
                CompositionSignature::Definition(body(&d["signature"]))
            },
            value: (d["value"]["kind"] != "absent").then(|| value(&d["value"], symbol)),
        })
        .collect();
    ir.vocabularies = list(&v["catalogues"])
        .map(|c| {
            let dependencies = list(&v["dependencies"])
                .find(|d| d["identity"] == c["identity"] && d["version"] == c["version"])
                .unwrap();
            CompositionBundle {
                identity: VocabularyIdentity::new(
                    text(&c["identity"]),
                    text(&c["version"]),
                    text(&c["schema_version"]),
                    neutral_vocabulary::PROJECT_VOCABULARY_ENCODING_VERSION,
                    VocabularyContentDigest::from_bytes(&[]),
                    strings(&c["features"]),
                ),
                dependencies: list(&dependencies["dependencies"])
                    .map(|d| CompositionDependency {
                        identity: text(&d[0]).to_owned(),
                        version: text(&d[1]).to_owned(),
                    })
                    .collect(),
                definitions: list(&v["definitions"])
                    .filter(|d| d["identity"] == c["identity"] && d["version"] == c["version"])
                    .map(|d| CompositionDefinition {
                        name: text(&d["name"]).to_owned(),
                        public: d["public"].as_bool().unwrap(),
                        body: body(&d["body"]),
                    })
                    .collect(),
            }
        })
        .collect();
    ir.provenance = list(&v["edges"])
        .map(|e| ProjectProvenance {
            from: symbol(&e["from"]),
            to: symbol(&e["to"]),
            kind: match text(&e["kind"]) {
                "type" => ProjectPublicEdgeKind::Type,
                "reference-type" => ProjectPublicEdgeKind::ReferenceType,
                "reference" => ProjectPublicEdgeKind::Reference,
                other => panic!("unknown edge {other}"),
            },
            location: base.source_maps[0].location,
        })
        .collect();
    ir
}
