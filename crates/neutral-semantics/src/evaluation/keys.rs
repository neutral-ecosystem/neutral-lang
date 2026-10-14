// SPDX-License-Identifier: Apache-2.0

//! Stable borrowed-key ordering after bounded envelope/type preflight.

use super::{BindingKey, SelectorSegment, T};
use std::cmp::Ordering;

/// Orders canonical input/output keys without allocating display strings or transcripts.
pub(super) fn compare(left: &BindingKey<'_>, right: &BindingKey<'_>) -> Ordering {
    match (left, right) {
        (BindingKey::Input(a), BindingKey::Input(b)) => a.cmp(b),
        (BindingKey::Input(_), BindingKey::Output { .. }) => Ordering::Less,
        (BindingKey::Output { .. }, BindingKey::Input(_)) => Ordering::Greater,
        (
            BindingKey::Output {
                target: a,
                result_type: at,
                selector: ap,
            },
            BindingKey::Output {
                target: b,
                result_type: bt,
                selector: bp,
            },
        ) => a
            .cmp(b)
            .then_with(|| ty(at, bt))
            .then_with(|| selectors(ap, bp)),
    }
}
/// Returns type tags in their frozen textual ordering; this is not nominal structural conversion.
fn tag(value: &T) -> &'static str {
    use neutral_ir::language;
    match value {
        T::Bool => language::BOOL,
        T::Num => language::NUM,
        T::String => language::STRING,
        T::Url => language::URL,
        T::Path => language::PATH,
        T::List(_) => language::LIST,
        T::Ref(_) => language::REF_TYPE,
        T::Nullable(_) => "nullable",
        T::Nominal(_) => "nominal",
        T::VocabularyNominal { .. } => "vocabulary",
    }
}
/// Compares already preflighted exact type trees, including canonical vocabulary revisions.
fn ty(left: &T, right: &T) -> Ordering {
    tag(left).cmp(tag(right)).then_with(|| match (left, right) {
        (T::List(a), T::List(b)) | (T::Ref(a), T::Ref(b)) | (T::Nullable(a), T::Nullable(b)) => {
            ty(a, b)
        }
        (T::Nominal(a), T::Nominal(b)) => a.cmp(b),
        (
            T::VocabularyNominal {
                identity: a,
                version: av,
                name: an,
            },
            T::VocabularyNominal {
                identity: b,
                version: bv,
                name: bn,
            },
        ) => (a, av, an).cmp(&(b, bv, bn)),
        _ => Ordering::Equal,
    })
}
/// Orders typed selector tuples by tag and exact member/index, then path length.
fn selectors(left: &[SelectorSegment], right: &[SelectorSegment]) -> Ordering {
    for (a, b) in left.iter().zip(right) {
        let order = segment_tag(a)
            .cmp(segment_tag(b))
            .then_with(|| match (a, b) {
                (SelectorSegment::Field(a), SelectorSegment::Field(b))
                | (SelectorSegment::Alternative(a), SelectorSegment::Alternative(b)) => a.cmp(b),
                (SelectorSegment::Index(a), SelectorSegment::Index(b)) => a.cmp(b),
                _ => Ordering::Equal,
            });
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}
/// Selector wire tags establish one stable total ordering without Debug formatting.
fn segment_tag(value: &SelectorSegment) -> &'static str {
    match value {
        SelectorSegment::Field(_) => "field",
        SelectorSegment::Index(_) => "index",
        SelectorSegment::Alternative(_) => "alternative",
    }
}
