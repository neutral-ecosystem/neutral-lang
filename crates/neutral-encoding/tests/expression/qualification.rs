// SPDX-License-Identifier: Apache-2.0

//! Literal tree framing and independent complete-artifact retention controls.

use super::*;

/// Compares both directions against hand-encoded restricted CBOR, not the production writer.
fn literal(tree: &Expression, parts: &[&[u8]]) {
    let expected = parts.concat();
    let mut writer = CborWriter::new();
    trees::write_tree(&mut writer, tree, 1).unwrap();
    assert_eq!(writer.finish().unwrap(), expected);
    let parsed = crate::decoder::parse_canonical_section(
        &expected,
        0,
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(
        &trees::read_tree(&parsed, 1, &CancellationToken::new()).unwrap(),
        tree
    );
}

/// Literal Boolean type and true-value tree shared only by independently hand-written byte oracles.
const BOOL: &[u8] = b"\x81\x64bool";
/// `["value", ["bool"], ["bool", true]]` under the frozen tree grammar.
const TRUE: &[u8] = b"\x83\x65value\x81\x64bool\x82\x64bool\xf5";
/// `[["1.0", "m"], "x"]`, separate from the production symbol encoder.
const SYMBOL: &[u8] = b"\x82\x82\x631.0\x61m\x61x";

/// Catches field swaps or dropped selector segments even if writer and reader agree on the same bug.
#[test]
fn literal_value_input_output_and_bound_nodes_preserve_exact_positions() {
    literal(&leaf(), &[TRUE]);
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::Input(symbol()),
        },
        &[b"\x83\x65input", BOOL, SYMBOL],
    );
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::Bound { depth: 1, slot: 2 },
        },
        &[b"\x84\x65bound", BOOL, b"\x01\x02"],
    );
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::Output {
                target: symbol(),
                result_type: T::Bool,
                selector: vec![
                    SelectorSegment::Field("f".into()),
                    SelectorSegment::Index(2),
                    SelectorSegment::Alternative("a".into()),
                ],
            },
        },
        &[
            b"\x85\x66output",
            BOOL,
            SYMBOL,
            BOOL,
            b"\x83\x82\x65field\x61f\x82\x65index\x02\x82\x6balternative\x61a",
        ],
    );
}

/// Every operator token has a literal CBOR width; shared spelling helpers cannot supply expectations.
#[test]
fn literal_unary_and_binary_operator_bytes_are_closed() {
    for (operator, token) in [
        (UnaryOperator::Not, b"\x61!"),
        (UnaryOperator::Negate, b"\x61-"),
    ] {
        literal(
            &Expression {
                ty: T::Bool,
                node: ExpressionNode::Unary {
                    operator,
                    operand: Box::new(leaf()),
                },
            },
            &[b"\x84\x65unary", BOOL, token, TRUE],
        );
    }
    for (operator, token) in [
        (BinaryOperator::Multiply, b"\x61*".as_slice()),
        (BinaryOperator::Divide, b"\x61/"),
        (BinaryOperator::Add, b"\x61+"),
        (BinaryOperator::Subtract, b"\x61-"),
        (BinaryOperator::Concat, b"\x62++"),
        (BinaryOperator::Less, b"\x61<"),
        (BinaryOperator::LessEqual, b"\x62<="),
        (BinaryOperator::Greater, b"\x61>"),
        (BinaryOperator::GreaterEqual, b"\x62>="),
        (BinaryOperator::Equal, b"\x62=="),
        (BinaryOperator::NotEqual, b"\x62!="),
        (BinaryOperator::And, b"\x62&&"),
        (BinaryOperator::Or, b"\x62||"),
    ] {
        literal(
            &Expression {
                ty: T::Bool,
                node: ExpressionNode::Binary {
                    operator,
                    left: Box::new(leaf()),
                    right: Box::new(leaf()),
                },
            },
            &[b"\x85\x66binary", BOOL, token, TRUE, TRUE],
        );
    }
}

/// Catches lost arms, omitted-field sentinels, list ordering and changed constructor tuple widths.
#[test]
fn literal_branch_and_constructor_nodes_retain_all_children() {
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::If {
                guard: Box::new(leaf()),
                then_branch: Box::new(leaf()),
                else_branch: Box::new(leaf()),
            },
        },
        &[b"\x85\x62if", BOOL, TRUE, TRUE, TRUE],
    );
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::Match {
                scrutinee: Box::new(leaf()),
                arms: vec![MatchArm {
                    tag: "a".into(),
                    payload_type: T::Bool,
                    body: leaf(),
                }],
            },
        },
        &[b"\x84\x65match", BOOL, TRUE, b"\x81\x83\x61a", BOOL, TRUE],
    );
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::Text(Box::new(leaf())),
        },
        &[b"\x83\x64text", BOOL, TRUE],
    );
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::Interpolate(vec![leaf()]),
        },
        &[b"\x83\x6binterpolate", BOOL, b"\x81", TRUE],
    );
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::List(vec![leaf()]),
        },
        &[b"\x83\x64list", BOOL, b"\x81", TRUE],
    );
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::Record(vec![("a".into(), Some(leaf())), ("b".into(), None)]),
        },
        &[
            b"\x83\x66record",
            BOOL,
            b"\x82\x82\x61a",
            TRUE,
            b"\x82\x61b\x81\x67omitted",
        ],
    );
    literal(
        &Expression {
            ty: T::Bool,
            node: ExpressionNode::Variant {
                tag: "a".into(),
                payload: Box::new(leaf()),
            },
        },
        &[b"\x84\x67variant", BOOL, b"\x61a", TRUE],
    );
}

