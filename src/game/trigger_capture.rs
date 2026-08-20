use crate::card::{BattlefieldEntryChoiceDestinationDef, DamageSourceGroupDef};

use crate::CharacteristicContext;

use super::{
    AbilityDef, AbilityId, AbilityOrigin, AbilityProcedureDef, AbilitySourceRef, AddManaEffectDef,
    BattlefieldTriggerListener, CardDefinitionId, CardPartId, CardType, CommittedTriggerEvent,
    DamageEventMatcherDef, DamageKindDef, DamageRecipientMatcherDef, DamageSourceMatcherDef,
    DeclarativeAbilityDef, EffectDef, EffectRecipientSetDef, EffectResolutionContext,
    EffectiveAbility, FrozenActivatedAbility, Game, GameEvent, GameObjectId,
    InstalledTriggerLifetime, KeywordAbility, Mana, ManaSelectionDef, ManaSource,
    ObjectPredicateDef, ObjectRefDef, ObjectSetDef, PendingTrigger, Permanent, PlayerId,
    PlayerRefDef, PlayerRelation, PlayerSetDef, RetiredObject, ScopedEffect, StackAbilityResolver,
    TapPurposeDef, Target, TriggerCapture, TriggerContext, TriggerEventDef, TriggerEventObject,
    ZoneKind,
};

mod graveyard;

impl Game {
    /// Finishes an atomic rules procedure before a player can receive
    /// priority. Mana abilities invoked while casting resolve inside the
    /// procedure, while ordinary triggers collected by them wait here.
    pub(super) fn finish_rules_procedure(&mut self) {
        // A decision can be one step in a still-resolving spell or turn-based
        // procedure. Neither state-based actions nor trigger placement happen
        // in the middle of that procedure: for example, a creature dealt
        // lethal damage by Chain Lightning can still activate a mana ability
        // when its controller is asked whether to pay for the copy. Drain the
        // continuation chain before reaching either priority-boundary check.
        loop {
            if self.pending_decisions.is_empty() && !self.pending_events.is_empty() {
                self.continue_pending_events();
            }
            if !self.pending_decisions.is_empty() || !self.pending_events.is_empty() {
                return;
            }
            if self.pending_procedures.is_empty() {
                break;
            }
            self.continue_pending_procedures();
        }

        self.check_state_based_actions();
        if self.result.is_none()
            && self.pending_decisions.is_empty()
            && self.pending_events.is_empty()
            && self.pending_procedures.is_empty()
        {
            self.begin_trigger_placement();
        }
    }

    pub(super) fn capture_trigger(&mut self, capture: &TriggerCapture) {
        // Rule 603.4: an intervening-if condition is checked as the ability
        // would trigger. Failing it means the ability never triggers at all,
        // so nothing reaches the stack and nothing is reported.
        if !self.trigger_capture_condition_holds(capture) {
            return;
        }
        self.capture_trigger_prechecked(capture);
    }

    fn trigger_capture_condition_holds(&self, capture: &TriggerCapture) -> bool {
        capture.condition.is_none_or(|condition| {
            self.trigger_condition_holds(
                condition,
                capture.source.object,
                capture.controller,
                capture.context.trigger,
                Some(capture.source.ability),
                None,
            )
        })
    }

    fn capture_trigger_prechecked(&mut self, capture: &TriggerCapture) {
        let id = self.next_trigger_id;
        self.next_trigger_id = self.next_trigger_id.saturating_add(1);
        self.pending_triggers.push(PendingTrigger {
            id,
            source: capture.source,
            definition: capture.definition,
            owner: capture.owner,
            controller: capture.controller,
            text: capture.text,
            target_defs: capture.target_defs.clone(),
            targets: capture.targets.clone(),
            effect: capture.effect,
            resolver: capture.resolver,
            context: capture.context.clone(),
            condition: capture.condition,
            x: capture.x,
        });
        self.events.push(GameEvent::AbilityTriggered {
            player: capture.controller,
            trigger: id,
            source: capture.source.object,
            definition: capture.definition,
        });
    }

    pub(super) const fn ability_presentation_definition(
        origin: AbilityOrigin,
        fallback: CardDefinitionId,
    ) -> CardDefinitionId {
        match origin {
            AbilityOrigin::Printed { definition, .. } => definition,
            AbilityOrigin::IntrinsicBasicLand(_) | AbilityOrigin::Granted { .. } => fallback,
        }
    }

