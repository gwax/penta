//! "You may play lands from your graveyard."
//!
//! The mirror of the play prohibitions next door: those say what a player
//! may not do, and this says what they may do that the ordinary rules would
//! refuse. It is asked where the action is offered, so a card the
//! permission does not name simply is not playable from there.

use std::ops::ControlFlow;

use super::{
    AppliedEffectDef, AppliedRuleDef, CardInstance, CharacteristicContext, DeclarativeAbilityDef,
    Game, PlayActionKind, PlayOptionDef, PlayerId,
};

impl Game {
    /// Whether this player may play this card out of a graveyard right now.
    pub(super) fn graveyard_play_is_permitted(
        &self,
        card: &CardInstance,
        player: PlayerId,
        option: &PlayOptionDef,
    ) -> bool {
        // Only your own graveyard: no printed permission reaches another
        // player's, and the enumeration walks only yours.
        if card.owner != player {
            return false;
        }
        let context = match option.action {
            PlayActionKind::CastSpell => CharacteristicContext::Stack {
                form: option.form.clone(),
            },
            PlayActionKind::PlayLand => CharacteristicContext::Hand,
        };
        let Some(object) =
            self.printed_trigger_event_object(card.id, card.definition, player, &context)
        else {
            return false;
        };
        self.visit_graveyard_play_permissions(player, |source, restriction| {
            if restriction.action.matches(option.action)
                && self.trigger_object_matches(
                    restriction.object,
                    &object,
                    source,
                    option.action == PlayActionKind::CastSpell,
                )
            {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .is_break()
    }

    fn visit_graveyard_play_permissions(
        &self,
        affected_player: PlayerId,
        mut visitor: impl FnMut(super::GameObjectId, crate::card::PlayRestrictionDef) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        for source in self.battlefield.iter().chain(self.emblems.iter()) {
            let Some(rules) = self.effective_rules(source) else {
                continue;
            };
            for ability in rules.ability_clauses() {
                if !ability.is_executable()
                    || !matches!(ability.definition, DeclarativeAbilityDef::Static(_))
                {
                    continue;
                }
                let Some(effect) = ability.declarative_effect() else {
                    continue;
                };
                let super::EffectDef::StaticApply { recipient, effect } = effect else {
                    continue;
                };
                if !self.static_player_recipient_matches(recipient, source, affected_player) {
                    continue;
                }
                let mut found = ControlFlow::Continue(());
                Self::visit_graveyard_play_components(effect, &mut |restriction| {
                    if found.is_continue() {
                        found = visitor(source.card.id, restriction);
                    }
                });
                found?;
            }
        }
        ControlFlow::Continue(())
    }

    fn visit_graveyard_play_components(
        effect: AppliedEffectDef,
        visitor: &mut impl FnMut(crate::card::PlayRestrictionDef),
    ) {
        match effect {
            AppliedEffectDef::Composite(effects) => {
                for effect in effects {
                    Self::visit_graveyard_play_components(*effect, visitor);
                }
            }
            AppliedEffectDef::Rule(AppliedRuleDef::MayPlayFromGraveyard(restriction)) => {
                visitor(restriction);
            }
            _ => {}
        }
    }
}
