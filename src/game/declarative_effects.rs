use super::{
    AbilityProcedureDef, AbilitySourceRef, AddManaEffectDef, BattlefieldArrival, CardPartId,
    CharacteristicSource, CopiableAbility, CounteredSpellZone, DeclarativeAbilityDef, EffectDef,
    EffectResolutionContext, Game, GameResult, InstalledTrigger, InstalledTriggerLifetime, Mana,
    ManaPool, ManaSelectionDef, ManaSource, Permanent, ResolvedEffectPayment, SacrificeDeclined,
    SacrificeFollowup, ScopedEffect, StackAbilityResolver, StackObject, Target, TriggerCapture,
    ValueDef, WinReason, ZoneKind, ZoneMoveCause,
};
use crate::card::{EffectPaymentCostDef, InstalledTriggerLifetimeDef};

mod hand_and_library;
mod permanent_state;
mod prevention;
mod tapping;

impl Game {
    #[allow(clippy::too_many_lines)]
    pub(super) fn resolve_effect_def(
        &mut self,
        scoped: ScopedEffect,
        object: &StackObject,
        context: impl Into<EffectResolutionContext>,
    ) {
        let context = context.into();
        match scoped.effect {
            EffectDef::Sequence(effects) => {
                self.resolve_effects_in_order(
                    effects
                        .iter()
                        .map(|effect| scoped.with_effect(*effect))
                        .collect(),
                    object,
                    context,
                    None,
                );
            }
            EffectDef::Randomized {
                likelihood,
                on_success,
                on_failure,
            } => {
                let branch = if self.rng.sample_probability(likelihood.value()) {
                    on_success
                } else {
                    on_failure
                };
                self.resolve_effect_def(scoped.with_effect(*branch), object, context);
            }
            EffectDef::Choose(definition) => {
                self.queue_effect_choice(definition, object, context, scoped);
            }
            EffectDef::PayOr(definition) => {
                let payers =
                    self.effect_players(definition.payment.payer, object, &context, scoped);
                let [player] = payers.as_slice() else {
                    if let Some(otherwise) = definition.otherwise {
                        self.resolve_effect_def(scoped.with_effect(*otherwise), object, context);
                    }
                    return;
                };
                let payment = match definition.payment.cost {
                    EffectPaymentCostDef::Mana(cost) => ResolvedEffectPayment::Mana(cost),
                    EffectPaymentCostDef::GenericMana(amount) => {
                        let amount = self
                            .effect_value(amount, object, &context, scoped)
                            .max(0)
                            .try_into()
                            .unwrap_or(u16::MAX);
                        ResolvedEffectPayment::Mana(crate::ManaCost::new(amount, 0))
                    }
                    EffectPaymentCostDef::ColoredMana { color, amount } => {
                        let amount = self
                            .effect_value(amount, object, &context, scoped)
                            .max(0)
                            .try_into()
                            .unwrap_or(u16::MAX);
                        ResolvedEffectPayment::Mana(crate::ManaCost::of_color(color, amount))
                    }
                    EffectPaymentCostDef::Life(amount) => ResolvedEffectPayment::Life(amount),
                    EffectPaymentCostDef::Mill(amount) => ResolvedEffectPayment::Mill(amount),
                    EffectPaymentCostDef::Discard(amount) => ResolvedEffectPayment::Discard(amount),
                };
                self.queue_pay_or(
                    *player,
                    payment,
                    definition.visibility,
                    scoped,
                    object,
                    context,
                    definition.if_paid.map(|effect| scoped.with_effect(*effect)),
                    definition
                        .otherwise
                        .map(|effect| scoped.with_effect(*effect)),
                );
            }
            EffectDef::SplitIntoPiles(definition) => {
                self.queue_effect_pile_split(definition, object, context, scoped);
            }
            EffectDef::AddMana(AddManaEffectDef {
                mana: ManaSelectionDef::One(kind),
                amount,
                restrictions,
                spend_effects,
                damage_to_controller,
                amount_override,
                // Read only by the mana runtime, which offers the ability;
                // a triggered mana effect resolving here has a plain amount.
                variable_amount: _,
                // Resolving from the stack, the ability's own controller is
                // the only recipient any current card names.
                recipient: _,
            }) => {
                let color = kind;
                let source = object
                    .source
                    .zip(object.ability_origin())
                    .map(|(object, ability)| ManaSource { object, ability });
                let mana = Mana {
                    color,
                    source,
                    restrictions,
                    spend_effects,
                };
                let amount = amount_override
                    .filter(|override_| {
                        self.static_condition_holds(
                            override_.condition,
                            object.controller,
                            object.source.unwrap_or(object.id),
                        )
                    })
                    .map_or(amount, |override_| override_.amount);
                self.add_mana(
                    object.controller,
                    std::iter::repeat_n(mana, usize::from(amount)),
                );
                if damage_to_controller > 0 {
                    self.damage_target_from(
                        object.source.or(Some(object.id)),
                        Some(Target::Player(object.controller)),
                        damage_to_controller,
                    );
                }
            }
            EffectDef::DrainLife { recipient, amount } => {
                let amount = self
                    .effect_value(amount, object, &context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    let available = self.drainable_from(target);
                    let dealt = self.damage_target_from(Some(object.id), Some(target), amount);
                    self.gain_life(object.controller, dealt.min(available));
                }
            }
            EffectDef::DealDamage { recipient, amount } => {
                // A divided total is chosen per target when the spell is
                // cast, so each one takes its own share rather than the same
                // amount as everyone else.
                let divided = matches!(amount, ValueDef::DividedAmongTargets);
                let shared = if divided {
                    0
                } else {
                    self.effect_value(amount, object, &context, scoped)
                        .max(0)
                        .try_into()
                        .unwrap_or(u16::MAX)
                };
                let slot = recipient
                    .legal_target()
                    .map(|target| scoped.target_slot(target));
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    let amount = if divided {
                        slot.and_then(|slot| Self::divided_share(object, slot, target))
                            .unwrap_or(0)
                    } else {
                        shared
                    };
                    if amount == 0 && divided {
                        continue;
                    }
                    self.damage_target_from(
                        object.source.or(Some(object.id)),
                        Some(target),
                        amount,
                    );
                }
            }
            EffectDef::GainLife { recipient, amount } => {
                let amount = self
                    .effect_value(amount, object, &context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Player(player) = target {
                        self.gain_life(player, amount);
                    }
                }
            }
            EffectDef::DestroyAtEndOfCombat { .. }
            | EffectDef::RemoveAllCounters { .. }
            | EffectDef::SkipNextUntapSteps { .. } => {
                self.resolve_permanent_state_effect(scoped, object, &context);
            }
            EffectDef::AddPoisonCounters { recipient, amount } => {
                let amount = self
                    .effect_value(amount, object, &context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Player(player) = target {
                        self.add_poison_counters(player, amount);
                    }
                }
            }
            EffectDef::EmptyManaPool { player: recipient } => {
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Player(player) = target {
                        self.players[player.index()].mana_pool = ManaPool::default();
                        self.players[player.index()].mana.clear();
                    }
                }
            }
            EffectDef::DrawCards { .. }
            | EffectDef::ShuffleLibrary { .. }
            | EffectDef::Discard { .. }
            | EffectDef::DiscardCards { .. }
            | EffectDef::Mill { .. }
            | EffectDef::MillUntil { .. }
            | EffectDef::LookAtHand { .. }
            | EffectDef::RevealHand { .. }
            | EffectDef::LookAtTopAndSelect { .. }
            | EffectDef::SearchZone { .. }
            | EffectDef::ChooseCards { .. }
            | EffectDef::ReplaceNextDrawThisTurn { .. } => {
                self.resolve_hand_and_library_effect(scoped, object, &context);
            }
            EffectDef::LoseLife { recipient, amount } => {
                let amount = self
                    .effect_value(amount, object, &context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Player(player) = target {
                        self.lose_life(player, amount);
                    }
                }
            }
            EffectDef::Tap { .. } | EffectDef::Untap { .. } => {
                self.resolve_tap_effect(scoped, object, &context);
            }
            EffectDef::RemoveFromCombat { object: recipient } => {
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Permanent(permanent) = target {
                        self.remove_permanent_from_combat(permanent);
                    }
                }
            }
            EffectDef::Regenerate { object: recipient } => {
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Permanent(permanent) = target {
                        self.add_regeneration_shield(permanent);
                    }
                }
            }
            EffectDef::CreateToken {
                token,
                count,
                tapped,
            } => {
                for _ in 0..self.effect_value(count, object, &context, scoped).max(0) {
                    self.create_token_arriving(object.controller, token, None, tapped);
                }
            }
            EffectDef::CreateAttachedToken { token } => {
                if let Some(source) = object.source {
                    self.create_attached_token(object.controller, token, source);
                }
            }
            EffectDef::CreateTokenCopyOf { object: recipient } => {
                let copies = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .filter_map(|target| match target {
                        Target::Permanent(id) => self
                            .battlefield
                            .iter()
                            .find(|permanent| permanent.card.id == id)
                            // A token that is itself a copy of something else
                            // copies what it became, not what it was made as.
                            .map(|permanent| {
                                permanent
                                    .copied_from
                                    .map_or(permanent.card.definition, |(definition, _)| definition)
                            }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                for definition in copies {
                    self.create_token(object.controller, definition);
                }
            }
            EffectDef::PreventDamage { .. } => {
                self.resolve_prevention_effect(scoped, object, &context);
            }
            EffectDef::Destroy {
                object: recipient,
                can_regenerate,
            } => {
                let permanents = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .filter_map(|target| match target {
                        Target::Permanent(permanent) => Some(permanent),
                        Target::Player(_) | Target::Card(_) | Target::Spell(_) => None,
                    })
                    .collect::<Vec<_>>();
                self.destroy_permanents(&permanents, can_regenerate);
            }
            EffectDef::Sacrifice { object: recipient } => {
                let permanents = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .filter_map(|target| match target {
                        Target::Permanent(permanent) => Some(permanent),
                        Target::Player(_) | Target::Card(_) | Target::Spell(_) => None,
                    })
                    .filter(|permanent| {
                        self.permanent_controller(*permanent)
                            .is_none_or(|controller| {
                                self.can_be_forced_to_sacrifice(controller, object.controller)
                            })
                    })
                    .collect::<Vec<_>>();
                self.move_permanents_to_graveyard(&permanents);
            }
            EffectDef::SacrificeOfChoice {
                amount: sacrificed_amount,
                player: recipient,
                object: predicate,
                then,
                otherwise,
                optional,
            } => {
                let source = object.source.unwrap_or(object.id);
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    let Target::Player(player) = target else {
                        continue;
                    };
                    // A prohibition on being forced to sacrifice does not
                    // reach an offer the player is free to decline.
                    if !optional && !self.can_be_forced_to_sacrifice(player, object.controller) {
                        continue;
                    }
                    let followup = then.map(|effect| SacrificeFollowup {
                        amount: sacrificed_amount,
                        object: Box::new(object.clone()),
                        context: context.clone(),
                        effect: scoped.with_effect(*effect),
                    });
                    let declined = otherwise.map(|effect| SacrificeDeclined {
                        object: Box::new(object.clone()),
                        context: context.clone(),
                        effect: scoped.with_effect(*effect),
                    });
                    self.queue_chosen_sacrifice(
                        player, predicate, source, followup, declined, optional,
                    );
                }
            }
            EffectDef::IfFormat {
                format,
                then,
                otherwise,
            } => {
                let effect = if self.format == format {
                    then
                } else {
                    otherwise
                };
                self.resolve_effect_def(scoped.with_effect(*effect), object, context);
            }
            EffectDef::CreateEmblem { emblem } => {
                let controller = object.controller;
                let card =
                    self.unbacked_object(emblem, controller, CharacteristicSource::Ability(emblem));
                let mut emblem = Permanent::entering(
                    card,
                    CardPartId::PRIMARY,
                    controller,
                    self.turns_started[controller.index()],
                );
                emblem.timestamp = self.allocate_continuous_effect_timestamp();
                emblem.emblem_source = object.ability_origin();
                self.emblems.push(emblem);
            }
            EffectDef::LoseTheGame { player: recipient } => {
                let mut losers = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .filter_map(|target| match target {
                        Target::Player(player) => Some(player),
                        Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                    })
                    .collect::<Vec<_>>();
                losers.sort_unstable();
                losers.dedup();
                match losers.as_slice() {
                    [loser] => self.finish(GameResult::Winner {
                        winner: loser.opponent(),
                        reason: WinReason::OpponentLostToAnEffect,
                    }),
                    [_, _] => self.finish(GameResult::Draw),
                    [] => {}
                    _ => unreachable!("a two-player game has at most two losers"),
                }
            }
            EffectDef::Transform { object: recipient } => {
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Permanent(id) = target {
                        self.transform_permanent(id);
                    }
                }
            }
            EffectDef::ScheduleTurnPhases(phases) => {
                self.schedule_turn_phases(phases);
            }
            EffectDef::TakeExtraTurn { player: recipient } => {
                let players = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .filter_map(|target| match target {
                        Target::Player(player) => Some(player),
                        Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                    });
                self.schedule_extra_turns(players);
            }
            EffectDef::GrantFlashToNextSorcery => {
                let grants = &mut self.sorcery_flash_grants[object.controller.index()];
                *grants = grants.saturating_add(1);
            }
            EffectDef::May {
                player: recipient,
                effect,
            } => {
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Player(player) = target {
                        self.queue_optional_effect(
                            player,
                            object,
                            context.clone(),
                            scoped.with_effect(*effect),
                        );
                    }
                }
            }
            EffectDef::ExileLinkedToSource { object: recipient } => {
                let source = object.source.unwrap_or(object.id);
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    let exiled = match target {
                        Target::Permanent(id) => self.exile_permanent_returning_card(id),
                        Target::Card(id) => self.exile_card_returning_card(id),
                        Target::Player(_) | Target::Spell(_) => None,
                    };
                    if let Some(exiled) = exiled {
                        self.linked_exiles.push((source, exiled));
                    }
                }
            }
            EffectDef::ReturnLinkedExiles {
                zone,
                grant,
                controller,
            } => {
                let source = object.source.unwrap_or(object.id);
                let returning = self
                    .linked_exiles
                    .iter()
                    .filter(|(exiled_by, _)| *exiled_by == source)
                    .map(|(_, card)| *card)
                    .collect::<Vec<_>>();
                self.linked_exiles
                    .retain(|(exiled_by, _)| *exiled_by != source);
                let arriving_controller = controller.map(|relation| {
                    if self.player_relation_matches(
                        object.controller,
                        relation,
                        object.controller,
                        context.trigger,
                    ) {
                        object.controller
                    } else {
                        object.controller.opponent()
                    }
                });
                for card in returning {
                    self.return_exiled_card(card, zone, grant, arriving_controller);
                }
            }
            EffectDef::Detain { object: recipient } => {
                let controller = object.controller;
                let created = self.turns_started[controller.index()];
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Permanent(id) = target
                        && let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|permanent| permanent.card.id == id)
                    {
                        permanent.detained_until_turn_of = Some((controller, created));
                    }
                }
            }
            EffectDef::GainControl {
                object: recipient,
                duration,
            } => {
                self.take_control_of(recipient, object, &context, scoped, duration);
            }
            EffectDef::IfCondition { condition, then } => {
                if self.trigger_condition_holds(
                    condition,
                    object.source.unwrap_or(object.id),
                    object.controller,
                    context.trigger,
                    object.ability.as_ref().map(|ability| ability.origin),
                    Some((object, scoped, &context)),
                ) {
                    self.resolve_effect_def(scoped.with_effect(*then), object, context);
                }
            }
            EffectDef::InstallTrigger(installed) => {
                let DeclarativeAbilityDef::Triggered(definition) = installed.ability.definition
                else {
                    return;
                };
                // Installed triggers use the ordinary pending-trigger and
                // stack paths. Declaring fresh targets would require a second
                // target namespace; until that exists they may only retain
                // the installing object's already-chosen target slots.
                if definition.procedure != AbilityProcedureDef::Shared
                    || !definition.targets.is_empty()
                {
                    return;
                }
                let Some(effect) = installed.ability.declarative_effect() else {
                    return;
                };
                let Some(frozen) = object.ability.as_ref() else {
                    return;
                };
                let lifetime = match installed.lifetime {
                    InstalledTriggerLifetimeDef::Once => InstalledTriggerLifetime::Once,
                    InstalledTriggerLifetimeDef::UntilNextTurn(player) => {
                        let Some(player) =
                            self.effect_player_reference(player, object, &context, scoped)
                        else {
                            return;
                        };
                        InstalledTriggerLifetime::UntilTurn {
                            player,
                            turn: self.turns_started[player.index()].saturating_add(1),
                        }
                    }
                };
                let id = self.next_installed_trigger_id;
                self.next_installed_trigger_id = self.next_installed_trigger_id.saturating_add(1);
                self.installed_triggers.push(InstalledTrigger {
                    id,
                    event: definition.event,
                    capture: TriggerCapture {
                        source: AbilitySourceRef {
                            object: object.source.unwrap_or(object.id),
                            ability: frozen.origin,
                        },
                        definition: frozen.presentation_definition,
                        owner: object.card.owner,
                        controller: object.controller,
                        text: installed.ability.text,
                        // The selections belong to the installing ability's
                        // lexical target namespace. They remain readable by
                        // the nested effect, but the installed ability does
                        // not target them again when it triggers.
                        target_defs: Vec::new(),
                        targets: frozen.targets.clone(),
                        effect,
                        resolver: StackAbilityResolver::Declarative(scoped.with_effect(effect)),
                        context,
                        condition: definition.condition,
                        x: frozen.x,
                    },
                    lifetime,
                });
            }
            EffectDef::AddManaEqualTo { color, amount } => {
                let amount = self
                    .effect_value(amount, object, &context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                self.add_unrestricted_mana(object.controller, color, amount);
            }
            EffectDef::Counter {
                object: recipient,
                zone,
            } => {
                let zone = if zone == ZoneKind::Exile {
                    CounteredSpellZone::Exile
                } else {
                    CounteredSpellZone::Graveyard
                };
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Spell(spell) = target {
                        self.counter_spell_into(spell, zone);
                    }
                }
            }
            EffectDef::AddCounters {
                object: recipient,
                kind,
                amount,
            } => {
                let amount = self
                    .effect_value(amount, object, &context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Permanent(permanent) = target
                        && let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|candidate| candidate.card.id == permanent)
                    {
                        permanent.add_counters(kind, amount);
                    }
                }
            }
            EffectDef::RemoveCounters {
                object: recipient,
                kind,
                amount,
            } => {
                let amount = self
                    .effect_value(amount, object, &context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Permanent(permanent) = target
                        && let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|candidate| candidate.card.id == permanent)
                    {
                        permanent.remove_counters(kind, amount);
                    }
                }
            }
            EffectDef::ChooseColor {
                object: recipient,
                operation,
                duration,
            } => {
                // Resolved before the question is asked: targets are already
                // chosen, and a group is whatever it is at this moment.
                let targets = self.effect_recipients(recipient, object, &context, scoped);
                if !targets.is_empty() {
                    self.queue_color_choice(
                        object.controller,
                        Box::new(object.clone()),
                        context.clone(),
                        scoped,
                        targets,
                        operation,
                        duration,
                    );
                }
            }
            EffectDef::ChangeTextBasicLandType { object: recipient } => {
                if let Some(target) = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .next()
                {
                    self.queue_basic_land_type_text_change(object.controller, target);
                }
            }
            EffectDef::BecomeCopyOf {
                object: recipient,
                retain_source_ability,
            } => {
                let Some(Target::Permanent(target)) = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .next()
                else {
                    return;
                };
                let Some(mut copy) = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == target)
                    .map(Self::copiable_characteristics)
                else {
                    return;
                };
                if retain_source_ability
                    && let Some(payload) = &object.ability
                    && let Some(definition) = payload.definition.as_deref()
                {
                    copy.added_abilities.push(CopiableAbility {
                        origin: payload.origin,
                        definition: *definition,
                    });
                }
                if let Some(source) = object.source
                    && let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == source)
                {
                    permanent.copy_effect = Some(copy);
                }
            }
            EffectDef::Apply {
                recipient,
                effect,
                duration,
            } => self.resolve_applied_effect(recipient, effect, duration, object, &context, scoped),
            EffectDef::MoveToZone {
                object: recipient,
                zone,
                controller,
                placement,
            } => {
                let arriving_controller = controller.map(|relation| {
                    if self.player_relation_matches(
                        object.controller,
                        relation,
                        object.controller,
                        context.trigger,
                    ) {
                        object.controller
                    } else {
                        object.controller.opponent()
                    }
                });
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    self.move_target_to_zone(
                        target,
                        zone,
                        ZoneMoveCause::Effect {
                            controller: object.controller,
                        },
                        arriving_controller.map(BattlefieldArrival::under),
                        placement,
                    );
                }
            }
            // An Aura attaches as its spell becomes a permanent, so its own
            // clause has nothing left to do. Equip resolves this instead.
            EffectDef::Attach { object: recipient } => {
                let Some(source) = object.source else {
                    return;
                };
                let host = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .find_map(|target| match target {
                        Target::Permanent(id) => Some(id),
                        _ => None,
                    });
                if let Some(host) = host {
                    self.try_attach(source, host);
                }
            }
            EffectDef::PairWithSource { object: recipient } => {
                let Some(source) = object.source else {
                    return;
                };
                let partner = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .find_map(|target| match target {
                        Target::Permanent(id) => Some(id),
                        _ => None,
                    });
                if let Some(partner) = partner {
                    self.pair_creatures(source, partner);
                }
            }
            EffectDef::Reconfigure { object: recipient } => {
                let Some(source) = object.source else {
                    return;
                };
                let host = self
                    .effect_recipients(recipient, object, &context, scoped)
                    .into_iter()
                    .find_map(|target| match target {
                        Target::Permanent(id) => Some(id),
                        _ => None,
                    });
                if let Some(host) = host {
                    self.try_attach(source, host);
                } else {
                    self.unattach(source);
                }
            }
            EffectDef::None
            | EffectDef::AddMana(AddManaEffectDef {
                mana: ManaSelectionDef::Choice(_),
                ..
            })
            | EffectDef::CannotBeForcedToSacrifice
            | EffectDef::ReduceGenericCostBy(_)
            | EffectDef::IncreaseMatchingAbilityCostBy { .. }
            | EffectDef::IncreaseMatchingSpellCostBy { .. }
            | EffectDef::ReduceMatchingSpellCostBy { .. }
            | EffectDef::LandwalkCanBeBlocked(_)
            | EffectDef::CannotAttackUnless(_)
            | EffectDef::CannotAttackIf(_)
            | EffectDef::StaticApply { .. }
            | EffectDef::Special(_) => {
                // Choice-bearing mana and the remaining declarative effect
                // families are execution seams until a supported card needs
                // their concrete rules procedure.
            }
        }
    }
}