    pub(super) fn capture_battlefield_triggers(&mut self, event: &CommittedTriggerEvent) {
        let listeners = self.battlefield_trigger_listeners();
        self.capture_battlefield_triggers_from_snapshot(&listeners, event);
    }

    /// "When you cycle this card" (CR 702.29b), raised as the cycling ability
    /// is activated. Only the cycled card can carry the clause, so its own
    /// printed abilities are the entire listener list -- there is no zone to
    /// scan. The card is read in the graveyard the discard cost has already
    /// put it in, which is also the object the trigger names.
    pub(super) fn capture_cycling_triggers(&mut self, cycled: GameObjectId, player: PlayerId) {
        let Some((_zone, card)) = self.card_in_nonbattlefield_zone(cycled) else {
            return;
        };
        let card = card.clone();
        let Some(object) = self.printed_trigger_event_object(
            cycled,
            card.definition,
            player,
            &CharacteristicContext::Graveyard,
        ) else {
            return;
        };
        let mut listeners = Vec::new();
        self.for_each_printed_card_ability(&card, &CharacteristicContext::Graveyard, |effective| {
            let ability = effective.ability;
            let DeclarativeAbilityDef::Triggered(definition) = ability.definition else {
                return;
            };
            if !ability.is_executable()
                || definition.event != TriggerEventDef::Cycled
                || definition.procedure != AbilityProcedureDef::Shared
            {
                return;
            }
            listeners.push(BattlefieldTriggerListener {
                event: definition.event,
                uses_stack: true,
                installed: None,
                capture: TriggerCapture {
                    source: AbilitySourceRef {
                        object: cycled,
                        ability: effective.origin,
                    },
                    definition: Self::ability_presentation_definition(
                        effective.origin,
                        card.definition,
                    ),
                    owner: card.owner,
                    controller: player,
                    text: ability.text,
                    target_defs: definition.targets.to_vec(),
                    targets: Vec::new(),
                    effect: ability.declarative_effect().unwrap_or(EffectDef::None),
                    resolver: Self::ability_resolver(effective.origin, &ability),
                    context: TriggerContext::empty().into(),
                    condition: definition.condition,
                    x: 0,
                },
            });
        });
        if listeners.is_empty() {
            return;
        }
        self.capture_battlefield_triggers_from_snapshot(
            &listeners,
            &CommittedTriggerEvent::Cycled { object },
        );
    }

    /// A spell's own "when you cast this spell" clause, raised as it is put on
    /// the stack. Storm is the case: the ability belongs to the spell rather
    /// than to anything on the battlefield, so the ordinary listener scan
    /// never sees it, and the spell it copies is still on the stack beneath
    /// the trigger when that trigger resolves.
    pub(super) fn capture_own_cast_triggers(&mut self, spell: GameObjectId) {
        let Some(cast) = self.stack.iter().find(|object| object.id == spell).cloned() else {
            return;
        };
        let Some(object) = self.stack_object_event_object(&cast) else {
            return;
        };
        let card = cast.card.clone();
        let Some(signature) = cast.signature.as_ref() else {
            return;
        };
        let context = CharacteristicContext::Stack {
            form: signature.form().clone(),
        };
        let mut listeners = Vec::new();
        self.for_each_printed_card_ability(&card, &context, |effective| {
            let ability = effective.ability;
            let DeclarativeAbilityDef::Triggered(definition) = ability.definition else {
                return;
            };
            if !ability.is_executable()
                || definition.event != TriggerEventDef::SpellCast(ObjectPredicateDef::Source)
                || definition.procedure != AbilityProcedureDef::Shared
            {
                return;
            }
            listeners.push(BattlefieldTriggerListener {
                event: definition.event,
                uses_stack: true,
                installed: None,
                capture: TriggerCapture {
                    source: AbilitySourceRef {
                        object: spell,
                        ability: effective.origin,
                    },
                    definition: Self::ability_presentation_definition(
                        effective.origin,
                        card.definition,
                    ),
                    owner: card.owner,
                    controller: cast.controller,
                    text: ability.text,
                    target_defs: definition.targets.to_vec(),
                    targets: Vec::new(),
                    effect: ability.declarative_effect().unwrap_or(EffectDef::None),
                    resolver: Self::ability_resolver(effective.origin, &ability),
                    context: TriggerContext::empty().into(),
                    condition: definition.condition,
                    x: 0,
                },
            });
        });
        if listeners.is_empty() {
            return;
        }
        self.capture_battlefield_triggers_from_snapshot(
            &listeners,
            &CommittedTriggerEvent::SpellCast { object },
        );
    }

