//! Leaving a pending decision without answering it, and resuming what it
//! suspended.
//!
//! Split out of the parent module for the source-size budget. What belongs
//! here is a decision that ends some way other than being answered: a cast
//! that stood in for the answer, a cancel, and the nested resolution that
//! picks up where the decision left off.

#![allow(clippy::wildcard_imports)]

use super::*;
use crate::ids::GameObjectId;

impl Game {
    /// Drops a standing "you may cast that card" offer because the card was
    /// cast. The offer is answered by the cast itself, so nothing about the
    /// decision resolves and the else branch never runs.
    pub(in crate::game) fn take_answered_cast_offer(&mut self, cast: GameObjectId) {
        let answered = self.pending_decisions.first().is_some_and(|pending| {
            matches!(
                pending.continuation,
                DecisionContinuation::MayCastExiled { card, .. } if card == cast
            )
        });
        if answered {
            self.pending_decisions.remove(0);
        }
    }

    pub(in crate::game) fn cancel_decision(&mut self, decision: u32) {
        debug_assert_eq!(self.pending_decisions[0].observation.id, decision);
        self.pending_decisions.remove(0);
    }

    pub(in crate::game) fn resolve_nested_effect_before_later(
        &mut self,
        effect: crate::game::ScopedEffect,
        object: &crate::game::StackObject,
        context: crate::game::EffectResolutionContext,
    ) {
        let mut later_procedures = std::mem::take(&mut self.pending_procedures);
        self.resolve_effect_def(effect, object, context);
        self.pending_procedures.append(&mut later_procedures);
    }
}
