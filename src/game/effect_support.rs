use super::{
    AbilityDef, AbilityId, AbilityOperationDef, AbilityOrigin, AbilitySourceRef,
    AbilityTargetPredicate, AppliedEffectDef, AppliedRuleDef, CardPartId, CastSignature,
    CharacteristicOperationDef, ColorChoiceOperationDef, ColorSet, ComparisonDef,
    ContinuousEffectExpiration, ContinuousEffectTimestamp, ControlFlow, CounterKind,
    EffectRecipientDef, EffectRecipientSetDef, EffectResolutionContext, Game, GameObjectId,
    GrantId, ManaColor, ObjectPredicateDef, ObjectQueryDef, ObjectRefDef, ObjectSetDef, Permanent,
    PlayerId, PlayerRefDef, PlayerSetDef, PowerToughnessOperationDef, QuantifierDef,
    ResolvedAbilityOperation, ResolvedContinuousEffect, ResolvedContinuousEffectKind,
    ResolvedDamageRedirect, ResolvedEffectDurationDef, ResolvedPlayRestriction,
    ResolvedPowerToughnessOperation, ScopedEffect, StackObject, StackObjectKind, Target,
    TargetIndex, TargetSelection, TargetSlotId, TemporaryAbilityGrant, TriggerConditionDef,
    TriggerContext, ZoneKind, abilities,
};

#[derive(Clone, Copy)]
struct ResolvedAppliedEffect<'a> {
    duration: ResolvedEffectDurationDef,
    timestamp: ContinuousEffectTimestamp,
    object: &'a StackObject,
    context: &'a EffectResolutionContext,
    scoped: ScopedEffect,
    component_order: u16,
}

mod queries;

impl Game {
    /// One protection ability per colour, so the chosen one has a static
    /// grant to point at. A granted ability is borrowed for the life of the
    /// game, which a colour picked at resolution cannot supply on its own.
    const PROTECTION_FROM_COLOR: [AbilityDef; 5] = [
        abilities::protection_from(ManaColor::White),
        abilities::protection_from(ManaColor::Blue),
        abilities::protection_from(ManaColor::Black),
        abilities::protection_from(ManaColor::Red),
        abilities::protection_from(ManaColor::Green),
    ];

    /// The single-colour sets, in the same order the choice offers them.
    const CHOSEN_COLOR_SETS: [ColorSet; 5] = [
        ColorSet::from_colors(&[ManaColor::White]),
        ColorSet::from_colors(&[ManaColor::Blue]),
        ColorSet::from_colors(&[ManaColor::Black]),
        ColorSet::from_colors(&[ManaColor::Red]),
        ColorSet::from_colors(&[ManaColor::Green]),
    ];