    pub(super) fn battlefield_trigger_listeners(&self) -> Vec<BattlefieldTriggerListener> {
        let mut listeners = Vec::new();
        for permanent in &self.battlefield {
            self.for_each_effective_ability(permanent, |effective| {
                let ability = effective.ability;
                if !ability.is_executable() {
                    return;
                }
                let (definition, uses_stack) = match ability.definition {
                    DeclarativeAbilityDef::TriggeredMana(definition) => {
                        if ability.declarative_effect().is_none() {
                            return;
                        }
                        (definition, false)
                    }
                    DeclarativeAbilityDef::Triggered(definition) => (definition, true),
                    DeclarativeAbilityDef::Spell(_)
                    | DeclarativeAbilityDef::ActivatedMana(_)
                    | DeclarativeAbilityDef::Activated(_)
                    | DeclarativeAbilityDef::Static(_)
                    | DeclarativeAbilityDef::Replacement(_)
                    | DeclarativeAbilityDef::AlternativeCast(_)
                    | DeclarativeAbilityDef::SpecialAction(_)
                    | DeclarativeAbilityDef::Keyword(_)
                    | DeclarativeAbilityDef::Legacy => return,
                };
                // Compatibility procedures execute elsewhere, so admitting
                // them here would manufacture a duplicate trigger.
                if definition.procedure != AbilityProcedureDef::Shared {
                    return;
                }
                if !definition.source_zones.contains(&ZoneKind::Battlefield) {
                    return;
                }
                let source = AbilitySourceRef {
                    object: permanent.card.id,
                    ability: effective.origin,
                };
                listeners.push(BattlefieldTriggerListener {
                    event: definition.event,
                    uses_stack,
                    installed: None,
                    capture: TriggerCapture {
                        source,
                        definition: Self::ability_presentation_definition(
                            effective.origin,
                            Self::effective_rules_source(permanent).0,
                        ),
                        owner: permanent.card.owner,
                        controller: permanent.controller,
                        text: ability.text,
                        target_defs: definition.targets.to_vec(),
                        targets: Vec::new(),
                        effect: ability.declarative_effect().unwrap_or(EffectDef::None),
                        resolver: Self::ability_resolver(effective.origin, &ability),
                        context: TriggerContext::empty().into(),
                        condition: definition.condition,
                        x: 0,
                    },
                });
            });
        }
        self.extend_with_graveyard_trigger_listeners(&mut listeners);
        // Installed triggers listen the same way, minus a permanent to hang
        // on; they are appended last so a permanent's own triggers keep the
        // relative order they had before any existed.
        listeners.extend(self.installed_triggers.iter().map(|installed| {
            BattlefieldTriggerListener {
                event: installed.event,
                uses_stack: true,
                installed: Some(installed.id),
                capture: installed.capture.clone(),
            }
        }));
        listeners
    }

    pub(super) fn capture_battlefield_triggers_from_snapshot(
        &mut self,
        listeners: &[BattlefieldTriggerListener],
        event: &CommittedTriggerEvent,
    ) {
        self.capture_battlefield_trigger_batch_from_snapshot(
            listeners,
            std::slice::from_ref(event),
        );
    }

    /// Determine every match and intervening-if result for one atomic event
    /// batch before any triggered-mana ability can mutate the game. Attack
    /// declarations and simultaneous exits both publish more than one
    /// object-local event, but all of those facts belong to one rules event.
    pub(super) fn capture_battlefield_trigger_batch_from_snapshot(
        &mut self,
        listeners: &[BattlefieldTriggerListener],
        events: &[CommittedTriggerEvent],
    ) {
        self.capture_battlefield_trigger_batch_with_mana_resolver(
            listeners,
            events,
            |game, capture| {
                game.resolve_triggered_mana_effect(
                    capture.source,
                    capture.controller,
                    capture.effect,
                    &capture.context,
                );
            },
        );
    }

