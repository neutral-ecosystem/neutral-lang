// SPDX-License-Identifier: Apache-2.0

//! Deep materialized literal accounting under the expression's shared preflight budget.

use super::{E, Inspection};
use crate::composition::BindingValue as V;

impl Inspection<'_> {
    /// Visits all dynamic value storage before any fallible value copy may begin.
    pub(super) fn value(&mut self, value: &V, depth: u64) -> Result<(), E> {
        self.structural_depth(depth)?;
        match value {
            V::Number(n) => {
                if n.coefficient().len() as u64 > self.limits.coefficient_digits
                    || n.scale().unsigned_abs() > self.limits.absolute_scale
                {
                    return Err(E::Limit);
                }
                self.text(n.coefficient())
            }
            V::String(s) | V::Url(s) | V::Path(s) => self.text(s),
            V::Bool(_) | V::Null => Ok(()),
            V::Reference(s) => self.symbol(s),
            V::List(values) => {
                self.storage::<V>(values.len())?;
                for v in values {
                    self.value(v, depth + 1)?;
                }
                Ok(())
            }
            V::Record(fields) => {
                self.storage::<(String, Option<V>)>(fields.len())?;
                let mut previous = None;
                for (name, value) in fields {
                    self.name(name, &mut previous)?;
                    if let Some(value) = value {
                        self.value(value, depth + 1)?;
                    }
                }
                Ok(())
            }
            V::Variant { tag, payload } => {
                self.text(tag)?;
                self.storage::<V>(1)?;
                self.value(payload, depth + 1)
            }
        }
    }
}
