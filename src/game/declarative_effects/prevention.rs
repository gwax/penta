use super::super::{
    DamageSourceGroupDef, EffectDef, EffectRecipientDef, EffectResolutionContext, Game,
    PreventionShield, RelationalDamagePrevention, RelationalSourceFilter, ScopedEffect,
    ShieldCoverageDef, StackObject, Target, TargetIndex,
};

impl Game {
    /// Records a turn-long prevention naming a group of sources. The group
    /// crosses from card vocabulary to engine vocabulary here, since only the
    /// engine's form has to survive a checkpoint.
    fn prevent_player_damage_from_group(
        &mut self,
        player: EffectRecipientDef,
        source: DamageSourceGroupDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let filter = match source {
            DamageSourceGroupDef::CreaturesWithFlying => {
                RelationalSourceFilter::CreaturesWithFlying
            }
            DamageSourceGroupDef::AttackingCreaturesWithoutFlying => {
                RelationalSourceFilter::AttackingCreaturesWithoutFlying
            }
            DamageSourceGroupDef::Artifacts => RelationalSourceFilter::Artifacts,
            DamageSourceGroupDef::UnblockedCreatures => RelationalSourceFilter::UnblockedCreatures,
        };
        for target in self.effect_recipients(player, object, context, scoped) {
            if let Target::Player(player) = target {
                self.relational_damage_preventions
                    .push(RelationalDamagePrevention::ToPlayerFrom {
                        player,
                        source: filter,
                    });
            }
        }
    }

    /// Marks each recipient as dealing no damage for the rest of the turn,
    /// either combat damage alone or every kind.
    fn silence_damage_sources(
        &mut self,
        recipient: EffectRecipientDef,
        every_kind: bool,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        for target in self.effect_recipients(recipient, object, context, scoped) {
            if let Target::Permanent(id) = target
                && let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == id)
            {
                if every_kind {
                    permanent.damage_dealt_by_prevented = true;
                } else {
                    permanent.combat_damage_dealt_by_prevented = true;
                }
            }
        }
    }

    /// Records a turn-long redirection naming one source and one
    /// destination. Both ends are object ids: unlike the group form there is
    /// no vocabulary word that would describe them.
    fn redirect_target_damage(
        &mut self,
        player: EffectRecipientDef,
        from: TargetIndex,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let destination = object.source.unwrap_or(object.id);
        let Some(source) = Game::chosen_targets(object, scoped.target_slot(from)).find_map(
            |target| match target {
                Target::Permanent(id) => Some(id),
                _ => None,
            },
        ) else {
            return;
        };
        for target in self.effect_recipients(player, object, context, scoped) {
            if let Target::Player(player) = target {
                self.relational_damage_preventions.push(
                    RelationalDamagePrevention::RedirectToPermanent {
                        player,
                        source,
                        destination,
                    },
                );
            }
        }
    }

    pub(super) fn resolve_prevention_effect(
        &mut self,
        scoped: ScopedEffect,
        object: &StackObject,
        context: &EffectResolutionContext,
    ) {
        match scoped.effect {
            EffectDef::PreventNextDamage {
                object: recipient,
                amount,
            } => {
                let amount = self
                    .effect_value(amount, object, context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    self.prevention_shields.push(PreventionShield {
                        recipient: target,
                        remaining: Some(amount),
                        source: None,
                        coverage: ShieldCoverageDef::All,
                        gain_life: false,
                    });
                }
            }
            EffectDef::PreventNextDamageFromSource { .. } => {
                self.install_damage_source_shield(scoped, object, context);
            }
            EffectDef::PreventAllDamageThisTurn { object: recipient } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    self.prevention_shields.push(PreventionShield {
                        recipient: target,
                        remaining: None,
                        source: None,
                        coverage: ShieldCoverageDef::All,
                        gain_life: false,
                    });
                }
            }
            EffectDef::PreventAllCombatDamageThisTurn => {
                self.all_combat_damage_prevented = true;
            }
            EffectDef::PreventCombatDamageThisTurn { object: recipient } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Permanent(id) = target
                        && let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|permanent| permanent.card.id == id)
                    {
                        permanent.combat_damage_prevented = true;
                    }
                }
            }
            EffectDef::PreventCombatDamageDealtByThisTurn { object: recipient } => {
                self.silence_damage_sources(recipient, false, object, context, scoped);
            }
            EffectDef::PreventDamageDealtByThisTurn { object: recipient } => {
                self.silence_damage_sources(recipient, true, object, context, scoped);
            }
            EffectDef::PreventDamageToPlayerAndControlledCreaturesThisTurn { player } => {
                for target in self.effect_recipients(player, object, context, scoped) {
                    if let Target::Player(player) = target {
                        self.relational_damage_preventions.push(
                            RelationalDamagePrevention::ToPlayerAndControlledCreatures(player),
                        );
                    }
                }
            }
            EffectDef::PreventDamageToPlayerFromThisTurn { player, source } => {
                self.prevent_player_damage_from_group(player, source, object, context, scoped);
            }
            EffectDef::RedirectTargetDamageToSourceThisTurn { player, from } => {
                self.redirect_target_damage(player, from, object, context, scoped);
            }
            EffectDef::PreventAllCombatDamageExceptSourceThisTurn { source } => {
                let source = self
                    .effect_recipients(source, object, context, scoped)
                    .into_iter()
                    .find_map(|target| match target {
                        Target::Permanent(id) | Target::Spell(id) | Target::Card(id) => Some(id),
                        Target::Player(_) => None,
                    });
                if let Some(source) = source {
                    self.relational_damage_preventions
                        .push(RelationalDamagePrevention::FromAllExcept(source));
                }
            }
            _ => unreachable!("resolve_prevention_effect called for a non-prevention effect"),
        }
    }

    fn install_damage_source_shield(
        &mut self,
        scoped: ScopedEffect,
        object: &StackObject,
        context: &EffectResolutionContext,
    ) {
        let EffectDef::PreventNextDamageFromSource {
            object: recipient,
            source,
            coverage,
            gain_life,
        } = scoped.effect
        else {
            unreachable!("damage source shield helper called for a different effect");
        };
        let named = self
            .effect_recipients(source, object, context, scoped)
            .into_iter()
            .find_map(|target| match target {
                Target::Permanent(id) | Target::Spell(id) | Target::Card(id) => Some(id),
                Target::Player(_) => None,
            });
        let Some(named) = named else {
            return;
        };
        for target in self.effect_recipients(recipient, object, context, scoped) {
            self.prevention_shields.push(PreventionShield {
                recipient: target,
                remaining: None,
                source: Some(named),
                coverage,
                gain_life,
            });
        }
    }
}