    pub(super) fn capture_battlefield_trigger_batch_with_mana_resolver(
        &mut self,
        listeners: &[BattlefieldTriggerListener],
        events: &[CommittedTriggerEvent],
        mut resolve_mana: impl FnMut(&mut Self, TriggerCapture),
    ) {
        let mut consumed_once = Vec::new();
        let mut matched = Vec::new();
        for event in events {
            for listener in listeners {
                if !self.trigger_event_matches_for_controller(
                    listener.event,
                    event,
                    listener.capture.source.object,
                    Some(listener.capture.controller),
                ) {
                    continue;
                }
                if let Some(id) = listener.installed
                    && self
                        .installed_triggers
                        .iter()
                        .find(|installed| installed.id == id)
                        .is_some_and(|installed| {
                            matches!(installed.lifetime, InstalledTriggerLifetime::Once)
                        })
                {
                    if consumed_once.contains(&id) {
                        continue;
                    }
                    // A once-only listener is consumed by the first matching
                    // event even when its intervening-if condition is false.
                    consumed_once.push(id);
                }
                let mut capture = listener.capture.clone();
                // Keep installer bindings and targets; only the committed
                // event-local context changes for this match.
                capture.context.trigger = event.context();
                let condition_holds = self.trigger_capture_condition_holds(&capture);
                matched.push((listener.uses_stack, capture, condition_holds));
            }
        }

        self.installed_triggers
            .retain(|installed| !consumed_once.contains(&installed.id));

        // Record ordinary triggers first, using the precomputed condition.
        // Any triggers caused while a triggered-mana ability resolves are
        // therefore later in the pending stream than the event that caused it.
        for (uses_stack, capture, condition_holds) in &matched {
            if *uses_stack && *condition_holds {
                self.capture_trigger_prechecked(capture);
            }
        }
        for (uses_stack, capture, condition_holds) in matched {
            if !uses_stack && condition_holds {
                resolve_mana(self, capture);
            }
        }
    }

    pub(super) fn resolve_triggered_mana_effect(
        &mut self,
        source: AbilitySourceRef,
        controller: PlayerId,
        effect: EffectDef,
        context: &EffectResolutionContext,
    ) {
        match effect {
            EffectDef::Sequence(effects) => {
                for effect in effects {
                    self.resolve_triggered_mana_effect(source, controller, *effect, context);
                }
            }
            EffectDef::AddMana(effect) => {
                self.resolve_triggered_add_mana_effect(source, controller, effect, context);
            }
            EffectDef::None
            | EffectDef::Randomized { .. }
            | EffectDef::Choose(_)
            | EffectDef::ChooseCardName { .. }
            | EffectDef::BindMatching { .. }
            | EffectDef::PayOr(_)
            | EffectDef::SplitIntoPiles(_)
            | EffectDef::PreventDamage { .. }
            | EffectDef::DealDamage { .. }
            | EffectDef::DealDamageAndApply { .. }
            | EffectDef::DrainLife { .. }
            | EffectDef::GainLife { .. }
            | EffectDef::AddPoisonCounters { .. }
            | EffectDef::DrawCards { .. }
            | EffectDef::Discard { .. }
            | EffectDef::DiscardCards { .. }
            | EffectDef::ShuffleLibrary { .. }
            | EffectDef::EmptyManaPool { .. }
            | EffectDef::LoseLife { .. }
            | EffectDef::LoseTheGame { .. }
            | EffectDef::AddManaEqualTo { .. }
            | EffectDef::Regenerate { .. }
            | EffectDef::Tap { .. }
            | EffectDef::RemoveFromCombat { .. }
            | EffectDef::DestroyAtEndOfCombat { .. }
            | EffectDef::SkipNextUntapSteps { .. }
            | EffectDef::RemoveAllCounters { .. }
            | EffectDef::Untap { .. }
            | EffectDef::Destroy { .. }
            | EffectDef::Sacrifice { .. }
            | EffectDef::SacrificeOfChoice { .. }
            | EffectDef::Mill { .. }
            | EffectDef::MillUntil { .. }
            | EffectDef::LookAtTopAndSelect { .. }
            | EffectDef::LookAtHand { .. }
            | EffectDef::RevealAtRandomFromHand { .. }
            | EffectDef::RevealHand { .. }
            | EffectDef::SearchZone { .. }
            | EffectDef::ChooseCards { .. }
            | EffectDef::ReplaceNextDrawThisTurn { .. }
            | EffectDef::IfFormat { .. }
            | EffectDef::Counter { .. }
            | EffectDef::ReturnSpellToHand { .. }
            | EffectDef::CopyResolvingSpell { .. }
            | EffectDef::AddCounters { .. }
            | EffectDef::RemoveCounters { .. }
            | EffectDef::ChangeTextBasicLandType { .. }
            | EffectDef::ChooseColor { .. }
            | EffectDef::BecomeCopyOf { .. }
            | EffectDef::May { .. }
            | EffectDef::CannotBeForcedToSacrifice
            | EffectDef::SubstituteBasicLandTypeUntilEndOfTurn { .. }
            | EffectDef::CreateEmblem { .. }
            | EffectDef::Transform { .. }
            | EffectDef::ScheduleTurnPhases(_)
            | EffectDef::TakeExtraTurn { .. }
            | EffectDef::BecomeMonarch { .. }
            | EffectDef::DamageCannotBePreventedThisTurn
            | EffectDef::GrantFlashToNextSorcery
            | EffectDef::ExileLinkedToSource { .. }
            | EffectDef::ReturnLinkedExiles { .. }
            | EffectDef::Detain { .. }
            | EffectDef::GainControl { .. }
            | EffectDef::ExchangeControl { .. }
            | EffectDef::IfCondition { .. }
            | EffectDef::InstallTrigger(_)
            | EffectDef::ReduceGenericCostBy(_)
            | EffectDef::IncreaseMatchingAbilityCostBy { .. }
            | EffectDef::IncreaseMatchingSpellCostBy { .. }
            | EffectDef::ReduceMatchingSpellCostBy { .. }
            | EffectDef::LandwalkCanBeBlocked(_)
            | EffectDef::CannotAttackUnless(_)
            | EffectDef::CannotAttackIf(_)
            | EffectDef::MoveToZone { .. }
            | EffectDef::Attach { .. }
            | EffectDef::PhaseOut { .. }
            | EffectDef::ReturnAttached { .. }
            | EffectDef::Reconfigure { .. }
            | EffectDef::PairWithSource { .. }
            | EffectDef::CreateToken { .. }
            | EffectDef::CreateAttachedToken { .. }
            | EffectDef::CreateTokenCopyOf { .. }
            | EffectDef::StaticApply { .. }
            | EffectDef::Apply { .. }
            | EffectDef::Special(_) => {
                // Choice-bearing and non-mana primitives need a dedicated
                // immediate procedure before a supported card can use them.
            }
        }
    }

