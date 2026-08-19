//! Dealing an effect's damage, and reporting where it landed.

use super::super::{
    EffectRecipientDef, EffectResolutionContext, Game, ScopedEffect, StackObject, Target, ValueDef,
};

impl Game {
    /// Deals one effect's damage and reports the permanents that actually took
    /// some, in the order they were damaged.
    ///
    /// The report is what a "dealt damage this way" rider needs. A recipient
    /// can be named and still take nothing -- prevention, protection, a
    /// redirect that moves the damage to some other permanent -- and it can
    /// take damage without ever having been named, which is the other half of
    /// what redirection does. Only permanents are reported: no rider in the
    /// supported pool follows damage to a player, and a player is not a
    /// creature.
    pub(super) fn deal_effect_damage(
        &mut self,
        recipient: EffectRecipientDef,
        amount: ValueDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<Target> {
        // A divided total is chosen per target when the spell is
        // cast, so each one takes its own share rather than the same
        // amount as everyone else.
        let divided = matches!(amount, ValueDef::DividedAmongTargets);
        let shared = if divided {
            0
        } else {
            self.effect_value(amount, object, context, scoped)
                .max(0)
                .try_into()
                .unwrap_or(u16::MAX)
        };
        let slot = recipient
            .legal_target()
            .map(|target| scoped.target_slot(target));
        let source = object.source.or(Some(object.id));
        let mut damaged = Vec::new();
        for target in self.effect_recipients(recipient, object, context, scoped) {
            let amount = if divided {
                slot.and_then(|slot| Self::divided_share(object, slot, target))
                    .unwrap_or(0)
            } else {
                shared
            };
            if amount == 0 && divided {
                continue;
            }
            // Asked before the damage, because that is when redirection is
            // decided, and answered the same way the deal itself will answer
            // it. Nothing between the two calls can change it.
            let landed = self.redirected_damage_target(source, Some(target));
            if self.damage_target_from(source, Some(target), amount) > 0
                && let Some(landed @ Target::Permanent(_)) = landed
                && !damaged.contains(&landed)
            {
                damaged.push(landed);
            }
        }
        damaged
    }
}