    /// Apply a named colour to everything the choice was resolved against.
    /// `index` is a position in [`Self::CHOOSABLE_COLORS`], which is what the
    /// decision offered.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_chosen_color(
        &mut self,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
        targets: &[Target],
        operation: ColorChoiceOperationDef,
        duration: ResolvedEffectDurationDef,
        index: usize,
    ) {
        let effect = match operation {
            ColorChoiceOperationDef::ProtectionFromChosenColor => {
                AppliedEffectDef::add_ability(&Self::PROTECTION_FROM_COLOR[index])
            }
            ColorChoiceOperationDef::BecomesChosenColor => {
                AppliedEffectDef::set_colors(Self::CHOSEN_COLOR_SETS[index])
            }
        };
        let timestamp = self.allocate_continuous_effect_timestamp();
        for target in targets {
            self.apply_applied_effect_component(
                *target,
                effect,
                ResolvedAppliedEffect {
                    duration,
                    timestamp,
                    object,
                    context,
                    scoped,
                    component_order: 0,
                },
            );
        }
    }

    /// Attach a continuous effect to a permanent that has just arrived,
    /// under the resolving object's own source and timestamp. It lasts as
    /// long as the permanent does, which is what "that creature is a black
    /// Zombie" means with no duration printed.
    pub(super) fn apply_arrival_effect(
        &mut self,
        arrived: GameObjectId,
        effect: AppliedEffectDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let timestamp = self.allocate_continuous_effect_timestamp();
        let mut components = Vec::new();
        Self::flatten_applied_effect(effect, &mut components);
        for (index, component) in components.into_iter().enumerate() {
            let component_order = u16::try_from(index)
                .expect("one applied effect contains at most 65,536 components");
            self.apply_applied_effect_component(
                Target::Permanent(arrived),
                component,
                ResolvedAppliedEffect {
                    duration: ResolvedEffectDurationDef::Permanent,
                    timestamp,
                    object,
                    context,
                    scoped,
                    component_order,
                },
            );
        }
    }

    pub(super) fn resolve_applied_effect(
        &mut self,
        recipient: EffectRecipientDef,
        effect: AppliedEffectDef,
        duration: ResolvedEffectDurationDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let timestamp = self.allocate_continuous_effect_timestamp();
        let base_resolution = ResolvedAppliedEffect {
            duration,
            timestamp,
            object,
            context,
            scoped,
            component_order: 0,
        };
        let mut components = Vec::new();
        Self::flatten_applied_effect(effect, &mut components);
        for target in self.effect_recipients(recipient, object, context, scoped) {
            Self::apply_components_to(self, target, &components, base_resolution);
        }
        // Everything else lasts until cleanup. Keeping the duration explicit
        // here makes unsupported permanent/granted effects visible rather
        // than silently changing their lifetime.
        debug_assert!(matches!(
            duration,
            ResolvedEffectDurationDef::UntilEndOfTurn
                | ResolvedEffectDurationDef::UntilEndOfCombat
                | ResolvedEffectDurationDef::Permanent
                | ResolvedEffectDurationDef::UntilYourNextUpkeep
                | ResolvedEffectDurationDef::UntilYourNextTurn
                | ResolvedEffectDurationDef::WhileSourceTapped
        ));
    }

    /// The same application, for recipients an effect has already resolved to
    /// concrete targets. Damage riders need this: which objects they apply to
    /// is decided by what the damage did, not by a recipient the runtime can
    /// evaluate a second time.
    pub(super) fn apply_effect_to_targets(
        &mut self,
        targets: &[Target],
        effect: AppliedEffectDef,
        duration: ResolvedEffectDurationDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let timestamp = self.allocate_continuous_effect_timestamp();
        let base_resolution = ResolvedAppliedEffect {
            duration,
            timestamp,
            object,
            context,
            scoped,
            component_order: 0,
        };
        let mut components = Vec::new();
        Self::flatten_applied_effect(effect, &mut components);
        for target in targets.iter().copied() {
            Self::apply_components_to(self, target, &components, base_resolution);
        }
    }

    fn apply_components_to(
        game: &mut Self,
        target: Target,
        components: &[AppliedEffectDef],
        base_resolution: ResolvedAppliedEffect<'_>,
    ) {
        for (index, component) in components.iter().copied().enumerate() {
            let component_order = u16::try_from(index)
                .expect("one applied effect contains at most 65,536 components");
            game.apply_applied_effect_component(
                target,
                component,
                ResolvedAppliedEffect {
                    component_order,
                    ..base_resolution
                },
            );
        }
    }

    fn flatten_applied_effect(effect: AppliedEffectDef, components: &mut Vec<AppliedEffectDef>) {
        match effect {
            AppliedEffectDef::Composite(effects) => {
                for effect in effects {
                    Self::flatten_applied_effect(*effect, components);
                }
            }
            leaf => components.push(leaf),
        }
    }

    /// Where a granted ability lands: the supported nonbattlefield flashback
    /// case keeps its cleanup-bounded card grant, while a permanent records an
    /// ordered, duration-aware layer operation for every ability category.
    fn apply_nonbattlefield_granted_ability(
        &mut self,
        target: Target,
        ability: &'static AbilityDef,
    ) {
        let Target::Card(target) = target else {
            return;
        };
        let grant = TemporaryAbilityGrant {
            object: target,
            ability: *ability,
        };
        if self.card_in_nonbattlefield_zone(target).is_some()
            && !self.temporary_ability_grants.contains(&grant)
        {
            self.temporary_ability_grants.push(grant);
        }
    }

    pub(super) fn continuous_effect_expiration(
        duration: ResolvedEffectDurationDef,
        controller: PlayerId,
        turns_started: u32,
    ) -> ContinuousEffectExpiration {
        match duration {
            ResolvedEffectDurationDef::UntilEndOfTurn => ContinuousEffectExpiration::EndOfTurn,
            ResolvedEffectDurationDef::UntilEndOfCombat => ContinuousEffectExpiration::EndOfCombat,
            ResolvedEffectDurationDef::UntilYourNextUpkeep => {
                ContinuousEffectExpiration::UpkeepOf(controller)
            }
            ResolvedEffectDurationDef::UntilYourNextTurn => ContinuousEffectExpiration::TurnOf {
                player: controller,
                turn: turns_started.saturating_add(1),
            },
            ResolvedEffectDurationDef::Permanent => ContinuousEffectExpiration::Never,
            ResolvedEffectDurationDef::WhileSourceTapped => {
                ContinuousEffectExpiration::WhileSourceTapped
            }
        }
    }

    fn apply_applied_effect_component(
        &mut self,
        target: Target,
        effect: AppliedEffectDef,
        resolution: ResolvedAppliedEffect<'_>,
    ) {
        match effect {
            AppliedEffectDef::Composite(_) => {
                unreachable!("applied effects are flattened before dispatch")
            }
            AppliedEffectDef::Characteristic(operation) => {
                self.apply_characteristic_component(target, effect, operation, resolution);
            }
            AppliedEffectDef::Rule(rule) => {
                self.apply_rule_component(target, effect, rule, resolution);
            }
        }
    }

    fn apply_rule_component(
        &mut self,
        target: Target,
        definition: AppliedEffectDef,
        rule: AppliedRuleDef,
        resolution: ResolvedAppliedEffect<'_>,
    ) {
        // `CannotBeCountered` is meaningful on a stack object, whose lifetime
        // is already represented by `AppliedStackEffect`. A resolving Apply
        // program cannot honestly give it one of the permanent durations.
        debug_assert_ne!(rule, AppliedRuleDef::CannotBeCountered);
        if rule == AppliedRuleDef::CannotBeCountered {
            return;
        }
        let expiration = Self::continuous_effect_expiration(
            resolution.duration,
            resolution.object.controller,
            self.turns_started[resolution.object.controller.index()],
        );
        if let AppliedRuleDef::RedirectDamageFromTo {
            source,
            destination,
        } = rule
        {
            let Target::Player(player) = target else {
                return;
            };
            let Some(source) = self.effect_object_reference_id(
                source,
                resolution.object,
                resolution.context,
                resolution.scoped,
            ) else {
                return;
            };
            let Some(destination) = self.effect_object_reference_id(
                destination,
                resolution.object,
                resolution.context,
                resolution.scoped,
            ) else {
                return;
            };
            self.damage_redirects.push(ResolvedDamageRedirect {
                player,
                source,
                destination,
                expiration,
            });
            return;
        }
        let source = AbilitySourceRef {
            object: resolution.object.source.unwrap_or(resolution.object.id),
            ability: resolution
                .object
                .ability_origin()
                .unwrap_or(AbilityOrigin::Printed {
                    definition: resolution.object.presentation_definition(),
                    part: CardPartId::PRIMARY,
                    ability: AbilityId::PRIMARY,
                }),
        };
        if let AppliedRuleDef::CannotPlay(restriction) = rule {
            let Target::Player(affected_player) = target else {
                return;
            };
            self.resolved_play_restrictions
                .push(ResolvedPlayRestriction {
                    definition,
                    source,
                    affected_player,
                    timestamp: resolution.timestamp,
                    component_order: resolution.component_order,
                    expiration,
                    restriction,
                });
            return;
        }
        let Target::Permanent(target) = target else {
            return;
        };
        if let Some(permanent) = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == target)
        {
            permanent
                .resolved_continuous_effects
                .push(ResolvedContinuousEffect {
                    definition,
                    source,
                    timestamp: resolution.timestamp,
                    component_order: resolution.component_order,
                    expiration,
                    kind: ResolvedContinuousEffectKind::Rule(rule),
                });
        }
    }

    fn apply_characteristic_component(
        &mut self,
        target: Target,
        definition: AppliedEffectDef,
        operation: CharacteristicOperationDef,
        resolution: ResolvedAppliedEffect<'_>,
    ) {
        if let CharacteristicOperationDef::Abilities(AbilityOperationDef::Add(ability)) = operation
            && matches!(target, Target::Card(_))
        {
            self.apply_nonbattlefield_granted_ability(target, ability);
            return;
        }
        if let (Target::Spell(target), CharacteristicOperationDef::Colors(operation)) =
            (target, operation)
        {
            let current = ManaColor::COLORS
                .into_iter()
                .zip(self.object_colors(target))
                .filter_map(|(color, present)| present.then_some(color))
                .fold(ColorSet::empty(), ColorSet::with);
            let colors = Self::apply_color_operation(current, operation);
            if let Some(spell) = self.stack.iter_mut().find(|spell| spell.id == target) {
                spell.colors = Some(colors);
            }
            return;
        }
        let Target::Permanent(target) = target else {
            return;
        };

        let Some(kind) = self.resolve_characteristic_kind(target, operation, resolution) else {
            return;
        };
        let source = AbilitySourceRef {
            object: resolution.object.source.unwrap_or(resolution.object.id),
            ability: resolution
                .object
                .ability_origin()
                .unwrap_or(AbilityOrigin::Printed {
                    definition: resolution.object.presentation_definition(),
                    part: CardPartId::PRIMARY,
                    ability: AbilityId::PRIMARY,
                }),
        };
        let expiration = Self::continuous_effect_expiration(
            resolution.duration,
            resolution.object.controller,
            self.turns_started[resolution.object.controller.index()],
        );
        if let Some(permanent) = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == target)
        {
            permanent
                .resolved_continuous_effects
                .push(ResolvedContinuousEffect {
                    definition,
                    source,
                    timestamp: resolution.timestamp,
                    component_order: resolution.component_order,
                    expiration,
                    kind,
                });
        }
    }

    fn resolve_characteristic_kind(
        &self,
        target: GameObjectId,
        operation: CharacteristicOperationDef,
        resolution: ResolvedAppliedEffect<'_>,
    ) -> Option<ResolvedContinuousEffectKind> {
        Some(match operation {
            CharacteristicOperationDef::Abilities(AbilityOperationDef::Add(ability)) => {
                let permanent = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == target)?;
                let mut used_grants = [false; 256];
                for grant in permanent
                    .resolved_continuous_effects
                    .iter()
                    .filter_map(|effect| match effect.kind {
                        ResolvedContinuousEffectKind::Abilities(
                            ResolvedAbilityOperation::Add { grant, .. },
                        ) => Some(grant),
                        _ => None,
                    })
                {
                    used_grants[grant.index()] = true;
                }
                let grant = used_grants
                    .iter()
                    .position(|used| !used)
                    .and_then(GrantId::from_index)
                    .expect("one permanent has at most 256 active resolved grants");
                ResolvedContinuousEffectKind::Abilities(ResolvedAbilityOperation::Add {
                    ability: *ability,
                    grant,
                })
            }
            CharacteristicOperationDef::Abilities(AbilityOperationDef::Remove(predicate)) => {
                ResolvedContinuousEffectKind::Abilities(ResolvedAbilityOperation::Remove(predicate))
            }
            CharacteristicOperationDef::BasicLandTypes(operation) => {
                ResolvedContinuousEffectKind::BasicLandTypes(operation)
            }
            CharacteristicOperationDef::CardTypes(operation) => {
                ResolvedContinuousEffectKind::CardTypes(operation)
            }
            CharacteristicOperationDef::Colors(operation) => {
                ResolvedContinuousEffectKind::Colors(operation)
            }
            CharacteristicOperationDef::CreatureTypes(operation) => {
                ResolvedContinuousEffectKind::CreatureTypes(operation)
            }
            CharacteristicOperationDef::PowerToughness(operation) => {
                let freeze = |value| {
                    i16::try_from(
                        self.effect_value(
                            value,
                            resolution.object,
                            resolution.context,
                            resolution.scoped,
                        )
                        .clamp(i32::from(i16::MIN), i32::from(i16::MAX)),
                    )
                    .expect("the effect value was clamped to i16")
                };
                ResolvedContinuousEffectKind::PowerToughness(match operation {
                    PowerToughnessOperationDef::SetBase { power, toughness } => {
                        ResolvedPowerToughnessOperation::SetBase {
                            power: freeze(power),
                            toughness: freeze(toughness),
                        }
                    }
                    PowerToughnessOperationDef::SetBasePower(power) => {
                        ResolvedPowerToughnessOperation::SetBasePower {
                            power: freeze(power),
                        }
                    }
                    PowerToughnessOperationDef::SetBaseToughness(toughness) => {
                        ResolvedPowerToughnessOperation::SetBaseToughness {
                            toughness: freeze(toughness),
                        }
                    }
                    PowerToughnessOperationDef::Modify { power, toughness } => {
                        ResolvedPowerToughnessOperation::Modify {
                            power: freeze(power),
                            toughness: freeze(toughness),
                        }
                    }
                    // Nothing to freeze: the switch names no value.
                    PowerToughnessOperationDef::Switch => ResolvedPowerToughnessOperation::Switch,
                })
            }
        })
    }

    pub(super) fn live_object_target(&self, object: GameObjectId) -> Option<Target> {
        if self
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == object)
        {
            return Some(Target::Permanent(object));
        }
        if self.stack.iter().any(|candidate| candidate.id == object) {
            return Some(Target::Spell(object));
        }
        if self.card_in_nonbattlefield_zone(object).is_some() {
            return Some(Target::Card(object));
        }
        // A trigger captured on the battlefield names the object that was
        // standing there, and the card it became on the way out has a
        // different identity. "Return it to its owner's hand" means that
        // card, so follow the move rather than finding nothing.
        let successor = self.successors.get(&object).copied()?;
        self.card_in_nonbattlefield_zone(successor)
            .is_some()
            .then_some(Target::Card(successor))
    }

    /// How many times this ability has been activated from this permanent so
    /// far this turn.
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
                TriggerConditionDef::CreatureDiedThisTurn => self.creature_died_this_turn,
                TriggerConditionDef::SourceMatches { object: predicate } => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
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

    /// How much of a divided total one target takes, read off the selection
    /// frozen when the object was put on the stack.
    pub(super) fn divided_share(
        object: &StackObject,
        slot: TargetSlotId,
        target: Target,
    ) -> Option<u16> {
        object
            .signature
            .as_ref()
            .map(CastSignature::targets)
            .or_else(|| {
                object
                    .ability
                    .as_ref()
                    .map(|ability| ability.targets.as_slice())
            })?
            .iter()
            .find(|selection| selection.slot() == slot)?
            .amount_for(target)
    }

    /// The targets frozen into one slot when the object was put on the stack,
    /// before any legality check.
    pub(super) fn chosen_targets(
        object: &StackObject,
        slot: TargetSlotId,
    ) -> impl Iterator<Item = Target> {
        object
            .signature
            .as_ref()
            .map(CastSignature::targets)
            .or_else(|| {
                object
                    .ability
                    .as_ref()
                    .map(|ability| ability.targets.as_slice())
            })
            .and_then(|selections| selections.iter().find(|selection| selection.slot() == slot))
            .into_iter()
            .flat_map(TargetSelection::targets)
            .copied()
    }

    pub(super) fn stack_ability_target_is_legal(
        &self,
        object: &StackObject,
        slot: TargetSlotId,
        target: Target,
    ) -> bool {
        let source = object.source.unwrap_or(object.id);
        let Some(ability) = &object.ability else {
            return true;
        };
        let Some(definition) = ability.target_defs.get(slot.index()) else {
            // Legacy custom actions can carry targets without a declarative
            // target slot. Their historic resolver remains authoritative.
            return true;
        };
        if Self::ability_target_uses_custom_predicate(definition.predicate) {
            // Custom activated handlers offered these targets before the
            // shared predicate vocabulary could express their full legality.
            // Preserve their prior zone-presence check until the named
            // predicate itself is migrated; treating `Special` as no matches
            // would incorrectly counter every such ability on resolution.
            return match target {
                Target::Player(_) => true,
                Target::Card(id) => self.card_in_nonbattlefield_zone(id).is_some(),
                Target::Permanent(id) => self
                    .battlefield
                    .iter()
                    .any(|permanent| permanent.card.id == id),
                Target::Spell(id) => self.stack.iter().any(|candidate| candidate.id == id),
            };
        }
        self.ability_targets_matching(
            definition.predicate,
            object.controller,
            source,
            ability.context.trigger,
        )
        .contains(&target)
    }
}

include!("effect_support/references.rs");
include!("effect_support/custom_predicates.rs");
