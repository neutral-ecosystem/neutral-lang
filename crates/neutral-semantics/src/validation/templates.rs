// SPDX-License-Identifier: Apache-2.0

//! Definition-scoped public template validation; no substitution, folding or invocation.

use super::{E, ExpressionRoot, Frame, State, shape_error};
use neutral_ir::{
    expression::inspect_public_template_retention,
    language::{is_protected_name, is_snake_name, is_upper_name},
};

impl State<'_> {
    /// Checks canonical owners and every parameter/default/body with one request budget.
    pub(super) fn templates(&mut self) -> Result<(), E> {
        if self.ir.public_templates.len() as u64 > self.limits.expansion.definitions {
            return Err(E::Limit);
        }
        if self.ir.public_templates.is_empty() {
            return Ok(());
        }
        // All templates/defaults share expansion controls; no per-definition budget reset.
        self.budget.work = self.budget.work.min(self.limits.expansion.work);
        self.budget.bytes = self
            .budget
            .bytes
            .min(self.limits.expansion.retained_bytes)
            .min(self.limits.expansion.output_bytes);
        let mut previous = None;
        for template in &self.ir.public_templates {
            self.budget.symbol(&template.identity)?;
            let owner = &template.identity;
            self.budget.lookup(
                (owner.module().module_name().len()
                    + owner.module().language_behavior_version().len()
                    + 1) as u64,
                self.ir.modules.len(),
            )?;
            self.budget.lookup(
                (owner.module().module_name().len() + owner.declaration_name().len() + 1) as u64,
                self.ir.declarations.len(),
            )?;
            if previous.is_some_and(|p| p >= owner)
                || !is_upper_name(owner.declaration_name())
                || is_protected_name(owner.declaration_name())
                || self
                    .ir
                    .modules
                    .binary_search_by(|m| m.identity.cmp(owner.module()))
                    .is_err()
                || self
                    .ir
                    .declarations
                    .binary_search_by(|d| d.identity.cmp(owner))
                    .is_ok()
            {
                return Err(E::Malformed);
            }
            previous = Some(owner);
            let cost = inspect_public_template_retention(
                template,
                self.retention_limits(),
                self.limits.expansion,
                self.budget.cancel,
            )
            .map_err(shape_error)?;
            self.budget.step(cost.work)?;
            self.budget.storage(cost.retained_bytes)?;
            self.ty(&template.result_type, true, owner, 0)?;
            let mut defaulted = false;
            for (index, parameter) in template.parameters.iter().enumerate() {
                self.budget.text(&parameter.name)?;
                self.budget.step(
                    (parameter.name.len() as u64 + 1)
                        .checked_mul(index as u64 + 1)
                        .ok_or(E::Limit)?,
                )?;
                if !is_snake_name(&parameter.name)
                    || is_protected_name(&parameter.name)
                    || template.parameters[..index]
                        .iter()
                        .any(|p| p.name == parameter.name)
                    || (defaulted && parameter.default.is_none())
                {
                    return Err(E::Malformed);
                }
                self.ty(&parameter.ty, true, owner, 0)?;
                if let Some(default) = &parameter.default {
                    defaulted = true;
                    self.frames[0] = Some(Frame::Parameters(&template.parameters[..index]));
                    self.frame_count = 1;
                    self.root = ExpressionRoot::TemplateDefault(index);
                    self.tree(default, true, owner, 1)?;
                    if default.ty != parameter.ty {
                        return Err(E::TypeMismatch);
                    }
                }
            }
            self.frames[0] = Some(Frame::Parameters(&template.parameters));
            self.frame_count = 1;
            self.root = ExpressionRoot::TemplateBody;
            self.tree(&template.body, true, owner, 1)?;
            if template.body.ty != template.result_type {
                return Err(E::TypeMismatch);
            }
            self.frames[0] = None;
            self.frame_count = 0;
            self.facts.public_templates += 1;
        }
        self.root = ExpressionRoot::Binding;
        Ok(())
    }
}
