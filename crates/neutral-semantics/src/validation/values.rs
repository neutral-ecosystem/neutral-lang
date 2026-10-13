// SPDX-License-Identifier: Apache-2.0

//! Complete materialized value checks against real contracts; no default synthesis or dereference.

use super::{B, E, S, State};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{BindingValue as V, FieldPresence, FieldRestrictions},
    language,
    project_interface::ProjectPublicType as T,
};

impl State<'_> {
    /// Retypes a retained complete value and enforces all actual field restrictions.
    pub(super) fn value(
        &mut self,
        value: &V,
        ty: &T,
        public: bool,
        owner: &ModuleSymbolIdentity,
        wire: bool,
        depth: u64,
    ) -> Result<(), E> {
        self.budget.step(1)?;
        if depth
            > self
                .limits
                .composition
                .value_depth
                .min(neutral_ir::project::PROJECT_MAX_DEPTH as u64)
        {
            return Err(E::Limit);
        }
        if wire {
            self.wire_text(match value {
                V::Number(_) => language::NUM,
                V::String(_) => language::STRING,
                V::Bool(_) => language::BOOL,
                V::Url(_) => language::URL,
                V::Path(_) => language::PATH,
                V::Null => language::NULL,
                V::List(_) => language::LIST,
                V::Record(_) => "record",
                V::Variant { .. } => "variant",
                V::Reference(_) => language::REF_TYPE,
            })?;
        }
        if let T::Nullable(inner) = ty {
            return if matches!(value, V::Null) {
                Ok(())
            } else {
                self.value_contents(value, inner, public, owner, wire, depth)
            };
        }
        self.value_contents(value, ty, public, owner, wire, depth)
    }
    /// Checks an already charged occurrence without double-counting nullable wire tags.
    fn value_contents(
        &mut self,
        value: &V,
        ty: &T,
        public: bool,
        owner: &ModuleSymbolIdentity,
        wire: bool,
        depth: u64,
    ) -> Result<(), E> {
        match (value, ty) {
            (V::Bool(_), T::Bool) => Ok(()),
            (V::Number(n), T::Num) => {
                if n.coefficient().len() as u64 > self.limits.expression.coefficient_digits
                    || n.scale().unsigned_abs() > self.limits.expression.absolute_scale
                {
                    return Err(E::Limit);
                }
                if wire {
                    self.wire_text(n.coefficient())
                } else {
                    self.budget.text(n.coefficient())
                }
            }
            (V::String(s), T::String) | (V::Url(s), T::Url) | (V::Path(s), T::Path) => {
                if wire {
                    self.wire_text(s)
                } else {
                    self.budget.text(s)
                }
            }
            (V::List(values), T::List(inner)) => {
                for v in values {
                    self.value(v, inner, public, owner, wire, depth + 1)?;
                }
                Ok(())
            }
            (V::Reference(target), T::Ref(inner)) => {
                if wire {
                    self.wire_symbol(target)?;
                }
                let target = self.binding(target, public, owner)?;
                if !matches!(&target.signature, S::Binding(t) if t == inner.as_ref()) {
                    return Err(E::TypeMismatch);
                }
                Ok(())
            }
            (V::Record(values), T::Nominal(_) | T::VocabularyNominal { .. }) => {
                let B::Record(fields) = self.nominal(ty, public, owner)? else {
                    return Err(E::TypeMismatch);
                };
                if fields.len() != values.len() {
                    return Err(E::TypeMismatch);
                }
                for (field, (name, value)) in fields.iter().zip(values) {
                    self.budget.text(name)?;
                    if name != &field.name {
                        return Err(E::TypeMismatch);
                    }
                    if wire {
                        self.wire_text(name)?;
                    }
                    match value {
                        Some(v) => {
                            self.value(v, &field.ty, public, owner, wire, depth + 1)?;
                            if field.restrictions != FieldRestrictions::default() {
                                neutral_vocabulary::composition::check_composition_field_restrictions_budgeted(
                                field,
                                v,
                                self.limits.composition,
                                &mut self.budget.work,
                                self.budget.cancel,
                            )
                            .map_err(E::from)?;
                            }
                        }
                        None if field.presence == FieldPresence::Optional => {
                            if wire {
                                self.wire_text("omitted")?;
                            }
                        }
                        _ => return Err(E::TypeMismatch),
                    }
                }
                Ok(())
            }
            (V::Variant { tag, payload }, T::Nominal(_) | T::VocabularyNominal { .. }) => {
                let B::Variant(alternatives) = self.nominal(ty, public, owner)? else {
                    return Err(E::TypeMismatch);
                };
                self.budget.text(tag)?;
                self.budget.step(alternatives.len() as u64 + 1)?;
                let alternative = alternatives
                    .iter()
                    .find(|a| &a.tag == tag)
                    .ok_or(E::MatchAlternatives)?;
                if wire {
                    self.wire_text(tag)?;
                }
                self.value(payload, &alternative.ty, public, owner, wire, depth + 1)
            }
            _ => Err(E::TypeMismatch),
        }
    }
}