    fn resolve_triggered_add_mana_effect(
        &mut self,
        source: AbilitySourceRef,
        controller: PlayerId,
        effect: AddManaEffectDef,
        context: &EffectResolutionContext,
    ) {
        let AddManaEffectDef {
            mana: ManaSelectionDef::One(kind),
            also: None,
            amount,
            restrictions,
            spend_effects,
            damage_to_controller,
            recipient,
            amount_override,
            variable_amount: _,
            sacrifice_source_when_out_of: _,
        } = effect
        else {
            return;
        };
        // A mana trigger resolves without ever going on the stack, so it has
        // no resolving object to read a general player reference from. The
        // two a printed clause asks for are the ability's own controller and
        // the controller of whatever was tapped.
        let controller = match recipient {
            PlayerRefDef::ControllerOf(ObjectRefDef::TriggeringObject) => context
                .trigger
                .object
                .and_then(|triggering| self.current_or_last_known_controller(triggering))
                .or(context.trigger.object_controller)
                .unwrap_or(controller),
            _ => controller,
        };
        let mana = Mana::from_ability(
            kind,
            ManaSource {
                object: source.object,
                ability: source.ability,
            },
            restrictions,
            spend_effects,
        );
        let amount = amount_override
            .filter(|override_| {
                self.static_condition_holds(override_.condition, controller, source.object)
            })
            .map_or(amount, |override_| override_.amount);
        self.add_mana(controller, std::iter::repeat_n(mana, usize::from(amount)));
        if damage_to_controller > 0 {
            self.damage_target_from(
                Some(source.object),
                Some(Target::Player(controller)),
                damage_to_controller,
            );
        }
    }

