//! Static rules whose subject is a player.
//!
//! A rule applied to an object is found through that object: damage
//! prevention is read off the source or target permanent, a blocking
//! restriction off the creature it restricts. A rule applied to a *player*
//! has no such anchor, so each of these walks the battlefield and emblems
//! and asks every static ability whether its recipient is the player in
//! question.

use std::ops::ControlFlow;

use crate::action::AbilityOrigin;
use crate::card::{
    AppliedEffectDef, AppliedRuleDef, DamageEventMatcherDef, DamageLimitDef, DeclarativeAbilityDef,
    EffectDef, EffectRecipientDef, EffectRecipientSetDef, PlayerRefDef, PlayerSetDef,
};
use crate::ids::{GameObjectId, PlayerId};

use super::super::{AppliedPlayRestriction, Game, Permanent, TriggerContext};

impl Game {
    /// Visits static and resolved play prohibitions in timestamp/component
    /// order for one player. Static prohibitions are derived live from their
    /// source; resolving prohibitions use the game-level stored rule list.
    pub(in crate::game) fn visit_play_restrictions(
        &self,
        affected_player: PlayerId,
        mut visitor: impl FnMut(AppliedPlayRestriction) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        let mut restrictions = self
            .resolved_play_restrictions
            .iter()
            .filter(|restriction| {
                restriction.affected_player == affected_player
                    && self.continuous_effect_expiration_is_active(
                        restriction.expiration,
                        restriction.source.object,
                    )
            })
            .map(|restriction| AppliedPlayRestriction {
                source: restriction.source.object,
                timestamp: restriction.timestamp,
                component_order: restriction.component_order,
                restriction: restriction.restriction,
            })
            .collect::<Vec<_>>();

        let land_type_sources = self.land_type_effect_sources(None);
        for source in self.battlefield.iter().chain(self.emblems.iter()) {
            let Some(rules) = self.effective_rules(source) else {
                continue;
            };
            let (source_definition, source_part) = Self::effective_rules_source(source);
            if self.rules_text_abilities_removed_from_sources(source, &land_type_sources) {
                continue;
            }
            for attached in rules.indexed_abilities() {
                if !attached.definition.is_executable()
                    || !matches!(
                        attached.definition.definition,
                        DeclarativeAbilityDef::Static(_)
                    )
                {
                    continue;
                }
                if !self.ability_survives_resolved_operations(
                    source,
                    AbilityOrigin::Printed {
                        definition: source_definition,
                        part: source_part,
                        ability: attached.id,
                    },
                ) {
                    continue;
                }
                let Some(effect) = attached.definition.declarative_effect() else {
                    continue;
                };
                let mut component_order = 0;
                self.collect_static_play_restrictions(
                    effect,
                    source,
                    affected_player,
                    true,
                    &mut component_order,
                    &mut restrictions,
                );
            }
        }

        restrictions
            .sort_by_key(|restriction| (restriction.timestamp, restriction.component_order));
        for restriction in restrictions {
            if visitor(restriction).is_break() {
                return ControlFlow::Break(());
            }
        }
        ControlFlow::Continue(())
    }

    /// Visits static damage limits that apply to one player, in the same
    /// battlefield-and-emblems walk the play prohibitions use. A limit whose
    /// recipient is a player has no other anchor: unlike a prevention on the
    /// source or target permanent, nothing about the damage event points at
    /// the permanent carrying the rule.
    pub(in crate::game) fn visit_player_damage_limits(
        &self,
        affected_player: PlayerId,
        mut visitor: impl FnMut(GameObjectId, DamageEventMatcherDef, DamageLimitDef) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        let land_type_sources = self.land_type_effect_sources(None);
        for source in self.battlefield.iter().chain(self.emblems.iter()) {
            let Some(rules) = self.effective_rules(source) else {
                continue;
            };
            let (source_definition, source_part) = Self::effective_rules_source(source);
            if self.rules_text_abilities_removed_from_sources(source, &land_type_sources) {
                continue;
            }
            for attached in rules.indexed_abilities() {
                if !attached.definition.is_executable()
                    || !matches!(
                        attached.definition.definition,
                        DeclarativeAbilityDef::Static(_)
                    )
                {
                    continue;
                }
                if !self.ability_survives_resolved_operations(
                    source,
                    AbilityOrigin::Printed {
                        definition: source_definition,
                        part: source_part,
                        ability: attached.id,
                    },
                ) {
                    continue;
                }
                let Some(EffectDef::StaticApply { recipient, effect }) =
                    attached.definition.declarative_effect()
                else {
                    continue;
                };
                let AppliedEffectDef::Rule(AppliedRuleDef::LimitDamage { matcher, limit }) = effect
                else {
                    continue;
                };
                if !self.static_player_recipient_matches(recipient, source, affected_player) {
                    continue;
                }
                if visitor(source.card.id, matcher, limit).is_break() {
                    return ControlFlow::Break(());
                }
            }
        }
        ControlFlow::Continue(())
    }

