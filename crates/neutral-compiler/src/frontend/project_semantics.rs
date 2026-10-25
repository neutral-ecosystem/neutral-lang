// SPDX-License-Identifier: Apache-2.0

//! Current captured-project composition and expression compilation entry points.

mod composition;
mod type_syntax;

pub use composition::{
    ExpressionCompilationCache, ExpressionCompileFailure, ProjectCacheLimits, ProjectCacheStats,
    compile_expression_project, format_expression_source,
};
pub(super) use type_syntax::parse_complete_type_fallible;

use neutral_core::{ByteSpan, allocation::TryClone};

/// One parsed nominal type expression and its source occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum TypeExpr {
    /// Exact core numeric type.
    Num,
    /// Exact core string type.
    String,
    /// Exact core Boolean type.
    Bool,
    /// Inert URL text type.
    Url,
    /// Inert path text type.
    Path,
    /// Local or alias-qualified nominal type and its source occurrence.
    Nominal(Option<String>, String, ByteSpan),
    /// Invariant ordered-list type.
    List(Box<Self>),
    /// Invariant typed reference target.
    Ref(Box<Self>),
    /// One outer nullability layer.
    Nullable(Box<Self>),
}

impl TryClone for TypeExpr {
    /// Copies bounded syntax types and every recursive wrapper fallibly for cache retention.
    fn try_clone(&self) -> Result<Self, neutral_core::allocation::AllocationError> {
        Ok(match self {
            Self::Num => Self::Num,
            Self::String => Self::String,
            Self::Bool => Self::Bool,
            Self::Url => Self::Url,
            Self::Path => Self::Path,
            Self::Nominal(alias, name, span) => {
                Self::Nominal(alias.try_clone()?, name.try_clone()?, *span)
            }
            Self::List(inner) => Self::List(inner.try_clone()?),
            Self::Ref(inner) => Self::Ref(inner.try_clone()?),
            Self::Nullable(inner) => Self::Nullable(inner.try_clone()?),
        })
    }
}