/// Registered fragment bytes are compared without depending on the optional portable tree at runtime.
#[test]
fn registered_input_and_binary_fragments_match_frozen_bytes() {
    let owner = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new("1.0", "example::expressions"),
        "count",
    );
    let input = || Expression {
        ty: T::Num,
        node: ExpressionNode::Input(owner.clone()),
    };
    literal(
        &input(),
        &[&hex(
            "8365696e70757481636e756d828263312e30746578616d706c653a3a65787072657373696f6e7365636f756e74",
        )],
    );
    let value = Expression {
        ty: T::Num,
        node: ExpressionNode::Value(CompositionValue::Number(
            neutral_ir::ExactNumber::from_source("1", 8, 8).unwrap(),
        )),
    };
    literal(
        &Expression {
            ty: T::Num,
            node: ExpressionNode::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(input()),
                right: Box::new(value),
            },
        },
        &[&hex(
            "856662696e61727981636e756d612b8365696e70757481636e756d828263312e30746578616d706c653a3a65787072657373696f6e7365636f756e74836576616c756581636e756d84636e756df4613100",
        )],
    );
}

/// Replaces one canonical unsigned field without changing any unrelated tuple bytes.
fn replace_unsigned(bytes: &[u8], location: &crate::decoder::LocatedValue, value: u64) -> Vec<u8> {
    let start = usize::try_from(location.offset).unwrap();
    let width = match bytes[start] & 31 {
        0..=23 => 1,
        24 => 2,
        25 => 3,
        26 => 5,
        27 => 9,
        _ => panic!("unsigned fixture field"),
    };
    let mut writer = CborWriter::new();
    writer.unsigned(value).unwrap();
    let mut mutated = bytes.to_vec();
    mutated.splice(start..start + width, writer.finish().unwrap());
    mutated
}

/// Every retained producer limit is required, including controls not consumed until later evaluation.
#[test]
fn each_producer_control_rejects_zero_without_publishing_a_prefix() {
    let bytes = vectors().remove(2);
    let ir = raw(&bytes);
    let parsed = parse_written_ir(&ir);
    let sections = crate::project::array(&parsed).unwrap();
    let cancel = CancellationToken::new();
    for section in [9, 14, 18, 19] {
        for (index, value) in crate::project::array(&sections[section])
            .unwrap()
            .iter()
            .enumerate()
        {
            let mutated = replace_unsigned(&bytes, value, 0);
            assert_eq!(
                decode_expression_project(&mutated, DecodeLimits::hard(), limits(&ir), &cancel)
                    .unwrap_err()
                    .class(),
                C::EncodedSizeLimit,
                "section {section}, dimension {index}"
            );
        }
    }
    assert!(decode_expression_project(&bytes, DecodeLimits::hard(), limits(&ir), &cancel).is_ok());
}

/// Checks meaningful semantic thresholds against the literal residual/static/template projects.
#[test]
fn caller_retention_limits_are_inclusive_and_independent() {
    type Control = fn(&mut SemanticLimits) -> &mut u64;
    let cases: &[(usize, &str, u64, Control)] = &[
        (0, "project modules", 1, |p| &mut p.project.modules),
        (1, "project declarations", 3, |p| {
            &mut p.project.declarations
        }),
        (1, "expression bindings", 3, |p| &mut p.expression.bindings),
        (1, "residual nodes", 6, |p| &mut p.expression.nodes),
        (1, "syntax depth", 3, |p| &mut p.expression.syntax_depth),
        (1, "evaluation depth", 3, |p| {
            &mut p.expression.evaluation_depth
        }),
        (1, "potential edges", 2, |p| {
            &mut p.expression.dependency_edges
        }),
        (0, "coefficient digits", 3, |p| {
            &mut p.expression.coefficient_digits
        }),
        (0, "absolute scale", 3, |p| &mut p.expression.absolute_scale),
        (2, "template definitions", 1, |p| {
            &mut p.expansion.definitions
        }),
        (2, "template parameters", 1, |p| &mut p.expansion.parameters),
    ];
    let vectors = vectors();
    for &(case, name, exact, select) in cases {
        let bytes = &vectors[case];
        let ir = raw(bytes);
        let mut policy = limits(&ir);
        *select(&mut policy) = exact;
        let cancel = CancellationToken::new();
        let accepted = decode_expression_project(bytes, DecodeLimits::hard(), policy, &cancel)
            .unwrap_or_else(|error| panic!("{name} at {exact}: {error:?}"));
        *select(&mut policy) -= 1;
        assert_eq!(
            decode_expression_project(bytes, DecodeLimits::hard(), policy, &cancel)
                .unwrap_err()
                .class(),
            C::EncodedSizeLimit,
            "{name} one over"
        );
        assert_eq!(accepted.ir(), &ir);
        assert!(
            decode_expression_project(bytes, DecodeLimits::hard(), limits(&ir), &cancel).is_ok()
        );
    }
}

