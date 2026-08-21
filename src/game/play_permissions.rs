//! "You may play lands from your graveyard", and the same sentence pointed
//! at the top of a library.
//!
//! The mirror of the play prohibitions next door: those say what a player
//! may not do, and this says what they may do that the ordinary rules would
//! refuse. It is asked where the action is offered, so a card the
//! permission does not name simply is not playable from there.

use std::ops::ControlFlow;

use super::{
    AppliedEffectDef, AppliedRuleDef, CardInstance, CharacteristicContext, DeclarativeAbilityDef,
    Game, GameObjectId, PlayActionKind, PlayOptionDef, PlayerId,
};
use crate::card::{PlayRestrictionDef, TopOfLibraryCostDef};

/// One printed permission to play a card from a zone the ordinary rules
/// would not allow, and what playing it that way costs.
#[derive(Clone, Copy)]
pub(super) enum PlayPermission {
    Graveyard(PlayRestrictionDef),
    TopOfLibrary {
        restriction: PlayRestrictionDef,
        cost: TopOfLibraryCostDef,
    },
}

impl PlayPermission {
    const fn restriction(self) -> PlayRestrictionDef {
        match self {
            Self::Graveyard(restriction) | Self::TopOfLibrary { restriction, .. } => restriction,
        }
    }
}

impl Game {
    /// Whether this player may play this card out of a graveyard right now.
    pub(super) fn graveyard_play_is_permitted(
        &self,
        card: &CardInstance,
        player: PlayerId,
        option: &PlayOptionDef,
    ) -> bool {
        self.matching_play_permission(card, player, option, |permission| {
            matches!(permission, PlayPermission::Graveyard(_)).then_some(())
        })
        .is_some()
    }

    /// What playing this card off the top of its owner's library would cost,
    /// or `None` when nothing permits it.
    pub(super) fn library_top_play_cost(
        &self,
        card: &CardInstance,
        player: PlayerId,
        option: &PlayOptionDef,
    ) -> Option<TopOfLibraryCostDef> {
        self.matching_play_permission(card, player, option, |permission| match permission {
            PlayPermission::TopOfLibrary { cost, .. } => Some(cost),
            PlayPermission::Graveyard(_) => None,
        })
    }

    /// What a spell cast off the top of `player`'s library pays in life,
    /// when the permission charges life rather than mana.
    ///
    /// The mana value is the card's own, with X counted as zero: a spell
    /// nobody is paying mana for has no X to choose (CR 202.3b), the same
    /// reading the energy permission next door already uses.
    pub(super) fn library_top_life_cost(
        &self,
        card: &CardInstance,
        player: PlayerId,
        option: &PlayOptionDef,
    ) -> Option<u16> {
        if self.library_top_play_cost(card, player, option)
            != Some(TopOfLibraryCostDef::LifeEqualToManaValue)
        {
            return None;
        }
        Some(
            self.catalog
                .get(card.definition)?
                .rules
                .printed_mana_cost()
                .mana_value(),
        )
    }

    /// The first live permission that names this card and this play option,
    /// as whatever `wanted` reads off it.
    fn matching_play_permission<T>(
        &self,
        card: &CardInstance,
        player: PlayerId,
        option: &PlayOptionDef,
        wanted: impl Fn(PlayPermission) -> Option<T>,
    ) -> Option<T> {
        // Only your own cards: no printed permission reaches another
        // player's zones, and the enumeration walks only yours.
        if card.owner != player {
            return None;
        }
        let context = match option.action {
            PlayActionKind::CastSpell => CharacteristicContext::Stack {
                form: option.form.clone(),
            },
            PlayActionKind::PlayLand => CharacteristicContext::Hand,
        };
        let object =
            self.printed_trigger_event_object(card.id, card.definition, player, &context)?;
        let mut found = None;
        let _ = self.visit_play_permissions(player, |source, permission| {
            let restriction = permission.restriction();
            if restriction.action.matches(option.action)
                && self.trigger_object_matches(
                    restriction.object,
                    &object,
                    source,
                    option.action == PlayActionKind::CastSpell,
                )
                && let Some(value) = wanted(permission)
            {
                found = Some(value);
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        });
        found
    }

    fn visit_play_permissions(
        &self,
        affected_player: PlayerId,
        mut visitor: impl FnMut(GameObjectId, PlayPermission) -> ControlFlow<()>,
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
                Self::visit_play_permission_components(effect, &mut |permission| {
                    if found.is_continue() {
                        found = visitor(source.card.id, permission);
                    }
                });
                found?;
            }
        }
        ControlFlow::Continue(())
    }

    fn visit_play_permission_components(
        effect: AppliedEffectDef,
        visitor: &mut impl FnMut(PlayPermission),
    ) {
        match effect {
            AppliedEffectDef::Composite(effects) => {
                for effect in effects {
                    Self::visit_play_permission_components(*effect, visitor);
                }
            }
            AppliedEffectDef::Rule(AppliedRuleDef::MayPlayFromGraveyard(restriction)) => {
                visitor(PlayPermission::Graveyard(restriction));
            }
            AppliedEffectDef::Rule(AppliedRuleDef::MayPlayFromTopOfLibrary {
                restriction,
                cost,
            }) => {
                visitor(PlayPermission::TopOfLibrary { restriction, cost });
            }
            _ => {}
        }
    }
}
