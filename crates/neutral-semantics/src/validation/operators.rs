// SPDX-License-Identifier: Apache-2.0

//! Independent operators checks over retained expression authority.

use super::{E, State};
use neutral_ir::expression::BinaryOperator as O;
use neutral_ir::expression::UnaryOperator as U;
use neutral_ir::{
    ModuleSymbolIdentity, expression::Expression, project_interface::ProjectPublicType as T,
};

impl<'a> State<'a> {
    /// Checks a prefix operation without applying it.
    pub(super) fn unary(
        &mut self,
        e: &'a Expression,
        operator: U,
        operand: &'a Expression,
        public: bool,
        owner: &ModuleSymbolIdentity,
        depth: u64,
    ) -> Result<(), E> {
        self.wire_text(operator.spelling())?;
        self.child(operand, 0, public, owner, depth + 1)?;
        let expected = if operator == U::Not {
            &T::Bool
        } else {
            &T::Num
        };
        if &operand.ty != expected || &e.ty != expected {
            return Err(E::TypeMismatch);
        }
        Ok(())
    }

    /// Checks retained operators semantics without evaluating values.
    pub(super) fn binary(
        &mut self,
        e: &'a Expression,
        operator: O,
        operands: (&'a Expression, &'a Expression),
        public: bool,
        owner: &ModuleSymbolIdentity,
        depth: u64,
    ) -> Result<(), E> {
        let (left, right) = operands;
        self.wire_text(operator.spelling())?;
        self.child(left, 0, public, owner, depth + 1)?;
        self.child(right, 1, public, owner, depth + 1)?;
        if left.ty != right.ty {
            return Err(E::TypeMismatch);
        }
        let valid = match operator {
            O::Multiply | O::Divide | O::Add | O::Subtract => left.ty == T::Num && e.ty == T::Num,
            O::And | O::Or => left.ty == T::Bool && e.ty == T::Bool,
            O::Less | O::LessEqual | O::Greater | O::GreaterEqual => {
                matches!(left.ty, T::Num | T::String) && e.ty == T::Bool
            }
            O::Equal | O::NotEqual => e.ty == T::Bool,
            O::Concat => matches!(left.ty, T::String | T::List(_)) && e.ty == left.ty,
        };
        if valid { Ok(()) } else { Err(E::TypeMismatch) }
    }
}