    pub(super) fn capture_custom_source_triggers(
        &mut self,
        source: &Permanent,
        abilities: &[EffectiveAbility],
        event: &CommittedTriggerEvent,
    ) {
        let triggers = abilities
            .iter()
            .filter_map(|effective| match effective.ability.definition {
                DeclarativeAbilityDef::Triggered(definition)
                    if effective.ability.is_executable()
                        && definition.procedure == AbilityProcedureDef::Legacy
                        && effective.ability.custom_behavior().is_some()
                        && definition.source_zones.contains(&ZoneKind::Battlefield)
                        && self.trigger_event_matches_for_controller(
                            definition.event,
                            event,
                            source.card.id,
                            Some(source.controller),
                        ) =>
                {
                    Some((
                        effective.origin,
                        effective.ability.text,
                        definition.targets,
                        effective
                            .ability
                            .declarative_effect()
                            .unwrap_or(EffectDef::None),
                        Self::ability_resolver(effective.origin, &effective.ability),
                    ))
                }
                DeclarativeAbilityDef::Spell(_)
                | DeclarativeAbilityDef::ActivatedMana(_)
                | DeclarativeAbilityDef::TriggeredMana(_)
                | DeclarativeAbilityDef::Activated(_)
                | DeclarativeAbilityDef::Triggered(_)
                | DeclarativeAbilityDef::Static(_)
                | DeclarativeAbilityDef::Replacement(_)
                | DeclarativeAbilityDef::AlternativeCast(_)
                | DeclarativeAbilityDef::SpecialAction(_)
                | DeclarativeAbilityDef::Keyword(_)
                | DeclarativeAbilityDef::Legacy => None,
            })
            .collect::<Vec<_>>();
        for (ability, text, targets, effect, resolver) in triggers {
            self.capture_trigger(&TriggerCapture {
                source: AbilitySourceRef {
                    object: source.card.id,
                    ability,
                },
                definition: Self::ability_presentation_definition(
                    ability,
                    Self::effective_rules_source(source).0,
                ),
                owner: source.card.owner,
                controller: source.controller,
                text,
                target_defs: targets.to_vec(),
                targets: Vec::new(),
                effect,
                resolver,
                context: event.context().into(),
                // A legacy custom trigger states its own condition inside its
                // behavior rather than declaring one here.
                condition: None,
                x: 0,
            });
        }
    }

    pub(super) fn ability_resolver(
        origin: AbilityOrigin,
        ability: &AbilityDef,
    ) -> StackAbilityResolver {
        if let Some(binding) = crate::card::ability_binding(origin, ability) {
            return StackAbilityResolver::CardOwned(binding.resolver());
        }
        if let Some(behavior) = ability.custom_behavior() {
            StackAbilityResolver::Custom(behavior)
        } else {
            let effect = match ability.declarative_effect() {
                Some(effect) => effect,
                None => EffectDef::None,
            };
            StackAbilityResolver::Declarative(ScopedEffect::primary(effect))
        }
    }

    pub(super) fn ability_origin_components(
        origin: AbilityOrigin,
        fallback: CardDefinitionId,
    ) -> (CardDefinitionId, CardPartId, AbilityId) {
        match origin {
            AbilityOrigin::Printed {
                definition,
                part,
                ability,
            } => (definition, part, ability),
            AbilityOrigin::Granted {
                source_definition,
                source_part,
                source_ability,
                ..
            } => (source_definition, source_part, source_ability),
            AbilityOrigin::IntrinsicBasicLand(_) => {
                (fallback, CardPartId::PRIMARY, AbilityId::PRIMARY)
            }
        }
    }

    pub(super) fn freeze_activated_ability(
        &self,
        permanent: &Permanent,
        origin: AbilityOrigin,
    ) -> FrozenActivatedAbility {
        let effective =
            self.find_effective_ability(permanent, |effective| effective.origin == origin);
        let fallback_definition = Self::effective_rules_source(permanent).0;
        let presentation_definition =
            Self::ability_presentation_definition(origin, fallback_definition);
        let text = effective.map(|effective| effective.ability.text);
        let definition = effective.map(|effective| Box::new(effective.ability));
        let (target_defs, resolver) = effective.map_or(
            (
                &[][..],
                StackAbilityResolver::Declarative(ScopedEffect::primary(EffectDef::None)),
            ),
            |effective| {
                let target_defs = match effective.ability.definition {
                    DeclarativeAbilityDef::Activated(definition) => definition.targets,
                    DeclarativeAbilityDef::Spell(_)
                    | DeclarativeAbilityDef::ActivatedMana(_)
                    | DeclarativeAbilityDef::TriggeredMana(_)
                    | DeclarativeAbilityDef::Triggered(_)
                    | DeclarativeAbilityDef::Static(_)
                    | DeclarativeAbilityDef::Replacement(_)
                    | DeclarativeAbilityDef::AlternativeCast(_)
                    | DeclarativeAbilityDef::SpecialAction(_)
                    | DeclarativeAbilityDef::Keyword(_)
                    | DeclarativeAbilityDef::Legacy => &[],
                };
                (
                    target_defs,
                    Self::ability_resolver(effective.origin, &effective.ability),
                )
            },
        );
        FrozenActivatedAbility {
            origin,
            definition,
            presentation_definition,
            text,
            target_defs,
            resolver,
            // Filled in by the activation, which is where X is chosen.
            x: 0,
        }
    }

