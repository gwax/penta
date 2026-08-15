use super::{
    AbilityDef, AbilityId, AbilityOperationDef, AbilityOrigin, AbilitySourceRef,
    AbilityTargetPredicate, AppliedEffectDef, AppliedRuleDef, CardPartId, CastSignature,
    CharacteristicOperationDef, ColorSet, ComparisonDef, ContinuousEffectExpiration,
    ContinuousEffectTimestamp, ControlFlow, CounterKind, EffectRecipientDef, EffectRecipientSetDef,
    EffectResolutionContext, Game, GameObjectId, GrantId, ManaColor, ObjectPredicateDef,
    ObjectQueryDef, ObjectRefDef, ObjectSetDef, Permanent, PlayerId, PlayerRefDef, PlayerSetDef,
    PowerToughnessOperationDef, QuantifierDef, ResolvedAbilityOperation, ResolvedContinuousEffect,
    ResolvedContinuousEffectKind, ResolvedDamageRedirect, ResolvedEffectDurationDef,
    ResolvedPlayRestriction, ResolvedPowerToughnessOperation, ScopedEffect, StackObject,
    StackObjectKind, Target, TargetIndex, TargetSelection, TargetSlotId, TemporaryAbilityGrant,
    TriggerConditionDef, TriggerContext, ZoneKind,
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
            for (index, component) in components.iter().copied().enumerate() {
                let component_order = u16::try_from(index)
                    .expect("one applied effect contains at most 65,536 components");
                self.apply_applied_effect_component(
                    target,
                    component,
                    ResolvedAppliedEffect {
                        component_order,
                        ..base_resolution
                    },
                );
            }
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
                    PowerToughnessOperationDef::Modify { power, toughness } => {
                        ResolvedPowerToughnessOperation::Modify {
                            power: freeze(power),
                            toughness: freeze(toughness),
                        }
                    }
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
        self.card_in_nonbattlefield_zone(object)
            .is_some()
            .then_some(Target::Card(object))
    }

    fn raw_target_reference(
        slot: TargetIndex,
        object: &StackObject,
        scoped: ScopedEffect,
    ) -> Option<Target> {
        Self::chosen_targets(object, scoped.target_slot(slot)).next()
    }

    fn object_reference_target(
        &self,
        reference: ObjectRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<Target> {
        match reference {
            ObjectRefDef::Source => object.source.map(Target::Permanent),
            ObjectRefDef::ResolvingObject => self.live_object_target(object.id),
            ObjectRefDef::Binding(binding) => context.single_object(binding),
            ObjectRefDef::AttachedToSource => object
                .source
                .and_then(|source| self.current_or_last_known_attached_host(source))
                .map(Target::Permanent),
            ObjectRefDef::Target(target) => {
                let slot = scoped.target_slot(target);
                Self::raw_target_reference(target, object, scoped)
                    .filter(|target| !matches!(target, Target::Player(_)))
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
            }
            ObjectRefDef::TriggeringObject => context
                .trigger
                .object
                .and_then(|triggering| self.live_object_target(triggering)),
        }
    }

    fn object_reference_id(
        &self,
        reference: ObjectRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<GameObjectId> {
        match reference {
            ObjectRefDef::Source => object.source,
            ObjectRefDef::ResolvingObject => Some(object.id),
            ObjectRefDef::Binding(binding) => {
                context
                    .single_object(binding)
                    .and_then(|target| match target {
                        Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => Some(id),
                        Target::Player(_) => None,
                    })
            }
            ObjectRefDef::AttachedToSource => object
                .source
                .and_then(|source| self.current_or_last_known_attached_host(source)),
            ObjectRefDef::Target(target) => {
                let slot = scoped.target_slot(target);
                Self::raw_target_reference(target, object, scoped)
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .and_then(|target| match target {
                        Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => Some(id),
                        Target::Player(_) => None,
                    })
            }
            ObjectRefDef::TriggeringObject => context.trigger.object,
        }
    }

    fn player_reference(
        &self,
        reference: PlayerRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<PlayerId> {
        match reference {
            PlayerRefDef::EffectController => Some(object.controller),
            PlayerRefDef::EventPlayer => context.trigger.event_player,
            PlayerRefDef::Target(target) => {
                let slot = scoped.target_slot(target);
                Self::chosen_targets(object, slot)
                    .find(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .and_then(|target| match target {
                        Target::Player(player) => Some(player),
                        Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                    })
            }
            // A direct object recipient still checks whether its target is
            // legal. Derived identity is different: a later instruction in
            // the same resolving effect may ask who controlled or owned an
            // object that an earlier instruction already moved. Preserve the
            // announced target here and answer from last-known information.
            PlayerRefDef::ControllerOf(ObjectRefDef::Target(target)) => {
                Self::raw_target_reference(target, object, scoped).and_then(|target| match target {
                    Target::Player(player) => Some(player),
                    Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => {
                        self.current_or_last_known_controller(id)
                    }
                })
            }
            PlayerRefDef::OwnerOf(ObjectRefDef::Target(target)) => {
                Self::raw_target_reference(target, object, scoped).and_then(|target| match target {
                    Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => {
                        self.current_or_last_known_owner(id)
                    }
                    Target::Player(_) => None,
                })
            }
            PlayerRefDef::ControllerOf(ObjectRefDef::TriggeringObject) => context
                .trigger
                .object
                .and_then(|triggering| self.current_or_last_known_controller(triggering))
                .or(context.trigger.object_controller),
            PlayerRefDef::ControllerOf(reference) => self
                .object_reference_id(reference, object, context, scoped)
                .and_then(|referenced| self.current_or_last_known_controller(referenced)),
            PlayerRefDef::OwnerOf(reference) => self
                .object_reference_id(reference, object, context, scoped)
                .and_then(|referenced| self.current_or_last_known_owner(referenced)),
        }
    }

    fn players_in_set(
        &self,
        players: PlayerSetDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<PlayerId> {
        match players {
            PlayerSetDef::All => vec![object.controller, object.controller.opponent()],
            PlayerSetDef::One(reference) => self
                .player_reference(reference, object, context, scoped)
                .into_iter()
                .collect(),
            PlayerSetDef::LegalTargets(target) => {
                let slot = scoped.target_slot(target);
                Self::chosen_targets(object, slot)
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .filter_map(|target| match target {
                        Target::Player(player) => Some(player),
                        Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                    })
                    .collect()
            }
            PlayerSetDef::Related(relation) => [object.controller, object.controller.opponent()]
                .into_iter()
                .filter(|candidate| {
                    self.player_relation_matches(
                        *candidate,
                        relation,
                        object.controller,
                        context.trigger,
                    )
                })
                .collect(),
        }
    }

    pub(super) fn effect_object_reference_id(
        &self,
        reference: ObjectRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<GameObjectId> {
        self.object_reference_id(reference, object, context, scoped)
    }

    pub(super) fn effect_player_reference(
        &self,
        reference: PlayerRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<PlayerId> {
        self.player_reference(reference, object, context, scoped)
    }

    pub(super) fn effect_players(
        &self,
        players: PlayerSetDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<PlayerId> {
        self.players_in_set(players, object, context, scoped)
    }

    fn objects_sharing_name_with_reference(
        &self,
        reference: ObjectRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<Target> {
        if let ObjectRefDef::Target(target) = reference {
            return self.objects_sharing_name_with_target(scoped.target_slot(target), object);
        }
        let Some(name) = self
            .object_reference_id(reference, object, context, scoped)
            .and_then(|referenced| self.object_card_name(referenced))
        else {
            return Vec::new();
        };
        self.battlefield
            .iter()
            .filter(|permanent| self.permanent_card_name(permanent.card.id) == Some(name))
            .map(|permanent| Target::Permanent(permanent.card.id))
            .collect()
    }

    pub(super) fn effect_recipients(
        &self,
        recipient: EffectRecipientDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<Target> {
        match recipient.0 {
            EffectRecipientSetDef::LegalTargets(target) => {
                let slot = scoped.target_slot(target);
                Self::chosen_targets(object, slot)
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .collect()
            }
            EffectRecipientSetDef::Objects(objects) => {
                self.effect_objects(objects, object, context, scoped)
            }
            EffectRecipientSetDef::Players(players) => self
                .players_in_set(players, object, context, scoped)
                .into_iter()
                .map(Target::Player)
                .collect(),
        }
    }

    pub(super) fn effect_objects(
        &self,
        objects: ObjectSetDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<Target> {
        match objects {
            ObjectSetDef::One(reference) => self
                .object_reference_target(reference, object, context, scoped)
                .into_iter()
                .collect(),
            ObjectSetDef::LegalTargets(target) => {
                let slot = scoped.target_slot(target);
                Self::chosen_targets(object, slot)
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .filter(|target| !matches!(target, Target::Player(_)))
                    .collect()
            }
            ObjectSetDef::Binding(binding) => context.object_group(binding).to_vec(),
            ObjectSetDef::Query(query) => {
                self.objects_matching_effect_query(query, object, context, scoped)
            }
            ObjectSetDef::SharingNameWith(reference) => {
                self.objects_sharing_name_with_reference(reference, object, context, scoped)
            }
        }
    }

    /// Whether a trigger's intervening-if condition holds right now. Rule
    /// 603.4 asks this when the ability would trigger and again as it
    /// resolves, so both call sites read the same board.
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
                TriggerConditionDef::SourceOnBattlefield => self
                    .battlefield
                    .iter()
                    .any(|permanent| permanent.card.id == source),
                TriggerConditionDef::SourceUntapped => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .is_some_and(|permanent| !permanent.tapped),
                TriggerConditionDef::ActivePlayer(relation) => {
                    self.player_relation_matches(self.active_player, *relation, controller, context)
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
                        matches!(target, Target::Permanent(id)
                        if self
                            .battlefield
                            .iter()
                            .find(|permanent| permanent.card.id == id)
                            .is_some_and(|permanent| {
                                self.trigger_object_matches(
                                    *predicate,
                                    &self.trigger_event_object(permanent),
                                    source,
                                    false,
                                )
                            }))
                    })
                }),
                TriggerConditionDef::SourceDealtDamageToOpponentThisTurn => self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .is_some_and(|permanent| permanent.dealt_damage_to_opponent_this_turn),
                TriggerConditionDef::SourceIsTapped => self.current_or_last_known_tapped(source),
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

include!("effect_support/custom_predicates.rs");
