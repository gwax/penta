use super::{
    BalanceAction, BasicLandType, BasicLandTypeChange, BattlefieldArrival,
    BattlefieldExitCompletion, CardRuntime, CounterKind, DecisionContinuation, DecisionOption,
    FORK_COPY_COLOR, Game, GameEvent, ManaCost, PendingProcedure, PileChoice, PileSplit, PlayerId,
    ReplaceableEvent, Target, TargetSelection, TargetSlotId, ZoneKind, ZoneMoveCause,
    ZonePlacement, remove_card,
};
use crate::card::{BattlefieldEntryChoiceDestinationDef, ReplacementEffectDef};

impl Game {
    #[allow(clippy::too_many_lines)]
    pub(super) fn choose_decision(&mut self, player: PlayerId, decision: u32, options: &[u32]) {
        let pending = self.pending_decisions.remove(0);
        debug_assert_eq!(pending.observation.id, decision);
        // Kept beside the answer because a payment decision that offered one
        // option per candidate has to look up which card the chosen option
        // named.
        let pending_options = pending.observation.options.clone();
        match pending.continuation {
            DecisionContinuation::BeginTurn {
                player,
                kind,
                applied,
                replacements,
                deferred,
            } => {
                if let Some(option) = options.first().copied() {
                    self.choose_begin_turn(player, kind, applied, &replacements, deferred, option);
                }
            }
            DecisionContinuation::DiscardForEffect {
                player,
                amount,
                mut remaining,
                mut chosen,
                cause,
            } => {
                let selected = pending
                    .observation
                    .options
                    .iter()
                    .filter(|option| options.contains(&option.id))
                    .filter_map(|option| option.card.map(|(card, _)| card))
                    .collect::<Vec<_>>();
                chosen.push((player, selected));
                if remaining.is_empty() {
                    self.complete_effect_discards(chosen, cause);
                } else {
                    let next = remaining.remove(0);
                    self.queue_next_effect_discard(next, amount, remaining, chosen, cause);
                }
            }
            DecisionContinuation::ChooseColor {
                object,
                context,
                scoped,
                targets,
                operation,
                duration,
            } => {
                let Some(index) = options
                    .first()
                    .and_then(|option| usize::try_from(*option).ok())
                    .filter(|index| *index < Self::CHOOSABLE_COLORS.len())
                else {
                    return;
                };
                self.apply_chosen_color(
                    &object, &context, scoped, &targets, operation, duration, index,
                );
            }
            DecisionContinuation::BasicLandTypeTextChange { target } => {
                let Some(option) = options.first().copied() else {
                    return;
                };
                let width = u32::try_from(BasicLandType::ALL.len())
                    .expect("the basic-land-type count fits u32");
                let Some(from) = usize::try_from(option / width)
                    .ok()
                    .and_then(BasicLandType::from_index)
                else {
                    return;
                };
                let Some(to) = usize::try_from(option % width)
                    .ok()
                    .and_then(BasicLandType::from_index)
                else {
                    return;
                };
                if from == to {
                    return;
                }
                let change = BasicLandTypeChange { from, to };
                match target {
                    Target::Permanent(id) => {
                        if let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|permanent| permanent.card.id == id)
                        {
                            permanent.text_changes.push(change);
                        }
                    }
                    Target::Spell(id) => {
                        if let Some(index) = self.stack.iter().position(|spell| spell.id == id) {
                            self.stack[index].text_changes.push(change);
                        }
                    }
                    Target::Player(_) | Target::Card(_) => {}
                }
            }
            DecisionContinuation::AugurOfBolas { player, revealed } => {
                let kept = pending
                    .observation
                    .options
                    .iter()
                    .find(|option| options.contains(&option.id))
                    .and_then(|option| option.card)
                    .map(|(card, _)| card);
                let (to_hand, to_bottom): (Vec<_>, Vec<_>) =
                    revealed.into_iter().partition(|card| Some(card.id) == kept);
                for card in to_hand {
                    let (card, _zone_change) = self.zone_change_card(card);
                    self.players[player.index()].hand.push(card);
                }
                // "In any order" -- printed order is as good as any, and the
                // rest of the library is already unknown to everyone.
                for card in to_bottom {
                    let (card, _zone_change) = self.zone_change_card(card);
                    self.players[player.index()].library.push(card);
                }
            }
            DecisionContinuation::TopCardSelection {
                player,
                revealed,
                selection,
                object,
                context,
                effect,
            } => {
                // Walked in the order the cards were named rather than the
                // order they were offered, because an arrangement reads that
                // sequence back below. Membership is all the ordinary path
                // asks of it, so nothing else notices.
                let selected = options
                    .iter()
                    .filter_map(|chosen| {
                        pending
                            .observation
                            .options
                            .iter()
                            .find(|option| option.id == *chosen)
                    })
                    .filter_map(|option| option.card.map(|(card, _)| card))
                    .collect::<Vec<_>>();
                let (mut chosen, rest): (Vec<_>, Vec<_>) = revealed
                    .into_iter()
                    .partition(|card| selected.contains(&card.id));
                // "Put them back in any order": the arrangement is the order
                // the cards were named, so it has to survive the partition,
                // which otherwise reports them in library order.
                if selection.selected_order_follows_choice {
                    chosen.sort_by_key(|card| {
                        selected
                            .iter()
                            .position(|id| *id == card.id)
                            .unwrap_or(usize::MAX)
                    });
                }
                self.finish_top_card_selection(player, chosen, rest, selection);
                if let Some(then) = selection.then {
                    self.resolve_nested_effect_before_later(
                        effect.with_effect(*then),
                        &object,
                        context,
                    );
                }
            }
            DecisionContinuation::BattlefieldEntryReplacement { candidates } => {
                let selected = options
                    .first()
                    .and_then(|option| usize::try_from(*option).ok())
                    .and_then(|index| candidates.get(index))
                    .copied();
                if let (Some(pending), Some(selected)) = (self.pending_events.pop_front(), selected)
                    && let Some(pending) = self.prepare_entry_replacement(pending, selected)
                {
                    self.pending_events.push_front(pending);
                    self.continue_pending_events();
                }
            }
            DecisionContinuation::BattlefieldEntryOptional { context, effect } => {
                self.resume_optional_entry_replacement(context, effect, options);
            }
            DecisionContinuation::BattlefieldExitReplacement {
                mut batch,
                candidates,
            } => {
                let selected = options
                    .first()
                    .and_then(|option| usize::try_from(*option).ok())
                    .and_then(|index| candidates.get(index))
                    .copied();
                if let Some(selected) = selected {
                    self.apply_battlefield_exit_replacement(&mut batch, selected);
                    self.continue_battlefield_exit_replacements(batch);
                }
            }
            DecisionContinuation::BattlefieldEntryPayment {
                context,
                player,
                payment,
                definition,
            } => {
                if let Some(mut pending) = self.pending_events.pop_front() {
                    let paid =
                        self.settle_payment_decision(player, payment, options, &pending_options);
                    let ReplacementEffectDef::PayOr {
                        if_paid,
                        if_declined,
                        ..
                    } = definition
                    else {
                        return;
                    };
                    Self::push_replacement_effects(
                        &mut pending,
                        context,
                        if paid { if_paid } else { if_declined },
                    );
                    self.pending_events.push_front(pending);
                    self.continue_pending_events();
                }
            }
            DecisionContinuation::BattlefieldEntryCopy {
                choices,
                added_types,
            } => {
                let copied = options
                    .first()
                    .and_then(|option| usize::try_from(*option).ok())
                    .filter(|option| *option > 0)
                    .and_then(|option| choices.get(option - 1).copied())
                    .and_then(|id| {
                        self.battlefield
                            .iter()
                            .find(|permanent| permanent.card.id == id)
                    })
                    .map(|permanent| {
                        let mut copy = Self::copiable_characteristics(permanent);
                        copy.added_types = copy.added_types.union(added_types);
                        copy
                    });
                if let Some(mut pending) = self.pending_events.pop_front() {
                    if let Some(copy) = copied {
                        let ReplaceableEvent::BattlefieldEntry(entry) = &mut pending.event;
                        entry.permanent.copied_from = Some(copy.base);
                        entry.permanent.copy_effect = Some(copy);
                    }
                    self.pending_events.push_front(pending);
                    self.continue_pending_events();
                }
            }
            DecisionContinuation::BattlefieldEntryScalarChoice {
                choice, choices, ..
            } => {
                let Some(selected) = options
                    .first()
                    .and_then(|option| usize::try_from(*option).ok())
                    .and_then(|index| choices.get(index))
                    .cloned()
                else {
                    return;
                };
                if let Some(mut pending) = self.pending_events.pop_front() {
                    let ReplaceableEvent::BattlefieldEntry(entry) = &mut pending.event;
                    match choice.destination {
                        BattlefieldEntryChoiceDestinationDef::CardName => {
                            entry.permanent.chosen_card_name = Some(selected);
                        }
                        BattlefieldEntryChoiceDestinationDef::CreatureType => {
                            entry.permanent.chosen_creature_type = Some(selected);
                        }
                    }
                    self.pending_events.push_front(pending);
                    self.continue_pending_events();
                }
            }
            DecisionContinuation::PayOr {
                player,
                payment,
                definition: _,
                object,
                context,
                if_paid,
                otherwise,
            } => {
                let paid = self.settle_payment_decision(player, payment, options, &pending_options);
                let branch = if paid { if_paid } else { otherwise };
                if let Some(effect) = branch {
                    self.resolve_nested_effect_before_later(effect, &object, context);
                }
            }
            DecisionContinuation::ChainLightning {
                player,
                spell,
                targets,
            } => {
                if let Some(option) = options.first().copied()
                    && option > 0
                    && let Some(target) = targets.get(usize::try_from(option - 1).unwrap_or(0))
                {
                    let cost = ManaCost::new(0, 2);
                    self.activate_mana_for_cost(player, cost, 0);
                    let _ = self.pay_player_cost(player, cost, 0);
                    let replacements = spell
                        .signature
                        .as_ref()
                        .and_then(|signature| signature.targets().first())
                        .map(|selection| vec![TargetSelection::single(selection.slot(), *target)])
                        .unwrap_or_default();
                    self.push_copy(spell, player, replacements);
                }
            }
            DecisionContinuation::Fork {
                player,
                spell,
                target_lists,
            } => {
                if let Some(option) = options.first().copied()
                    && let Some(targets) = target_lists.get(usize::try_from(option).unwrap_or(0))
                {
                    self.push_copy_with_colors(
                        spell,
                        player,
                        targets.clone(),
                        Some(FORK_COPY_COLOR),
                    );
                }
            }
            DecisionContinuation::GrislySalvage { player, revealed } => {
                let kept = pending
                    .observation
                    .options
                    .iter()
                    .find(|option| options.contains(&option.id))
                    .and_then(|option| option.card)
                    .map(|(card, _)| card);
                let (to_hand, to_graveyard): (Vec<_>, Vec<_>) =
                    revealed.into_iter().partition(|card| Some(card.id) == kept);
                for card in to_hand {
                    let (card, _zone_change) = self.zone_change_card(card);
                    self.players[player.index()].hand.push(card);
                }
                self.bury_cards(player, to_graveyard);
            }
            DecisionContinuation::OptionalEffect {
                object,
                context,
                effect,
            } => {
                if options.contains(&1) {
                    self.resolve_effect_def(effect, &object, context);
                }
            }
            DecisionContinuation::ChooseForEffect {
                definition: _,
                binding,
                object,
                mut context,
                candidates,
                effect,
            } => {
                let selected = pending
                    .observation
                    .options
                    .iter()
                    .filter(|option| options.contains(&option.id))
                    .filter_map(|option| usize::try_from(option.id).ok())
                    .filter_map(|index| candidates.get(index))
                    .copied()
                    .collect();
                Self::bind_effect_choice(&mut context, binding, selected);
                self.resolve_nested_effect_before_later(effect, &object, context);
            }
            DecisionContinuation::SplitForEffect {
                definition,
                chooser,
                items,
                object,
                context,
            } => {
                let (first, second) = items.into_iter().enumerate().fold(
                    (Vec::new(), Vec::new()),
                    |(mut first, mut second), (index, item)| {
                        if u32::try_from(index).is_ok_and(|id| options.contains(&id)) {
                            first.push(item);
                        } else {
                            second.push(item);
                        }
                        (first, second)
                    },
                );
                self.queue_effect_pile_choice(chooser, first, second, object, context, definition);
            }
            DecisionContinuation::ChoosePileForEffect {
                definition: _,
                first,
                second,
                chosen,
                unchosen,
                object,
                mut context,
                effect,
            } => {
                let choose_first = options.first().copied() == Some(0);
                let (chosen_objects, unchosen_objects) = if choose_first {
                    (first, second)
                } else {
                    (second, first)
                };
                context.bind_object_group(chosen, chosen_objects);
                context.bind_object_group(unchosen, unchosen_objects);
                self.resolve_nested_effect_before_later(effect, &object, context);
            }
            DecisionContinuation::MiracleReveal { card } => {
                if options.contains(&1) {
                    self.miracle_window = Some(card);
                }
            }
            DecisionContinuation::SeparateIntoPiles {
                resolving_controller,
                subject,
                items,
                on_complete,
            } => {
                let first = items
                    .iter()
                    .filter(|option| options.contains(&option.id))
                    .cloned()
                    .collect();
                let second = items
                    .iter()
                    .filter(|option| !options.contains(&option.id))
                    .cloned()
                    .collect();
                let mut runtime = CardRuntime { game: self };
                on_complete.run(
                    &mut runtime,
                    PileSplit {
                        resolving_controller,
                        subject,
                        first,
                        second,
                    },
                );
            }
            DecisionContinuation::ChoosePile { piles, on_complete } => {
                let chosen = if options.contains(&0) {
                    &piles.first
                } else {
                    &piles.second
                };
                let unchosen = if options.contains(&0) {
                    &piles.second
                } else {
                    &piles.first
                };
                let object_ids = |pile: &[DecisionOption]| {
                    pile.iter()
                        .flat_map(|option| {
                            if option.members.is_empty() {
                                option.card.into_iter().collect::<Vec<_>>()
                            } else {
                                option.members.clone()
                            }
                        })
                        .map(|(id, _)| id)
                        .collect::<Vec<_>>()
                };
                let mut runtime = CardRuntime { game: self };
                on_complete.run(
                    &mut runtime,
                    PileChoice {
                        resolving_controller: piles.resolving_controller,
                        subject: piles.subject,
                        chosen: object_ids(chosen),
                        unchosen: object_ids(unchosen),
                    },
                );
            }
            DecisionContinuation::SacrificeOfChoice {
                followup,
                declined,
                optional,
            } => {
                let sacrificed = pending
                    .observation
                    .options
                    .iter()
                    .filter(|option| options.contains(&option.id))
                    .filter_map(|option| option.card.map(|(card, _)| card))
                    .collect::<Vec<_>>();
                let chosen = sacrificed.first().copied();
                // "If a player does" -- declining an optional sacrifice earns
                // nothing, while a compulsory one pays out even for nothing.
                if let Some(followup) = followup
                    && (chosen.is_some() || !optional)
                {
                    self.move_permanents_to_graveyard_then(
                        &sacrificed,
                        Some(BattlefieldExitCompletion::SacrificeFollowup {
                            followup,
                            sacrificed: chosen,
                        }),
                    );
                } else {
                    self.move_permanents_to_graveyard(&sacrificed);
                }
                // The declined branch is the other half of one printed
                // clause, so it runs only when nothing was actually given up.
                if chosen.is_none() {
                    self.resolve_sacrifice_declined(declined);
                }
            }
            DecisionContinuation::SearchZone {
                controller,
                source,
                destination,
                placement,
                reveal,
                shuffle,
                enters_tapped,
            } => {
                let selected = options
                    .iter()
                    .filter_map(|selected| {
                        pending
                            .observation
                            .options
                            .iter()
                            .find(|option| option.id == *selected)
                            .and_then(|option| option.card)
                    })
                    .collect::<Vec<_>>();
                if reveal {
                    self.events
                        .extend(selected.iter().map(|(card, definition)| {
                            GameEvent::CardRevealed {
                                player,
                                card: *card,
                                definition: *definition,
                            }
                        }));
                }

                // Cards put into a library with an explicit placement belong
                // there only after the shuffle. A card excluded from its own
                // library's shuffle never changed zones, so preserve its
                // object identity; cards arriving from another zone do make
                // the ordinary zone change before being held aside.
                if destination == ZoneKind::Library {
                    let mut held = Vec::new();
                    if source == ZoneKind::Library {
                        held.extend(selected.iter().filter_map(|(card, _)| {
                            remove_card(&mut self.players[player.index()].library, *card)
                                .map(|card| (card.owner, card))
                        }));
                    } else {
                        for (card, _) in selected {
                            let Some((moved, actual_destination)) = self
                                .move_card_from_nonbattlefield_zone(
                                    card,
                                    source,
                                    destination,
                                    ZoneMoveCause::Effect { controller },
                                    None,
                                )
                            else {
                                continue;
                            };
                            if actual_destination == ZoneKind::Library
                                && let Some(card) = remove_card(
                                    &mut self.players[moved.owner.index()].library,
                                    moved.id,
                                )
                            {
                                held.push((moved.owner, card));
                            }
                        }
                    }
                    if shuffle {
                        self.rng.shuffle(&mut self.players[player.index()].library);
                    }
                    for (owner, card) in held {
                        match placement {
                            ZonePlacement::Top => {
                                self.players[owner.index()].library.push(card);
                            }
                            ZonePlacement::Bottom => {
                                self.players[owner.index()].library.insert(0, card);
                            }
                        }
                    }
                    return;
                }

                if source != destination {
                    for (card, _) in selected {
                        let _ = self.move_card_from_nonbattlefield_zone(
                            card,
                            source,
                            destination,
                            ZoneMoveCause::Effect { controller },
                            (destination == ZoneKind::Battlefield).then(|| {
                                if enters_tapped {
                                    BattlefieldArrival::tapped_under(player)
                                } else {
                                    BattlefieldArrival::under(player)
                                }
                            }),
                        );
                    }
                }
                if shuffle {
                    // Putting a searched-for permanent onto the battlefield can
                    // suspend for an as-enters choice. Finish that prospective
                    // entry before carrying out the search's subsequent
                    // shuffle, but still precede any enclosing effect tail.
                    self.pending_procedures
                        .push_front(PendingProcedure::ShuffleLibrary { player });
                }
            }
            DecisionContinuation::ChooseCards {
                controller,
                destination,
                placement,
                reveal,
            } => {
                let selected = options
                    .iter()
                    .filter_map(|selected| {
                        pending
                            .observation
                            .options
                            .iter()
                            .find(|option| option.id == *selected)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if reveal {
                    self.events.extend(selected.iter().filter_map(|option| {
                        option
                            .card
                            .map(|(card, definition)| GameEvent::CardRevealed {
                                player,
                                card,
                                definition,
                            })
                    }));
                }
                for option in selected {
                    let Some((id, _)) = option.card else {
                        continue;
                    };
                    if option.zone == crate::game::DecisionZone::OutsideGame {
                        // Outside-game imports currently have one shared
                        // supported destination: the hand. Reject any broader
                        // authored shape without consuming the physical card.
                        if destination != ZoneKind::Hand {
                            continue;
                        }
                        let Some(card) =
                            remove_card(&mut self.players[player.index()].outside_game, id)
                        else {
                            continue;
                        };
                        let owner = card.owner;
                        let (card, _zone_change) = self.zone_change_card(card);
                        self.players[owner.index()].hand.push(card);
                        continue;
                    }
                    let source = match option.zone {
                        crate::game::DecisionZone::Library => ZoneKind::Library,
                        crate::game::DecisionZone::Hand => ZoneKind::Hand,
                        crate::game::DecisionZone::Graveyard => ZoneKind::Graveyard,
                        crate::game::DecisionZone::Exile => ZoneKind::Exile,
                        crate::game::DecisionZone::Battlefield
                        | crate::game::DecisionZone::Stack
                        | crate::game::DecisionZone::Command
                        | crate::game::DecisionZone::OutsideGame
                        | crate::game::DecisionZone::DrawnThisStep
                        | crate::game::DecisionZone::None => continue,
                    };
                    let Some((moved, actual_destination)) = self
                        .move_card_from_nonbattlefield_zone(
                            id,
                            source,
                            destination,
                            ZoneMoveCause::Effect { controller },
                            (destination == ZoneKind::Battlefield)
                                .then(|| BattlefieldArrival::under(player)),
                        )
                    else {
                        continue;
                    };
                    if actual_destination == ZoneKind::Library
                        && placement == ZonePlacement::Bottom
                        && let Some(card) =
                            remove_card(&mut self.players[moved.owner.index()].library, moved.id)
                    {
                        self.players[moved.owner.index()].library.insert(0, card);
                    }
                }
            }
            DecisionContinuation::DrawReplacement {
                player,
                mut replacements,
            } => {
                let selected = options
                    .first()
                    .and_then(|option| usize::try_from(*option).ok())
                    .filter(|index| *index < replacements.len());
                let Some(selected) = selected else {
                    self.draw_replacements[player.index()].extend(replacements);
                    return;
                };
                let replacement = replacements.remove(selected);
                self.draw_replacements[player.index()].extend(replacements);
                // The interrupted draw instruction and any enclosing effect
                // tail are already queued behind this choice. A chosen
                // replacement is part of the current draw, so let every
                // procedure it starts finish before restoring that later
                // work. In particular, a replacement that draws must not be
                // deferred until after the original instruction resumes.
                let mut later_procedures = std::mem::take(&mut self.pending_procedures);
                self.resolve_effect_def(
                    replacement.effect,
                    &replacement.object,
                    replacement.context,
                );
                self.pending_procedures.append(&mut later_procedures);
            }
            DecisionContinuation::RecallDiscard { player } => {
                let discarded = pending
                    .observation
                    .options
                    .iter()
                    .filter(|option| options.contains(&option.id))
                    .filter_map(|option| option.card.map(|(card, _)| card))
                    .collect::<Vec<_>>();
                let count = discarded.len();
                self.discard_cards_with_cause(
                    player,
                    &discarded,
                    ZoneMoveCause::Effect { controller: player },
                );
                // "for each card discarded this way" -- and those cards are in
                // the graveyard now, so Recall can hand any of them straight
                // back.
                self.queue_recall_return(player, count);
            }
            DecisionContinuation::RecallReturn { player } => {
                for option in &pending.observation.options {
                    if options.contains(&option.id)
                        && let Some((card, _)) = option.card
                        && let Some(card) =
                            remove_card(&mut self.players[player.index()].graveyard, card)
                    {
                        let (card, _zone_change) = self.zone_change_card(card);
                        self.players[player.index()].hand.push(card);
                    }
                }
            }
            DecisionContinuation::Balance {
                controller,
                phase,
                task,
                mut remaining,
            } => {
                let mut discards = Vec::new();
                let mut sacrifices = Vec::new();
                for option in &pending.observation.options {
                    if !options.contains(&option.id) {
                        continue;
                    }
                    let Some((card, _)) = option.card else {
                        continue;
                    };
                    match task.action {
                        BalanceAction::Sacrifice => sacrifices.push(card),
                        BalanceAction::Discard => discards.push(card),
                    }
                }
                if sacrifices.is_empty() {
                    self.discard_cards_with_cause(task.player, &discards, task.cause);
                    if !remaining.is_empty() {
                        let next = remaining.remove(0);
                        self.queue_balance_task(controller, phase, next, remaining);
                    } else if let Some(next) = phase.next() {
                        self.queue_balance_phase(controller, next);
                    }
                } else {
                    debug_assert!(discards.is_empty(), "a Balance task uses one zone action");
                    self.move_permanents_to_graveyard_then(
                        &sacrifices,
                        Some(BattlefieldExitCompletion::Balance {
                            controller,
                            phase,
                            remaining,
                        }),
                    );
                }
            }
            DecisionContinuation::SylvanOffer { player } => {
                if !options.contains(&1) {
                    return;
                }
                self.draw_cards(player, 2);
                if !self.pending_decisions.is_empty()
                    || !self.pending_events.is_empty()
                    || !self.pending_procedures.is_empty()
                {
                    self.pending_procedures
                        .push_back(PendingProcedure::SylvanAfterDraw { player });
                } else {
                    let candidates = self.sylvan_candidates(player);
                    // Two cards, or every card drawn this turn if fewer remain.
                    let choices = candidates.len().min(2);
                    if choices > 0 {
                        self.queue_sylvan_select(player, candidates, choices);
                    }
                }
            }
            DecisionContinuation::SylvanSelect {
                player,
                mut candidates,
                choices_left,
            } => {
                let selected = pending
                    .observation
                    .options
                    .iter()
                    .find(|option| options.contains(&option.id))
                    .and_then(|option| option.card)
                    .map(|(card, _)| card);
                if let Some(card) = selected {
                    candidates.retain(|candidate| *candidate != card);
                    self.queue_sylvan_mode(player, card, candidates, choices_left);
                }
            }
            DecisionContinuation::SylvanMode {
                player,
                card,
                candidates,
                choices_left,
            } => {
                if options.contains(&1) {
                    self.players[player.index()].life -= 4;
                } else if let Some(card) = remove_card(&mut self.players[player.index()].hand, card)
                {
                    let (card, _zone_change) = self.zone_change_card(card);
                    self.players[player.index()].library.push(card);
                }
                if choices_left > 1 && self.result.is_none() {
                    self.queue_sylvan_select(player, candidates, choices_left - 1);
                }
            }
            DecisionContinuation::TetravusDetach { source } => {
                // The counters have to still be there: two upkeep triggers
                // resolve one after the other, and the assemble trigger can
                // run first.
                let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                else {
                    return;
                };
                let removed = options
                    .len()
                    .min(usize::from(permanent.counters(CounterKind::PlusOnePlusOne)));
                let Ok(removed) = u16::try_from(removed) else {
                    return;
                };
                if removed == 0 {
                    return;
                }
                permanent.remove_counters(CounterKind::PlusOnePlusOne, removed);
                let controller = permanent.controller;
                for _ in 0..removed {
                    self.create_token_from(
                        controller,
                        crate::card::cards::TETRAVITE_TOKEN,
                        Some(source),
                    );
                }
            }
            DecisionContinuation::TetravusAssemble { source } => {
                let exiled = pending
                    .observation
                    .options
                    .iter()
                    .filter(|option| options.contains(&option.id))
                    .filter_map(|option| option.card)
                    .map(|(card, _)| card)
                    .collect::<Vec<_>>();
                let mut returned: u16 = 0;
                for token in exiled {
                    // Only tokens this Tetravus still owns count, in case one
                    // changed hands or left between the offer and the answer.
                    if self.battlefield.iter().any(|permanent| {
                        permanent.card.id == token && permanent.created_by == Some(source)
                    }) {
                        self.exile_permanent(token);
                        returned = returned.saturating_add(1);
                    }
                }
                if returned == 0 {
                    return;
                }
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                {
                    permanent.add_counters(CounterKind::PlusOnePlusOne, returned);
                }
            }
            DecisionContinuation::TriggerOrder { batch, remaining } => {
                self.complete_trigger_order(&batch, remaining, options);
            }
            DecisionContinuation::TriggerPlacement {
                mut trigger,
                pending,
                remaining,
                candidates,
            } => {
                let target_index = trigger.targets.len();
                let selected = options
                    .iter()
                    .filter_map(|option| {
                        usize::try_from(*option)
                            .ok()
                            .and_then(|index| candidates.get(index))
                            .copied()
                    })
                    .collect();
                let slot = TargetSlotId::from_index(target_index)
                    .expect("validated trigger targets fit the runtime slot space");
                trigger.targets.push(TargetSelection::new(slot, selected));
                let mut continued = vec![trigger];
                continued.extend(pending);
                self.place_trigger_sequence(continued, remaining);
            }
        }
    }

    pub(super) fn cancel_decision(&mut self, decision: u32) {
        debug_assert_eq!(self.pending_decisions[0].observation.id, decision);
        self.pending_decisions.remove(0);
    }

    fn resolve_nested_effect_before_later(
        &mut self,
        effect: super::ScopedEffect,
        object: &super::StackObject,
        context: super::EffectResolutionContext,
    ) {
        let mut later_procedures = std::mem::take(&mut self.pending_procedures);
        self.resolve_effect_def(effect, object, context);
        self.pending_procedures.append(&mut later_procedures);
    }
}
