// SPDX-License-Identifier: Apache-2.0

//! Independent matching checks over retained expression authority.

use super::{B, E, State};
use neutral_ir::expression::MatchArm;
use neutral_ir::{ModuleSymbolIdentity, expression::Expression};

impl<'a> State<'a> {
    /// Checks retained matching semantics without evaluating values.
    pub(super) fn matching(
        &mut self,
        e: &'a Expression,
        scrutinee: &'a Expression,
        arms: &'a [MatchArm],
        public: bool,
        owner: &ModuleSymbolIdentity,
        depth: u64,
    ) -> Result<(), E> {
        self.child(scrutinee, 0, public, owner, depth + 1)?;
        let B::Variant(alternatives) = self.nominal(&scrutinee.ty, public, owner)? else {
            return Err(E::TypeMismatch);
        };
        if arms.len() != alternatives.len() {
            return Err(E::MatchAlternatives);
        }
        for (index, (arm, alternative)) in arms.iter().zip(alternatives).enumerate() {
            self.wire_text(&arm.tag)?;
            self.ty(&arm.payload_type, public, owner, 0)?;
            self.wire_type(&arm.payload_type)?;
            if arm.tag != alternative.tag || arm.payload_type != alternative.ty {
                return Err(E::MatchAlternatives);
            }
            if self.frame_count >= self.frames.len() {
                return Err(E::Limit);
            }
            self.frames[self.frame_count] = Some(&arm.payload_type);
            self.frame_count += 1;
            let result = self.child(&arm.body, index as u64 + 1, public, owner, depth + 1);
            self.frame_count -= 1;
            self.frames[self.frame_count] = None;
            result?;
            if arm.body.ty != e.ty {
                return Err(E::TypeMismatch);
            }
        }
        Ok(())
    }
}
