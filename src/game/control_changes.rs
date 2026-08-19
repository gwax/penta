//! Taking control of a permanent, for a turn or for as long as something
//! holds it.
//!
//! Both printed forms share a body; they differ only in what ends them, which
//! is recorded on the permanent and checked elsewhere -- cleanup for the
//! turn-scoped form, state-based actions for the held one.

use super::{
    ControlDurationDef, EffectRecipientDef, EffectResolutionContext, Game, GameObjectId, PlayerId,
    ScopedEffect, StackObject, Target,
};

impl Game {
    /// The shared body of both control-change durations.
    pub(super) fn take_control_of(
        &mut self,
        recipient: EffectRecipientDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
        duration: ControlDurationDef,
        controller: PlayerId,
    ) {
        let holder = match duration {
            ControlDurationDef::UntilEndOfTurn | ControlDurationDef::Indefinitely => None,
            ControlDurationDef::WhileSourceRemains { while_tapped } => {
                Some((object.source.unwrap_or(object.id), while_tapped))
            }
        };
        for target in self.effect_recipients(recipient, object, context, scoped) {
            let Target::Permanent(id) = target else {
                continue;
            };
            let Some(index) = self
                .battlefield
                .iter()
                .position(|permanent| permanent.card.id == id)
            else {
                continue;
            };
            if self.battlefield[index].controller == controller
                || self.cannot_change_controller(&self.battlefield[index])
            {
                continue;
            }
            let permanent = &mut self.battlefield[index];
            // Only the first change records where control came from, so
            // passing a permanent around and back still returns it to whoever
            // had it before the turn started. An indefinite change records
            // nothing: there is nothing for cleanup to give back, and an
            // earlier turn-scoped change over the same permanent still ends
            // the way it was going to.
            if duration != ControlDurationDef::Indefinitely {
                permanent
                    .control_reverts_to
                    .get_or_insert(permanent.controller);
            }
            permanent.controller = controller;
            permanent.control_source = holder.map(|(id, _)| id);
            permanent.control_requires_source_tapped = holder.is_some_and(|(_, tapped)| tapped);
            // It has not been under its new controller's control since their
            // turn began, so it is summoning sick unless something grants
            // haste. This is why the cards that steal a creature almost always
            // grant it too.
            permanent.entered_controller_turn = self.turns_started[controller.index()];
        }
    }
}

impl Game {
    /// Swap who controls two permanents. Both controllers are read before
    /// either moves: doing it as two ordinary control changes would let the
    /// first one change the answer the second needs.
    pub(super) fn exchange_control_of(
        &mut self,
        first: EffectRecipientDef,
        second: EffectRecipientDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let one = self.single_permanent_recipient(first, object, context, scoped);
        let other = self.single_permanent_recipient(second, object, context, scoped);
        let (Some(one), Some(other)) = (one, other) else {
            return;
        };
        if one == other {
            return;
        }
        let controllers = [one, other].map(|id| {
            self.battlefield
                .iter()
                .find(|permanent| permanent.card.id == id)
                .map(|permanent| permanent.controller)
        });
        let ([Some(one_controller), Some(other_controller)], false) = (
            controllers,
            [one, other].iter().any(|id| {
                self.battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == *id)
                    .is_some_and(|permanent| self.cannot_change_controller(permanent))
            }),
        ) else {
            return;
        };
        for (id, controller) in [(one, other_controller), (other, one_controller)] {
            let turns_started = self.turns_started[controller.index()];
            let Some(permanent) = self
                .battlefield
                .iter_mut()
                .find(|permanent| permanent.card.id == id)
            else {
                continue;
            };
            permanent.controller = controller;
            // An exchange lasts indefinitely, so nothing is recorded for
            // cleanup to give back. It is still a new controller who has not
            // had it since their turn began, which is what makes it
            // summoning sick.
            permanent.entered_controller_turn = turns_started;
        }
    }

    fn single_permanent_recipient(
        &self,
        recipient: EffectRecipientDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<GameObjectId> {
        let mut found = self
            .effect_recipients(recipient, object, context, scoped)
            .into_iter()
            .filter_map(|target| match target {
                Target::Permanent(id) => Some(id),
                Target::Card(_) | Target::Player(_) | Target::Spell(_) => None,
            });
        let first = found.next()?;
        found.next().is_none().then_some(first)
    }
}
