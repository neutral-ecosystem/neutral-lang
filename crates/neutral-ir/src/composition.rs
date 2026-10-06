// SPDX-License-Identifier: Apache-2.0

//! Shared raw vocabulary-composition contracts for producers and independent readers.
//!
//! These structures are not validated artifacts. The vocabulary boundary checks
//! their captured representation before publishing an immutable catalogue. They
//! do not extend the frozen project IR or its encoding/identity profiles.

use crate::{
    ExactNumber, ModuleSymbolIdentity, VocabularyIdentity, project_interface::ProjectPublicType,
};
use std::cmp::Ordering;

pub mod profile;

/// Whether omission rejects, remains absent, or materializes a closed default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldPresence {
    /// Every initializer must explicitly supply the field.
    Required,
    /// An omitted field remains absent, distinct from explicit null.
    Optional,
    /// Omission materializes the field's validated closed default.
    Defaulted,
}

/// One bounded, closed value with no reference, expression, or execution channel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosedValue {
    /// Normalized exact decimal number.
    Number(ExactNumber),
    /// Exact Unicode scalar sequence.
    String(String),
    /// Boolean scalar.
    Bool(bool),
    /// Inert URL text, never normalized or opened.
    Url(String),
    /// Inert path text, never resolved or opened.
    Path(String),
    /// Explicit null, permitted only by nullable typing.
    Null,
    /// Ordered contextual list elements.
    List(Vec<Self>),
    /// Canonically named fields; `None` represents an omitted optional field.
    Record(Vec<(String, Option<Self>)>),
    /// Exactly one selected tag and contextually typed payload.
    Variant {
        /// Exact alternative tag.
        tag: String,
        /// Payload checked against that tag's alternative type.
        payload: Box<Self>,
    },
}

/// A schema-checked location inside a closed value, never a host/source path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValuePathSegment {
    /// Canonical record field name.
    Field(String),
    /// Zero-based ordered list element.
    Element(u64),
    /// Payload of the selected closed variant tag.
    Payload,
}

/// How an occurrence entered the final value, separate from its logical meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueOriginKind {
    /// Explicitly supplied non-null value.
    Supplied,
    /// Explicit null at a nullable position.
    ExplicitNull,
    /// Absent optional record field, not a null scalar.
    OmittedOptional,
    /// Omission materialized a closed contract default, including its children.
    Defaulted,
}

/// Safe occurrence classification without fabricated source coordinates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueOrigin {
    /// Canonical field/list/payload path from the value root.
    pub path: Vec<ValuePathSegment>,
    /// Supplied/null/absent/default classification.
    pub kind: ValueOriginKind,
}

/// Closed scalar/length restrictions, never executable predicates.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FieldRestrictions {
    /// Sorted finite non-null scalar choices, or no finite-choice restriction.
    pub choices: Option<Vec<ClosedValue>>,
    /// Inclusive exact numeric lower bound.
    pub minimum: Option<ExactNumber>,
    /// Inclusive exact numeric upper bound.
    pub maximum: Option<ExactNumber>,
    /// Inclusive Unicode-scalar or immediate-list-element lower bound.
    pub min_length: Option<u64>,
    /// Inclusive Unicode-scalar or immediate-list-element upper bound.
    pub max_length: Option<u64>,
}

/// One field's complete semantic contract, including unused defaults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionField {
    /// Canonical field name.
    pub name: String,
    /// Resolved alias-independent type; nominal ownership distinguishes variants/records.
    pub ty: ProjectPublicType,
    /// Explicit omission policy.
    pub presence: FieldPresence,
    /// Complete declarative restrictions.
    pub restrictions: FieldRestrictions,
    /// Materialized closed default, present exactly for `Defaulted` fields.
    pub default: Option<ClosedValue>,
}

/// One closed tag and its complete payload type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionAlternative {
    /// Exact case-sensitive alternative tag.
    pub tag: String,
    /// Resolved alias-independent payload type.
    pub ty: ProjectPublicType,
}

/// A nominal record or tagged variant; kinds share one catalogue namespace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionBody {
    /// Fields in canonical name order.
    Record(Vec<CompositionField>),
    /// Nonempty alternatives in canonical tag order.
    Variant(Vec<CompositionAlternative>),
}

/// One raw nominal definition; validation includes all public/private alternatives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionDefinition {
    /// Unique uppercase-leading name within its canonical owner.
    pub name: String,
    /// Whether source and external bundles may name the type.
    pub public: bool,
    /// Complete record or variant contract.
    pub body: CompositionBody,
}

/// A source-owned raw record or variant using the same contracts as vocabulary declarations.
///
/// Construction does not confer validity or activate a project profile. The shared
/// composition scope checks ownership, all payload branches, defaults and closure
/// before independent readers may inspect these definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCompositionDefinition {
    /// Exact module-symbol owner, never a source alias, host path or graph-local ID.
    pub owner: ModuleSymbolIdentity,
    /// Common record/variant body; its name must equal the owner's declaration name.
    pub definition: CompositionDefinition,
}

/// A direct canonical bundle dependency, never a source-local alias or locator.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompositionDependency {
    /// Canonical vocabulary identity.
    pub identity: String,
    /// Exact semantic revision.
    pub version: String,
}

/// Complete raw vocabulary contract with captured facts retained separately from meaning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionBundle {
    /// Exact schema/encoding/features, captured digest, canonical identity and revision.
    pub identity: VocabularyIdentity,
    /// Canonically ordered direct dependency requirements.
    pub dependencies: Vec<CompositionDependency>,
    /// Complete public/private nominal definitions in canonical name order.
    pub definitions: Vec<CompositionDefinition>,
}

/// Compares normalized decimal values without floating point or exponent-sized expansion.
///
/// Work is bounded by the longer coefficient. Wide signed decimal positions
/// avoid overflow even for extreme valid scales; virtual zero padding avoids
/// constructing an enormous integer when comparing tiny/large magnitudes.
#[must_use]
pub fn compare_exact_numbers(left: &ExactNumber, right: &ExactNumber) -> Ordering {
    if left == right {
        return Ordering::Equal;
    }
    if left.is_negative() != right.is_negative() {
        return right.is_negative().cmp(&left.is_negative());
    }
    let magnitude = if left.coefficient() == "0" {
        Ordering::Less
    } else if right.coefficient() == "0" {
        Ordering::Greater
    } else {
        let left_position = left.coefficient().len() as i128 + i128::from(left.scale());
        let right_position = right.coefficient().len() as i128 + i128::from(right.scale());
        left_position.cmp(&right_position).then_with(|| {
            let length = left.coefficient().len().max(right.coefficient().len());
            left.coefficient()
                .bytes()
                .chain(std::iter::repeat(b'0'))
                .take(length)
                .cmp(
                    right
                        .coefficient()
                        .bytes()
                        .chain(std::iter::repeat(b'0'))
                        .take(length),
                )
        })
    };
    if left.is_negative() {
        magnitude.reverse()
    } else {
        magnitude
    }
}