    // Long because the event vocabulary is wide, not because the function
    // does several things: every arm pairs one definition with one event.
    #[allow(clippy::too_many_lines)]
    pub(super) fn trigger_event_matches_for_controller(
        &self,
        definition: TriggerEventDef,
        event: &CommittedTriggerEvent,
        source: GameObjectId,
        controller: Option<PlayerId>,
    ) -> bool {
        match (definition, event) {
            // One printed ability, several ways in. Each alternative is asked
            // the same question the single-event forms are.
            (TriggerEventDef::AnyOf(events), _) => events.iter().any(|alternative| {
                self.trigger_event_matches_for_controller(*alternative, event, source, controller)
            }),
            (
                TriggerEventDef::ZoneChanged(matcher),
                CommittedTriggerEvent::ZoneChanged {
                    object,
                    from: actual_from,
                    to: actual_to,
                    damage_sources,
                },
            ) => {
                matcher.from.is_none_or(|expected| expected == *actual_from)
                    && matcher.to.is_none_or(|expected| expected == *actual_to)
                    && matcher.previously_damaged_by.is_none_or(|reference| {
                        self.trigger_event_object_reference(reference, source, event)
                            .is_some_and(|source| damage_sources.contains(&source))
                    })
                    && self.trigger_object_matches_for_controller(
                        matcher.object,
                        object,
                        source,
                        false,
                        controller,
                    )
            }
            (
                TriggerEventDef::Tapped(matcher),
                CommittedTriggerEvent::Tapped { object, for_mana },
            ) => {
                (matcher.purpose == TapPurposeDef::Any || *for_mana)
                    && self.trigger_object_matches_for_controller(
                        matcher.object,
                        object,
                        source,
                        false,
                        controller,
                    )
            }
            (
                TriggerEventDef::BecomesBlocked(predicate),
                CommittedTriggerEvent::BecomesBlocked { object, .. },
            )
            | (
                TriggerEventDef::AttacksAndIsNotBlocked {
                    attacker: predicate,
                },
                CommittedTriggerEvent::AttacksAndIsNotBlocked { object },
            )
            | (
                TriggerEventDef::Transforms(predicate),
                CommittedTriggerEvent::Transformed { object },
            ) => self.trigger_object_matches_for_controller(
                predicate, object, source, false, controller,
            ),
            // The listener is the permanent that was pointed at, and the
            // predicate reads the spell doing the pointing.
            (
                TriggerEventDef::BecomesTargetOfSpell(predicate),
                CommittedTriggerEvent::BecameTargetOfSpell { target, object },
            ) => {
                *target == source
                    && self.trigger_object_matches_for_controller(
                        predicate, object, source, false, controller,
                    )
            }
            (
                TriggerEventDef::BlocksOrBecomesBlockedBy { object: predicate },
                CommittedTriggerEvent::BlocksOrBecomesBlocked { creature, other },
            ) => {
                creature.id == source
                    && self.trigger_object_matches_for_controller(
                        predicate, other, source, false, controller,
                    )
            }
            // The one-directional halves read the same ordered pair and tell
            // the sides apart by which of the two was attacking.
            (
                TriggerEventDef::Blocks { blocked: predicate },
                CommittedTriggerEvent::BlocksOrBecomesBlocked { creature, other },
            ) => {
                creature.id == source
                    && !creature.attacking
                    && self.trigger_object_matches_for_controller(
                        predicate, other, source, false, controller,
                    )
            }
            (
                TriggerEventDef::BecomesBlockedBy { blocker: predicate },
                CommittedTriggerEvent::BlocksOrBecomesBlocked { creature, other },
            ) => {
                creature.id == source
                    && creature.attacking
                    && self.trigger_object_matches_for_controller(
                        predicate, other, source, false, controller,
                    )
            }
            (
                TriggerEventDef::Attacks(matcher),
                CommittedTriggerEvent::Attacks {
                    object,
                    declaration_size,
                    attack_number,
                },
            ) => {
                *declaration_size >= matcher.declaration.minimum
                    && matcher
                        .declaration
                        .maximum
                        .is_none_or(|maximum| *declaration_size <= maximum)
                    && matcher
                        .attack_number
                        .is_none_or(|number| *attack_number == number)
                    && self.trigger_object_matches_for_controller(
                        matcher.attacker,
                        object,
                        source,
                        false,
                        controller,
                    )
            }
            (
                TriggerEventDef::DamageDealt(matcher),
                damage @ CommittedTriggerEvent::DamageDealt { .. },
            ) => self.damage_trigger_matches(matcher, damage, source, controller),
            // Both name only the player the event happened to.
            (TriggerEventDef::Discarded(relation), CommittedTriggerEvent::Discarded { player })
            | (
                TriggerEventDef::BecomesMonarch(relation),
                CommittedTriggerEvent::BecameMonarch { player },
            )
            | (
                TriggerEventDef::LifeGained(relation),
                CommittedTriggerEvent::LifeGained { player, .. },
            ) => {
                let controller = controller.unwrap_or(*player);
                self.player_relation_matches(*player, relation, controller, event.context())
            }
            // The listener list for a cycled card holds only that card's own
            // clauses, so there is nothing further to match on: any card
            // whose ability reached here is the card that was cycled.
            (TriggerEventDef::Cycled, CommittedTriggerEvent::Cycled { object }) => {
                object.id == source
            }
            (
                TriggerEventDef::SpellCast(predicate),
                CommittedTriggerEvent::SpellCast { object },
            ) => self
                .trigger_object_matches_for_controller(predicate, object, source, true, controller),
            (
                TriggerEventDef::StepBegins { step, player },
                CommittedTriggerEvent::StepBegins {
                    step: actual_step,
                    player: actual_player,
                },
            ) => {
                if step != *actual_step {
                    return false;
                }
                if player == PlayerRelation::ChosenPlayer {
                    return self.chosen_player_of(source) == Some(*actual_player);
                }
                if player == PlayerRelation::ControllerOfAttachedPermanent {
                    return self.attached_host_controller_of(source) == Some(*actual_player);
                }
                let controller = controller
                    .or_else(|| self.current_or_last_known_controller(source))
                    .unwrap_or(*actual_player);
                self.player_relation_matches(*actual_player, player, controller, event.context())
            }
            _ => false,
        }
    }

