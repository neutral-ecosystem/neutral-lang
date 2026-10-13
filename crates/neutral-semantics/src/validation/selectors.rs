// SPDX-License-Identifier: Apache-2.0

//! Independent selectors checks over retained expression authority.

use super::{B, E, S, State};
use crate::model::AvailabilityRequirement as A;
use neutral_ir::composition::FieldPresence;
use neutral_ir::expression::SelectorSegment;
use neutral_ir::{
    ModuleSymbolIdentity, expression::Expression, project_interface::ProjectPublicType as T,
};

impl<'a> State<'a> {
    /// Checks retained selectors semantics without evaluating values.
    pub(super) fn output(
        &mut self,
        e: &'a Expression,
        target: &ModuleSymbolIdentity,
        result_type: &'a T,
        selector: &'a [SelectorSegment],
        owner: &ModuleSymbolIdentity,
    ) -> Result<(), E> {
        self.edge()?;
        self.facts.output_leaves += 1;
        self.wire_symbol(target)?;
        self.ty(result_type, true, owner, 0)?;
        self.wire_type(result_type)?;
        let d = self.binding(target, true, owner)?;
        if !d.public
            || !matches!(
                &d.signature,
                S::Binding(T::Nominal(_) | T::VocabularyNominal { .. })
            )
        {
            return Err(E::UnknownOrPrivate);
        }
        self.nominal(result_type, true, owner)?;
        let mut ty = result_type;
        for (index, segment) in selector.iter().enumerate() {
            match (segment, ty) {
                (SelectorSegment::Field(name), T::Nominal(_) | T::VocabularyNominal { .. }) => {
                    self.wire_text("field")?;
                    self.wire_text(name)?;
                    let B::Record(fields) = self.nominal(ty, true, owner)? else {
                        return Err(E::Restriction);
                    };
                    self.budget.step(fields.len() as u64 + 1)?;
                    let field = fields
                        .iter()
                        .find(|f| &f.name == name)
                        .ok_or(E::Restriction)?;
                    if field.presence == FieldPresence::Optional {
                        self.available(owner, index, A::Presence)?;
                    }
                    ty = &field.ty;
                }
                (SelectorSegment::Index(_), T::List(inner)) => {
                    self.wire_text("index")?;
                    self.available(owner, index, A::Index)?;
                    ty = inner;
                }
                (
                    SelectorSegment::Alternative(tag),
                    T::Nominal(_) | T::VocabularyNominal { .. },
                ) => {
                    self.wire_text("alternative")?;
                    self.wire_text(tag)?;
                    let B::Variant(alternatives) = self.nominal(ty, true, owner)? else {
                        return Err(E::Restriction);
                    };
                    self.budget.step(alternatives.len() as u64 + 1)?;
                    let alternative = alternatives
                        .iter()
                        .find(|a| &a.tag == tag)
                        .ok_or(E::Restriction)?;
                    self.available(owner, index, A::Alternative)?;
                    ty = &alternative.ty;
                }
                _ => return Err(E::Restriction),
            }
        }
        if ty != &e.ty {
            return Err(E::TypeMismatch);
        }
        Ok(())
    }
}
