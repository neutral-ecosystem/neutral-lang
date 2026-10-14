// SPDX-License-Identifier: Apache-2.0

//! Raw successor expression trees, not validated project or evaluation authority.
//!
//! Both branches and exact input/output contracts remain data. This module neither
//! activates capture/wire `/3` nor resolves bindings, evaluates values or schedules effects.

use crate::{
    ModuleSymbolIdentity, composition::BindingValue, project_interface::ProjectPublicType,
};

pub mod attribution;
mod fallible;
mod inspection;
pub mod profile;
pub mod project;
pub mod templates;
pub use fallible::try_clone_value_bounded;
pub use inspection::{
    ExpressionInspection, ExpressionShape, ExpressionShapeError, inspect_expression_retention,
    inspect_expression_shape, inspect_expression_type_retention,
    inspect_expression_value_retention, inspect_public_template_retention,
};

/// A binding body; external input is distinct from typed null and absent definitions.
#[derive(Debug, Eq, PartialEq)]
pub enum ExpressionBody {
    /// Fully materialized immutable value.
    Value(BindingValue),
    /// Externally supplied value, with no initializer or fabricated default.
    Input,
    /// Retained typed expression, not an instruction to evaluate it.
    Expression(Expression),
}

/// Raw resolved-result annotation; only the independent reader can establish its truth.
#[derive(Debug, Eq, PartialEq)]
pub struct Expression {
    /// Exact invariant result type, including nominal ownership and wrappers.
    pub ty: ProjectPublicType,
    /// Closed expression structure in evaluation order.
    pub node: ExpressionNode,
}

/// Owned typed selector data; never implicitly unwraps null or follows a reference.
#[derive(Debug, Eq, PartialEq)]
pub enum SelectorSegment {
    /// Exact record member.
    Field(String),
    /// Zero-based future list position; availability is not established here.
    Index(u32),
    /// Closed variant payload; selection is not established here.
    Alternative(String),
}

/// One match arm; canonical wire order is tag order, not source spelling order.
#[derive(Debug, Eq, PartialEq)]
pub struct MatchArm {
    /// Exact alternative tag.
    pub tag: String,
    /// Declared payload type for lexical slot zero.
    pub payload_type: ProjectPublicType,
    /// Arm body under its additional lexical frame.
    pub body: Expression,
}

/// Frozen unary operations; no executable string or callback dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperator {
    /// Boolean negation.
    Not,
    /// Exact numeric negation.
    Negate,
}

/// Frozen binary operations, preserving operand order and decimal semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperator {
    /// Exact multiplication.
    Multiply,
    /// Profile-governed decimal division.
    Divide,
    /// Exact addition.
    Add,
    /// Exact subtraction.
    Subtract,
    /// String or homogeneous list construction.
    Concat,
    /// Ordered comparison.
    Less,
    /// Inclusive ordered comparison.
    LessEqual,
    /// Ordered comparison.
    Greater,
    /// Inclusive ordered comparison.
    GreaterEqual,
    /// Exact typed structural equality.
    Equal,
    /// Exact typed structural inequality.
    NotEqual,
    /// Short-circuit conjunction.
    And,
    /// Short-circuit disjunction.
    Or,
}

/// Every frozen residual node; construction alone establishes no type/visibility authority.
#[derive(Debug, Eq, PartialEq)]
pub enum ExpressionNode {
    /// Typed static value using unchanged composition value semantics.
    Value(BindingValue),
    /// Stable declared external input, never a host environment name.
    Input(ModuleSymbolIdentity),
    /// Potential output dependency; consumer must validate producer/result association.
    Output {
        /// Existing public nominal binding.
        target: ModuleSymbolIdentity,
        /// Exact public nominal result contract.
        result_type: ProjectPublicType,
        /// Ordered type-mode selector with unresolved availability obligations.
        selector: Vec<SelectorSegment>,
    },
    /// Prefix operation.
    Unary {
        /// Closed operation.
        operator: UnaryOperator,
        /// Operand, never evaluated by inspection.
        operand: Box<Expression>,
    },
    /// Ordered binary operation.
    Binary {
        /// Closed operation.
        operator: BinaryOperator,
        /// First operand.
        left: Box<Expression>,
        /// Second operand.
        right: Box<Expression>,
    },
    /// Guarded selection; both branches remain retained and type-checkable.
    If {
        /// Boolean guard.
        guard: Box<Expression>,
        /// True path.
        then_branch: Box<Expression>,
        /// False path.
        else_branch: Box<Expression>,
    },
    /// Exhaustive closed-variant selection with arm-local payload scope.
    Match {
        /// Variant expression.
        scrutinee: Box<Expression>,
        /// Canonical tag-ordered arms.
        arms: Vec<MatchArm>,
    },
    /// Explicit scalar text conversion, not a general callable function.
    Text(Box<Expression>),
    /// Ordered string expressions; no shell/backend escaping is implied.
    Interpolate(Vec<Expression>),
    /// Ordered homogeneous expressions.
    List(Vec<Expression>),
    /// Canonical fields; absence means omitted, never a fabricated null.
    Record(Vec<(String, Option<Expression>)>),
    /// Exact closed alternative and its typed payload.
    Variant {
        /// Alternative tag.
        tag: String,
        /// Payload expression.
        payload: Box<Expression>,
    },
    /// Lexical reference, not an external binding key; scope validation is reader-owned.
    Bound {
        /// Zero-based enclosing frame depth.
        depth: u64,
        /// Zero-based declared parameter or payload slot.
        slot: u64,
    },
}

impl ExpressionNode {
    /// Returns the exact frozen wire/transcript node tag, never a host dispatch name.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Value(_) => "value",
            Self::Input(_) => "input",
            Self::Output { .. } => "output",
            Self::Unary { .. } => "unary",
            Self::Binary { .. } => "binary",
            Self::If { .. } => "if",
            Self::Match { .. } => "match",
            Self::Text(_) => "text",
            Self::Interpolate(_) => "interpolate",
            Self::List(_) => "list",
            Self::Record(_) => "record",
            Self::Variant { .. } => "variant",
            Self::Bound { .. } => "bound",
        }
    }
}
impl UnaryOperator {
    /// Returns the exact frozen operator token.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Not => "!",
            Self::Negate => "-",
        }
    }
}
impl BinaryOperator {
    /// Returns the exact frozen operator token without relying on Debug spelling.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Concat => "++",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }
}
