// Asking whether a condition holds: the intervening-ifs a trigger checks
// twice, the guards an effect branches on, and the two values such a
// condition compares.
//
// Split out of `effect_support.rs` only to keep one file readable; these are
// ordinary members of the same `impl Game`. The paths and imports are the
// parent module's.

impl Game {
    /// How many times this ability has been activated from this permanent so
    /// far this turn.
    /// One side of a two-value condition. The board-readable values a cost
    /// reduction already answers, plus the two a condition of this shape
    /// actually asks about: what a player is devoted to, and how much
    /// library they have left.
    fn condition_value(
        &self,
        value: crate::card::ValueDef,
        source: GameObjectId,
        controller: PlayerId,
    ) -> i32 {
        match value {
            crate::card::ValueDef::DevotionTo(_) | crate::card::ValueDef::LibrarySize(_) => {
                self.player_readable_value(value, controller)
            }
            other => i32::from(self.cost_reduction_value(other, controller, source)),
        }
    }

    pub(super) fn ability_activations_this_turn(
        &self,
        source: GameObjectId,
        ability: AbilityOrigin,
    ) -> u8 {
        self.battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
            .and_then(|permanent| {
                permanent
                    .activations_this_turn
                    .iter()
                    .find(|(origin, _)| *origin == ability)
            })
            .map_or(0, |(_, count)| *count)
    }