    fn collect_static_play_restrictions(
        &self,
        effect: EffectDef,
        source: &Permanent,
        affected_player: PlayerId,
        enabled: bool,
        component_order: &mut u16,
        restrictions: &mut Vec<AppliedPlayRestriction>,
    ) {
        match effect {
            EffectDef::Sequence(effects) => {
                for effect in effects {
                    self.collect_static_play_restrictions(
                        *effect,
                        source,
                        affected_player,
                        enabled,
                        component_order,
                        restrictions,
                    );
                }
            }
            EffectDef::IfCondition { condition, then } => {
                let condition_holds = enabled
                    && self.trigger_condition_holds(
                        condition,
                        source.card.id,
                        source.controller,
                        TriggerContext::empty(),
                        None,
                        None,
                    );
                self.collect_static_play_restrictions(
                    *then,
                    source,
                    affected_player,
                    condition_holds,
                    component_order,
                    restrictions,
                );
            }
            EffectDef::StaticApply { recipient, effect } => {
                let include = enabled
                    && self.static_player_recipient_matches(recipient, source, affected_player);
                Self::collect_play_restriction_components(
                    effect,
                    source,
                    include,
                    component_order,
                    restrictions,
                );
            }
            _ => {}
        }
    }

    fn collect_play_restriction_components(
        effect: AppliedEffectDef,
        source: &Permanent,
        include: bool,
        component_order: &mut u16,
        restrictions: &mut Vec<AppliedPlayRestriction>,
    ) {
        match effect {
            AppliedEffectDef::Composite(effects) => {
                for effect in effects {
                    Self::collect_play_restriction_components(
                        *effect,
                        source,
                        include,
                        component_order,
                        restrictions,
                    );
                }
            }
            AppliedEffectDef::Characteristic(_) | AppliedEffectDef::Rule(_) => {
                let order = *component_order;
                *component_order = component_order
                    .checked_add(1)
                    .expect("one static ability contains at most 65,536 applied components");
                if include
                    && let AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(restriction)) = effect
                {
                    restrictions.push(AppliedPlayRestriction {
                        source: source.card.id,
                        timestamp: source.timestamp,
                        component_order: order,
                        restriction,
                    });
                }
            }
        }
    }

    fn static_player_recipient_matches(
        &self,
        recipient: EffectRecipientDef,
        source: &Permanent,
        affected_player: PlayerId,
    ) -> bool {
        match recipient.0 {
            EffectRecipientSetDef::Players(PlayerSetDef::All) => true,
            EffectRecipientSetDef::Players(PlayerSetDef::One(PlayerRefDef::EffectController)) => {
                affected_player == source.controller
            }
            EffectRecipientSetDef::Players(PlayerSetDef::Related(relation)) => self
                .player_relation_matches(
                    affected_player,
                    relation,
                    source.controller,
                    TriggerContext::empty(),
                ),
            EffectRecipientSetDef::LegalTargets(_)
            | EffectRecipientSetDef::Objects(_)
            | EffectRecipientSetDef::Players(
                PlayerSetDef::LegalTargets(_)
                | PlayerSetDef::One(
                    PlayerRefDef::EventPlayer
                    | PlayerRefDef::Target(_)
                    | PlayerRefDef::ControllerOf(_)
                    | PlayerRefDef::OwnerOf(_),
                ),
            ) => false,
        }
    }
}