    fn trigger_event_player_reference(
        &self,
        reference: PlayerRefDef,
        ability_source: GameObjectId,
        controller: Option<PlayerId>,
        event: &CommittedTriggerEvent,
    ) -> Option<PlayerId> {
        match reference {
            PlayerRefDef::EffectController => controller,
            PlayerRefDef::Opponent => controller.map(PlayerId::opponent),
            PlayerRefDef::EventPlayer => event.context().event_player,
            PlayerRefDef::ControllerOf(reference) => self
                .trigger_event_object_reference(reference, ability_source, event)
                .and_then(|object| self.current_or_last_known_controller(object)),
            PlayerRefDef::OwnerOf(reference) => self
                .trigger_event_object_reference(reference, ability_source, event)
                .and_then(|object| self.current_or_last_known_owner(object)),
            PlayerRefDef::Target(_) => None,
        }
    }

    fn trigger_event_object_reference(
        &self,
        reference: ObjectRefDef,
        ability_source: GameObjectId,
        event: &CommittedTriggerEvent,
    ) -> Option<GameObjectId> {
        match reference {
            ObjectRefDef::Source => Some(ability_source),
            ObjectRefDef::AttachedToSource => {
                self.current_or_last_known_attached_host(ability_source)
            }
            ObjectRefDef::TriggeringObject => event.context().object,
            ObjectRefDef::ResolvingObject
            | ObjectRefDef::Binding(_)
            | ObjectRefDef::Target(_)
            | ObjectRefDef::SourceOfTargetedStackObject(_) => None,
        }
    }

    /// Who controls an object, whether it is still on the battlefield or has
    /// left and is only remembered.
    pub(super) fn controller_of_object(&self, object: GameObjectId) -> Option<PlayerId> {
        self.battlefield
            .iter()
            .chain(self.emblems.iter())
            .find(|permanent| permanent.card.id == object)
            .map(|permanent| permanent.controller)
            .or_else(|| match self.retired_objects.get(&object) {
                Some(RetiredObject::Permanent { permanent, .. }) => Some(permanent.controller),
                Some(RetiredObject::Stack(object)) => Some(object.controller),
                Some(RetiredObject::Card(_)) | None => None,
            })
    }
}

include!("trigger_capture/damage_matching.rs");
include!("trigger_capture/object_matching.rs");