/// Budget costs depend on target layouts, so find their boundary without hard-coding `size_of` values.
#[test]
fn storage_and_work_controls_reject_one_over_for_both_producer_and_caller() {
    type Control = fn(&mut SemanticLimits) -> &mut u64;
    let controls: &[(&str, Control)] = &[
        ("project text", |p| &mut p.project.text_bytes),
        ("expression work", |p| &mut p.expression.work),
        ("expression retention", |p| &mut p.expression.retained_bytes),
        ("composition work", |p| &mut p.composition.work),
        ("expansion work", |p| &mut p.expansion.work),
        ("expansion retention", |p| &mut p.expansion.retained_bytes),
        ("expansion output", |p| &mut p.expansion.output_bytes),
    ];
    let bytes = vectors().remove(2);
    let ir = raw(&bytes);
    let cancel = CancellationToken::new();
    for &(name, select) in controls {
        let mut policy = limits(&ir);
        let (mut lower, mut upper) = (1, *select(&mut policy));
        while lower < upper {
            let candidate = lower + (upper - lower) / 2;
            *select(&mut policy) = candidate;
            if decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel).is_ok() {
                upper = candidate;
            } else {
                lower = candidate + 1;
            }
        }
        assert!(upper > 1, "{name} must constrain actual retained data/work");
        *select(&mut policy) = upper;
        assert!(
            decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel).is_ok(),
            "{name}"
        );
        let mut producer = raw(&bytes);
        producer.limits = policy.project;
        producer.composition_limits = policy.composition.policy();
        producer.expression_limits = policy.expression;
        producer.expansion_limits = policy.expansion;
        assert!(
            decode_expression_project(
                &forge(&producer),
                DecodeLimits::hard(),
                limits(&ir),
                &cancel
            )
            .is_ok(),
            "producer {name}"
        );
        *select(&mut policy) -= 1;
        assert_eq!(
            decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel)
                .unwrap_err()
                .class(),
            C::EncodedSizeLimit,
            "caller {name}"
        );
        producer.limits = policy.project;
        producer.composition_limits = policy.composition.policy();
        producer.expression_limits = policy.expression;
        producer.expansion_limits = policy.expansion;
        assert_eq!(
            decode_expression_project(
                &forge(&producer),
                DecodeLimits::hard(),
                limits(&ir),
                &cancel
            )
            .unwrap_err()
            .class(),
            C::EncodedSizeLimit,
            "producer {name}"
        );
    }
}

/// A caller cannot silently default any of the frozen positive controls, even unused expansion axes.
#[test]
fn every_caller_control_is_independently_required() {
    type Control = fn(&mut SemanticLimits) -> &mut u64;
    macro_rules! fields {
        ($($family:ident { $($field:ident),* $(,)? }),* $(,)?) => {
            &[$($( (concat!(stringify!($family), ".", stringify!($field)),
                (|p: &mut SemanticLimits| &mut p.$family.$field) as Control), )*)*]
        };
    }
    let controls: &[(&str, Control)] = fields!(
        project {
            modules,
            declarations,
            import_edges,
            nodes,
            text_bytes,
            artifact_bytes
        },
        composition {
            bundles,
            captured_bytes,
            dependencies_per_bundle,
            dependency_edges,
            dependency_depth,
            total_types,
            total_fields,
            alternatives_per_type,
            total_alternatives,
            choices_per_field,
            total_choices,
            type_depth,
            value_depth,
            value_nodes,
            work
        },
        expression {
            nodes,
            syntax_depth,
            evaluation_depth,
            bindings,
            dependency_edges,
            work,
            retained_bytes,
            output_bytes,
            coefficient_digits,
            absolute_scale,
            diagnostics
        },
        expansion {
            definitions,
            applications,
            parameters,
            axes,
            axis_items,
            product_tuples,
            aggregate_tuples,
            expanded_bindings,
            expanded_nodes,
            depth,
            work,
            retained_bytes,
            output_bytes
        },
    );
    let bytes = vectors().remove(2);
    let ir = raw(&bytes);
    let cancel = CancellationToken::new();
    for &(name, select) in controls {
        let mut policy = limits(&ir);
        *select(&mut policy) = 0;
        assert_eq!(
            decode_expression_project(&bytes, DecodeLimits::hard(), policy, &cancel)
                .unwrap_err()
                .class(),
            C::EncodedSizeLimit,
            "{name}"
        );
    }
}