    /// Whether a trigger's intervening-if condition holds right now. Rule
    /// 603.4 asks this when the ability would trigger and again as it
    /// resolves, so both call sites read the same board.
    #[allow(clippy::too_many_lines)]
    pub(super) fn trigger_condition_holds(
        &self,
        condition: &TriggerConditionDef,
        source: GameObjectId,
        controller: PlayerId,
        context: TriggerContext,
        ability: Option<AbilityOrigin>,
        object: Option<(&StackObject, ScopedEffect, &EffectResolutionContext)>,
    ) -> bool {
        let TriggerConditionDef::ObjectCount {
            query,
            comparison,
            amount,
        } = condition
        else {
            return match condition {
                TriggerConditionDef::All(conditions) => conditions.iter().all(|condition| {
                    self.trigger_condition_holds(
                        condition, source, controller, context, ability, object,
                    )
                }),
                TriggerConditionDef::Not(condition) => !self.trigger_condition_holds(
                    condition, source, controller, context, ability, object,
                ),
                // Both sides are read where the condition is checked, which
                // is the only way "X or more cards in your library" can be
                // said: neither amount is a printed number.
                TriggerConditionDef::ValueComparison(values) => {
                    let left = self.condition_value(values.left, source, controller);
                    let right = self.condition_value(values.right, source, controller);
                    compare(&left, values.comparison, &right)
                }
                TriggerConditionDef::SourceOnBattlefield => self
                    .battlefield
                    .iter()
                    .any(|permanent| permanent.card.id == source),
                // Names, not identities: a second copy of the named card is
                // still the named card, so the definitions are compared.
                TriggerConditionDef::BoundObjectsShareName { first, second } => {
                    let named = |binding| {
                        object
                            .and_then(|(_, _, context): (_, _, &EffectResolutionContext)| {
                                context.single_object(binding)
                            })
                            .and_then(|target| match target {
                                Target::Permanent(id) | Target::Card(id) | Target::Spell(id) => {
                                    self.object_definition(id)
                                }
                                Target::Player(_) => None,
                            })
                    };
                    match (named(*first), named(*second)) {
                        (Some(first), Some(second)) => first == second,
                        _ => false,
                    }
                }
                // The permanent records the controller's turn count as it
                // arrived. By this upkeep that count has advanced once, so
                // "since the last upkeep" is exactly one turn ago -- and the
                // check stops being true afterwards, which is what keeps an
                // echo cost from coming due a second time.
                TriggerConditionDef::SourceArrivedSinceControllersLastUpkeep => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .is_some_and(|permanent| {
                        self.turns_started[permanent.controller.index()]
                            == permanent.entered_controller_turn.saturating_add(1)
                    }),
                TriggerConditionDef::SourceUntapped => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .is_some_and(|permanent| !permanent.tapped),
                TriggerConditionDef::ActivePlayer(relation) => {
                    self.player_relation_matches(self.active_player, *relation, controller, context)
                }
                TriggerConditionDef::SpellsCastThisTurn {
                    quantifier,
                    player: relation,
                    comparison,
                    amount,
                } => {
                    let mut matching =
                        [PlayerId::One, PlayerId::Two].into_iter().filter(|player| {
                            self.player_relation_matches(*player, *relation, controller, context)
                        });
                    let satisfies = |player: PlayerId| {
                        compare(
                            &self.spells_cast_this_turn[player.index()],
                            *comparison,
                            &u16::from(*amount),
                        )
                    };
                    match quantifier {
                        QuantifierDef::Every => matching.all(satisfies),
                        QuantifierDef::Any => matching.any(satisfies),
                    }
                }
                TriggerConditionDef::SpellsCastLastTurn {
                    quantifier,
                    player: relation,
                    comparison,
                    amount,
                } => {
                    let mut matching =
                        [PlayerId::One, PlayerId::Two].into_iter().filter(|player| {
                            self.player_relation_matches(*player, *relation, controller, context)
                        });
                    let satisfies = |player: PlayerId| {
                        compare(
                            &self.spells_cast_last_turn[player.index()],
                            *comparison,
                            &u16::from(*amount),
                        )
                    };
                    match quantifier {
                        QuantifierDef::Every => matching.all(satisfies),
                        QuantifierDef::Any => matching.any(satisfies),
                    }
                }
                // A tie counts, so this asks whether anything is strictly
                // bigger rather than whether one creature is unique.
                TriggerConditionDef::ControlsGreatestPowerCreature => {
                    let mut best: Option<i16> = None;
                    let mut mine: Option<i16> = None;
                    for permanent in &self.battlefield {
                        let Some(power) = self.power(permanent) else {
                            continue;
                        };
                        best = Some(best.map_or(power, |seen: i16| seen.max(power)));
                        if permanent.controller == controller {
                            mine = Some(mine.map_or(power, |seen: i16| seen.max(power)));
                        }
                    }
                    match (mine, best) {
                        (Some(mine), Some(best)) => mine >= best,
                        _ => false,
                    }
                }
                // Follows the attachment rather than being frozen when the
                // Equipment moved, so the answer is about where it is now.
                TriggerConditionDef::ControllerHasCitysBlessing => {
                    self.citys_blessing[controller.index()]
                }
                TriggerConditionDef::ControllerGainedLifeThisTurn => {
                    self.life_gained_this_turn[controller.index()] > 0
                }
                TriggerConditionDef::ControllerHadPermanentLeaveThisTurn => {
                    self.permanent_left_battlefield_this_turn[controller.index()]
                }
                TriggerConditionDef::CreatureDiedThisTurn => self.creature_died_this_turn,
                // A dies-trigger asks about the permanent that died, which
                // is no longer there to look at. "If it was a creature" has
                // to read what it last was (CR 603.10); finding nothing and
                // answering no would be the wrong answer rather than a
                // deliberate one.
                TriggerConditionDef::SourceMatches { object: predicate } => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .or_else(|| match self.retired_objects.get(&source) {
                        Some(crate::game::RetiredObject::Permanent { permanent, .. }) => {
                            Some(permanent.as_ref())
                        }
                        _ => None,
                    })
                    .is_some_and(|permanent| {
                        self.trigger_object_matches(
                            *predicate,
                            &self.trigger_event_object(permanent),
                            source,
                            false,
                        )
                    }),
                TriggerConditionDef::AttachedPermanentMatches { object: predicate } => self
                    .current_or_last_known_attached_host(source)
                    .and_then(|host| {
                        self.battlefield
                            .iter()
                            .find(|permanent| permanent.card.id == host)
                    })
                    .is_some_and(|host| {
                        self.trigger_object_matches(
                            *predicate,
                            &self.trigger_event_object(host),
                            source,
                            false,
                        )
                    }),
                // Read live off the source, so a card whose counters change
                // during a turn answers differently each time it is asked.
                TriggerConditionDef::SourceCounters {
                    kind,
                    comparison,
                    amount,
                } => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .is_some_and(|permanent| {
                        compare(&permanent.counters(*kind), *comparison, &u16::from(*amount))
                    }),
                // A permanent remembers how it was cast; a spell still on the
                // stack carries the cast signature itself, which is where a
                // "when you cast this spell, if it was kicked" trigger has to
                // read it -- the permanent does not exist yet.
                // Read from the permanent, which remembers it, or from the
                // spell still on the stack when the question comes earlier.
                TriggerConditionDef::SourceCastAtInstantSpeed => {
                    self.battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == source)
                        .is_some_and(|permanent| permanent.cast_at_instant_speed)
                        || self
                            .stack
                            .iter()
                            .find(|candidate| candidate.id == source)
                            .is_some_and(|candidate| candidate.cast_at_instant_speed)
                }
                TriggerConditionDef::SourceCastFromHand => {
                    self.battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == source)
                        .is_some_and(|permanent| permanent.cast_from_hand)
                        || self
                            .stack
                            .iter()
                            .find(|candidate| candidate.id == source)
                            .is_some_and(|candidate| candidate.cast_from_hand)
                }
                TriggerConditionDef::SourceCastWith(kind) => {
                    self.battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == source)
                        .is_some_and(|permanent| permanent.cast_alternative == Some(*kind))
                        || self.stack_object_cast_with(source) == Some(*kind)
                }
                TriggerConditionDef::SourceLoyalty { comparison, amount } => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .is_some_and(|permanent| {
                        compare(
                            &permanent.counters(CounterKind::Loyalty),
                            *comparison,
                            &u16::from(*amount),
                        )
                    }),
                // Counting the activation now resolving is what makes
                // "activated four or more times" true on the fourth one.
                TriggerConditionDef::SourceActivationsThisTurn { comparison, amount } => ability
                    .is_some_and(|origin| {
                        compare(
                            &self.ability_activations_this_turn(source, origin),
                            *comparison,
                            amount,
                        )
                    }),
                // Read now rather than when the ability was created, so a
                // delayed effect asks about the target as it is at that point.
                TriggerConditionDef::TargetMatches {
                    slot,
                    object: predicate,
                } => object.is_some_and(|(stack, scoped, _)| {
                    Self::chosen_targets(stack, scoped.target_slot(*slot)).any(|target| {
                        // A target is a permanent or a spell depending on
                        // what the slot names, and "if its mana value is 2 or
                        // less" is asked of either -- Prohibit reads a spell
                        // on the stack where Overload reads a permanent.
                        let matched = match target {
                            Target::Permanent(id) => self
                                .battlefield
                                .iter()
                                .find(|permanent| permanent.card.id == id)
                                .map(|permanent| self.trigger_event_object(permanent)),
                            Target::Spell(id) => self
                                .stack
                                .iter()
                                .find(|candidate| candidate.id == id)
                                .and_then(|candidate| self.stack_trigger_event_object(candidate)),
                            // A card in a hidden zone has no continuous
                            // effects to read; nothing targets one this way.
                            Target::Player(_) | Target::Card(_) => None,
                        };
                        matched.is_some_and(|matched| {
                            self.trigger_object_matches(*predicate, &matched, source, false)
                        })
                    })
                }),
                TriggerConditionDef::SourceDealtDamageToOpponentThisTurn => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .is_some_and(|permanent| permanent.dealt_damage_to_opponent_this_turn),
                TriggerConditionDef::SourceIsPaired => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .is_some_and(|permanent| permanent.paired_with.is_some()),
                TriggerConditionDef::SourceIsTapped => self.current_or_last_known_tapped(source),
                TriggerConditionDef::SourceIsUntapped => !self.current_or_last_known_tapped(source),
                TriggerConditionDef::ControllerLifeAtMost(threshold) => {
                    i32::from(self.players[controller.index()].life) <= i32::from(*threshold)
                }
                // Rounded up, as every "half your starting life total" clause
                // is: at twenty the boundary is ten, and at an odd total it
                // is the higher half.
                TriggerConditionDef::ControllerLifeAtMostHalfStartingLife => {
                    let starting = i32::from(self.format.rules().starting_life);
                    i32::from(self.players[controller.index()].life) <= starting.div_euclid(2)
                }
                TriggerConditionDef::ObjectCount { .. } => {
                    unreachable!("the object-count arm is destructured above")
                }
            };
        };
        let mut count = 0;
        let result = self.visit_objects_matching_query_with_prospective(
            *query,
            controller,
            source,
            context,
            None,
            object,
            |_| {
                count += 1;
                ControlFlow::Continue(())
            },
        );
        debug_assert!(result.is_continue());
        compare(&i64::from(count), *comparison, &i64::from(*amount))
    }
}
