use super::{
    EffectResolutionContext, Game, GameObjectId, ObjectPredicateDef, PlayerId, RetiredObject,
    ScopedEffect, StackObject, Target, ValueDef,
};

impl Game {
    #[allow(clippy::too_many_lines)]
    pub(super) fn effect_value(
        &self,
        value: ValueDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> i32 {
        match value {
            ValueDef::Constant(value) => value,
            ValueDef::CreaturesDiedThisTurn => i32::from(self.creatures_died_this_turn),
            ValueDef::ChosenX => i32::from(object.x()),
            ValueDef::SourcePower => object
                .source
                .and_then(|source| self.current_or_last_known_power(source))
                .map_or(0, i32::from),
            ValueDef::SourceToughness => object
                .source
                .and_then(|source| self.current_or_last_known_toughness(source))
                .map_or(0, i32::from),
            ValueDef::TriggerEventAmount => context.trigger.amount.unwrap_or(0),
            // Resolved per target by the divided-damage path; anything else
            // reading it has no target in hand and so no share.
            ValueDef::DividedAmongTargets => 0,
            ValueDef::SourceCastX => self
                .battlefield
                .iter()
                .find(|permanent| Some(permanent.card.id) == object.source)
                .map_or(0, |permanent| i32::from(permanent.cast_x)),
            ValueDef::TriggeringObjectPower => context
                .trigger
                .object
                .and_then(|object| self.current_or_last_known_power(object))
                .map_or(0, i32::from),
            ValueDef::TriggeringObjectToughness => context
                .trigger
                .object
                .and_then(|object| self.current_or_last_known_toughness(object))
                .map_or(0, i32::from),
            ValueDef::TargetPower(target) => {
                Self::chosen_targets(object, scoped.target_slot(target))
                    .find_map(|target| match target {
                        Target::Permanent(id) => self.current_or_last_known_power(id),
                        Target::Player(_) | Target::Card(_) | Target::Spell(_) => None,
                    })
                    .map_or(0, i32::from)
            }
            ValueDef::TargetToughness(target) => {
                Self::chosen_targets(object, scoped.target_slot(target))
                    .find_map(|target| match target {
                        Target::Permanent(id) => self.current_or_last_known_toughness(id),
                        Target::Player(_) | Target::Card(_) | Target::Spell(_) => None,
                    })
                    .map_or(0, i32::from)
            }
            ValueDef::LifeTotal(relation) => [PlayerId::One, PlayerId::Two]
                .into_iter()
                .find(|candidate| {
                    self.player_relation_matches(
                        *candidate,
                        relation,
                        object.controller,
                        context.trigger,
                    )
                })
                .map_or(0, |player| i32::from(self.players[player.index()].life)),
            ValueDef::TargetLibrarySize(target) => {
                Self::chosen_targets(object, scoped.target_slot(target))
                    .find_map(|target| match target {
                        Target::Player(player) => {
                            i32::try_from(self.players[player.index()].library.len()).ok()
                        }
                        Target::Permanent(_) | Target::Card(_) | Target::Spell(_) => None,
                    })
                    .unwrap_or(0)
            }
            ValueDef::TargetManaValue(target) => {
                Self::chosen_targets(object, scoped.target_slot(target))
                    .find_map(|target| match target {
                        Target::Permanent(id) | Target::Card(id) | Target::Spell(id) => {
                            self.current_or_last_known_mana_value(id)
                        }
                        Target::Player(_) => None,
                    })
                    .map_or(0, i32::from)
            }
            ValueDef::CountersOnSource(kind) => object.source.map_or(0, |source| {
                i32::from(self.current_or_last_known_counters(source, kind))
            }),
            ValueDef::DamageTakenThisTurn { player, source } => {
                let player = [PlayerId::One, PlayerId::Two]
                    .into_iter()
                    .find(|candidate| {
                        self.player_relation_matches(
                            *candidate,
                            player,
                            object.controller,
                            context.trigger,
                        )
                    })
                    .unwrap_or(object.controller);
                i32::from(self.damage_taken_this_turn(player, source))
            }
            ValueDef::CardsInHandAbove { player, threshold } => {
                let player = [PlayerId::One, PlayerId::Two]
                    .into_iter()
                    .find(|candidate| {
                        self.player_relation_matches(
                            *candidate,
                            player,
                            object.controller,
                            context.trigger,
                        )
                    })
                    .unwrap_or(object.controller);
                i32::try_from(
                    self.players[player.index()]
                        .hand
                        .len()
                        .saturating_sub(usize::from(threshold)),
                )
                .unwrap_or(i32::MAX)
            }
            ValueDef::CountMatchingObjects(query) => i32::try_from(
                self.objects_matching_effect_query(*query, object, context, scoped)
                    .len(),
            )
            .unwrap_or(i32::MAX),
            // Zero when nothing matches, which is what "the greatest power
            // among creatures you control" is worth with no creatures.
            ValueDef::GreatestPowerAmong(query) => self
                .objects_matching_effect_query(*query, object, context, scoped)
                .into_iter()
                .filter_map(|target| match target {
                    Target::Permanent(id) => self.current_or_last_known_power(id),
                    Target::Player(_) | Target::Card(_) | Target::Spell(_) => None,
                })
                .max()
                .map_or(0, i32::from),
            ValueDef::AnyMatchingObject(query) => i32::from(self.any_battlefield_object_matches(
                query,
                object.source.unwrap_or(object.id),
                object.controller,
            )),
            ValueDef::IfTargetMatches(_)
            | ValueDef::IfMatchingObjectCount(_)
            | ValueDef::IfCreatureDiedThisTurn(_)
            | ValueDef::IfControllerLifeAtMost(_) => {
                self.conditional_effect_value(value, object, context, scoped)
            }
            ValueDef::Negate(inner) => self
                .effect_value(*inner, object, context, scoped)
                .saturating_neg(),
            ValueDef::Scaled(scaled) => self
                .effect_value(scaled.value, object, context, scoped)
                .saturating_mul(scaled.factor),
            ValueDef::Halved(halved) => {
                halved.apply(self.effect_value(halved.value, object, context, scoped))
            }
            ValueDef::Sum(sum) => self
                .effect_value(sum.left, object, context, scoped)
                .saturating_add(self.effect_value(sum.right, object, context, scoped)),
        }
    }

    /// Whether the permanent a target slot points at matches, reading it as
    /// it last existed when it is no longer on the battlefield.
    ///
    /// "If that creature was a Human" is asked after the destruction that
    /// removed it, and a permanent that leaves gets a fresh object identity
    /// in its new zone -- so the corpse in the retired table is the only
    /// thing the old target still names.
    fn permanent_condition_matches(
        &self,
        predicate: ObjectPredicateDef,
        id: GameObjectId,
        source: GameObjectId,
    ) -> bool {
        if let Some(permanent) = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .or_else(|| match self.retired_objects.get(&id) {
                Some(RetiredObject::Permanent { permanent, .. }) => Some(permanent.as_ref()),
                _ => None,
            })
        {
            return self.trigger_object_matches(
                predicate,
                &self.trigger_event_object(permanent),
                source,
                false,
            );
        }
        self.card_in_nonbattlefield_zone(id)
            .is_some_and(|(zone, card)| self.card_object_matches(predicate, card, zone, source))
    }

    /// Resolve values that select between two branches separately from the
    /// direct value forms above.
    fn conditional_effect_value(
        &self,
        value: ValueDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> i32 {
        match value {
            ValueDef::IfTargetMatches(condition) => {
                let source = object.source.unwrap_or(object.id);
                // The chosen target rather than the still-legal one: "if that
                // creature was a Human" is asked after the destruction that
                // made it illegal, which is the only time it is interesting.
                let matched = Self::chosen_targets(object, scoped.target_slot(condition.slot)).any(
                    |target| match target {
                        Target::Card(id) => {
                            self.card_in_nonbattlefield_zone(id)
                                .is_some_and(|(zone, card)| {
                                    self.card_object_matches(condition.object, card, zone, source)
                                })
                        }
                        Target::Permanent(id) => {
                            self.permanent_condition_matches(condition.object, id, source)
                        }
                        Target::Player(_) | Target::Spell(_) => false,
                    },
                );
                let chosen = if matched {
                    condition.then
                } else {
                    condition.otherwise
                };
                self.effect_value(chosen, object, context, scoped)
            }
            ValueDef::IfMatchingObjectCount(condition) => {
                let count = self.effect_value(
                    ValueDef::CountMatchingObjects(&condition.query),
                    object,
                    context,
                    scoped,
                );
                let chosen = if count == i32::from(condition.equals) {
                    condition.then
                } else {
                    condition.otherwise
                };
                self.effect_value(chosen, object, context, scoped)
            }
            ValueDef::IfControllerLifeAtMost(branches) => {
                let chosen = if i32::from(self.players[object.controller.index()].life)
                    <= i32::from(branches.threshold)
                {
                    branches.then
                } else {
                    branches.otherwise
                };
                self.effect_value(chosen, object, context, scoped)
            }
            ValueDef::IfCreatureDiedThisTurn(branches) => {
                let chosen = if self.creature_died_this_turn {
                    branches.then
                } else {
                    branches.otherwise
                };
                self.effect_value(chosen, object, context, scoped)
            }
            // The caller only routes conditional values here.
            _ => 0,
        }
    }
}
