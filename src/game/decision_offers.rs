use super::{
    CardInstance, CharacteristicContext, CharacteristicSource, ColorSet, DecisionContinuation,
    DecisionKind, DecisionObservation, DecisionOption, DecisionPreference, DecisionVisibility,
    DecisionZone, DeclarativeAbilityDef, EffectResolutionContext, FORK_COPY_COLOR, Game, ManaCost,
    PendingDecision, PlayerId, ResolvedEffectPayment, ScopedEffect, StackObject, Target,
    TargetSelection, TargetSlotId, TriggerContext, ZoneKind, ZoneMoveCause, ZonePlacement,
    flatten_target_selections, target_combinations,
};
use crate::card::{ChoiceVisibilityDef, ObjectPredicateDef};
use crate::ids::GameObjectId;

pub(super) const fn effect_choice_visibility(
    visibility: ChoiceVisibilityDef,
) -> DecisionVisibility {
    match visibility {
        ChoiceVisibilityDef::Public => DecisionVisibility::Public,
        ChoiceVisibilityDef::Private => DecisionVisibility::Private,
    }
}

impl Game {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn queue_decision(
        &mut self,
        player: PlayerId,
        prompt: impl Into<String>,
        visibility: DecisionVisibility,
        preference: DecisionPreference,
        bounds: std::ops::RangeInclusive<usize>,
        cancellable: bool,
        options: Vec<DecisionOption>,
        continuation: DecisionContinuation,
    ) {
        // A player can only choose from what is there. Asking for a minimum
        // the options cannot supply leaves no legal `ChooseDecision`, because
        // `is_legal` requires at least `minimum` of them — and when the
        // decision is also not cancellable, the game has no legal action at
        // all and deadlocks. Demonic Tutor did exactly that on an empty
        // library. Magic resolves as much of an effect as it can, so lower the
        // requirement to what exists and let the continuation take it from
        // there; each one already handles being handed nothing.
        let minimum = (*bounds.start()).min(options.len());

        let id = self.next_decision_id;
        self.next_decision_id = self.next_decision_id.saturating_add(1);
        self.pending_decisions.push(PendingDecision {
            observation: DecisionObservation {
                id,
                player,
                kind: DecisionKind::Choice,
                order_semantics: None,
                prompt: prompt.into(),
                visibility,
                preference,
                minimum,
                maximum: (*bounds.end()).max(minimum),
                cancellable,
                options,
            },
            continuation,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn queue_pay_or(
        &mut self,
        player: PlayerId,
        payment: ResolvedEffectPayment,
        visibility: ChoiceVisibilityDef,
        definition: ScopedEffect,
        object: &StackObject,
        context: EffectResolutionContext,
        if_paid: Option<ScopedEffect>,
        otherwise: Option<ScopedEffect>,
    ) {
        if if_paid.is_none() && otherwise.is_none() {
            return;
        }
        let can_pay = self.can_pay_effect_payment(player, payment);
        if !can_pay && let Some(effect) = otherwise {
            self.resolve_effect_def(effect, object, context);
            return;
        }
        let options = self.payment_options(player, payment, can_pay, "Decline");
        self.queue_decision(
            player,
            object.ability_text().unwrap_or("Pay the cost?"),
            effect_choice_visibility(visibility),
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::PayOr {
                player,
                payment,
                definition,
                object: Box::new(object.clone()),
                context,
                if_paid,
                otherwise,
            },
        );
    }

    /// Applies a payment decision's answer: option zero declines, and every
    /// other option is a way of paying. Only a matching discard has more than
    /// one, and its option carries the card that goes.
    pub(super) fn settle_payment_decision(
        &mut self,
        player: PlayerId,
        payment: ResolvedEffectPayment,
        answered: &[u32],
        options: &[DecisionOption],
    ) -> Option<u16> {
        let chosen = answered.iter().copied().find(|option| *option != 0)?;
        match payment {
            // The option id is the amount, so the answer carries how much was
            // paid without a second question.
            ResolvedEffectPayment::ChosenGenericMana => {
                let amount = u16::try_from(chosen).unwrap_or(u16::MAX);
                let cost = ManaCost::new(amount, 0);
                if !self.can_pay_cost(player, cost, 0) {
                    return None;
                }
                self.activate_mana_for_cost(player, cost, 0);
                let _spent = self.pay_player_cost(player, cost, 0);
                Some(amount)
            }
            ResolvedEffectPayment::ReturnPermanentMatching(predicate) => {
                let permanent = options
                    .iter()
                    .find(|option| option.id == chosen)
                    .and_then(|option| option.card)
                    .map(|(permanent, _)| permanent)?;
                if !self
                    .matching_permanents_controlled(player, predicate)
                    .contains(&permanent)
                {
                    return None;
                }
                self.move_target_to_zone(
                    Target::Permanent(permanent),
                    ZoneKind::Hand,
                    ZoneMoveCause::Effect { controller: player },
                    None,
                    ZonePlacement::Top,
                );
                Some(0)
            }
            ResolvedEffectPayment::SacrificePermanentMatching(predicate) => {
                let permanent = options
                    .iter()
                    .find(|option| option.id == chosen)
                    .and_then(|option| option.card)
                    .map(|(permanent, _)| permanent)?;
                if !self
                    .matching_permanents_controlled(player, predicate)
                    .contains(&permanent)
                {
                    return None;
                }
                self.move_permanents_to_graveyard(&[permanent]);
                Some(0)
            }
            ResolvedEffectPayment::DiscardMatching(predicate) => {
                let card = options
                    .iter()
                    .find(|option| option.id == chosen)
                    .and_then(|option| option.card)
                    .map(|(card, _)| card)?;
                self.pay_matching_discard(player, predicate, card)
                    .then_some(0)
            }
            payment => (chosen == 1 && self.pay_effect_payment(player, payment)).then_some(0),
        }
    }

    /// The largest generic payment the player could make right now, which is
    /// what a chosen-amount payment offers. Read through the ordinary cost
    /// check so an unspendable source cannot inflate the list.
    pub(super) fn maximum_generic_payment(&self, player: PlayerId) -> u16 {
        let mut amount = 0;
        while amount < u16::MAX && self.can_pay_cost(player, ManaCost::new(amount + 1, 0), 0) {
            amount += 1;
        }
        amount
    }

    /// Asks `player` to name a card while an effect resolves, then continues
    /// that effect with the answer. The catalog supplies the list, which is
    /// the same one an entering permanent's naming choice offers.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn queue_card_name_choice(
        &mut self,
        player: PlayerId,
        nonland_only: bool,
        searched: PlayerId,
        zone: ZoneKind,
        binding: crate::ObjectSetBindingIndex,
        object: StackObject,
        context: EffectResolutionContext,
        effect: ScopedEffect,
    ) {
        let choice = if nonland_only {
            crate::card::BattlefieldEntryScalarChoiceDef::NONLAND_CARD_NAME
        } else {
            crate::card::BattlefieldEntryScalarChoiceDef::CARD_NAME
        };
        let (prompt, choices) = self.entry_scalar_choices(player, choice);
        let options = choices
            .iter()
            .enumerate()
            .map(|(index, value)| DecisionOption {
                id: u32::try_from(index).unwrap_or(u32::MAX),
                label: value.clone(),
                card: None,
                members: Vec::new(),
                ability_text: None,
                zone: DecisionZone::None,
            })
            .collect();
        self.queue_decision(
            player,
            prompt,
            DecisionVisibility::Public,
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::CardNameChoice {
                choices,
                searched,
                zone,
                binding,
                object: Box::new(object),
                context,
                effect,
            },
        );
    }

    pub(super) fn can_pay_effect_payment(
        &self,
        player: PlayerId,
        payment: ResolvedEffectPayment,
    ) -> bool {
        match payment {
            ResolvedEffectPayment::Mana(cost) => self.can_pay_cost(player, cost, 0),
            ResolvedEffectPayment::Life(amount) => i16::try_from(amount)
                .is_ok_and(|amount| self.players[player.index()].life >= amount),
            // A short library is not a failure to pay, so this is always
            // affordable. Running out of cards is answered by the draw that
            // finds none, not by refusing the payment.
            ResolvedEffectPayment::Mill(_) => true,
            // A discard needs cards to choose from, so an empty hand cannot
            // pay at all. That is the difference from a mill, where a short
            // library still pays with what it has.
            ResolvedEffectPayment::Discard(amount) => {
                self.players[player.index()].hand.len() >= usize::from(amount)
            }
            // A hand full of spells cannot pay for a land, which is the whole
            // difference between this and the count above.
            ResolvedEffectPayment::DiscardMatching(predicate) => {
                !self.matching_cards_in_hand(player, predicate).is_empty()
            }
            // Paying nothing is not paying, so this needs one generic mana
            // before the choice is worth offering at all.
            ResolvedEffectPayment::ChosenGenericMana => {
                self.can_pay_cost(player, ManaCost::new(1, 0), 0)
            }
            ResolvedEffectPayment::ReturnPermanentMatching(predicate)
            | ResolvedEffectPayment::SacrificePermanentMatching(predicate) => !self
                .matching_permanents_controlled(player, predicate)
                .is_empty(),
        }
    }

    /// The payer's own cards that a payment predicate matches, in hand order.
    /// This is the candidate list a payment decision offers and the one its
    /// checkpoint rebuilds, so both read it from the same place.
    pub(super) fn matching_cards_in_hand(
        &self,
        player: PlayerId,
        predicate: ObjectPredicateDef,
    ) -> Vec<CardInstance> {
        self.players[player.index()]
            .hand
            .iter()
            .filter(|card| {
                self.printed_trigger_event_object(
                    card.id,
                    card.definition,
                    player,
                    &CharacteristicContext::Hand,
                )
                .is_some_and(|object| {
                    self.trigger_object_matches(predicate, &object, card.id, false)
                })
            })
            .cloned()
            .collect()
    }

    /// The payer's own permanents a payment predicate matches, in battlefield
    /// order. Read by the option list and by the payment that follows it, so
    /// a permanent that stopped matching in between cannot be spent.
    pub(super) fn matching_permanents_controlled(
        &self,
        player: PlayerId,
        predicate: ObjectPredicateDef,
    ) -> Vec<GameObjectId> {
        self.battlefield
            .iter()
            .filter(|permanent| permanent.controller == player)
            .filter(|permanent| {
                self.trigger_object_matches_for_controller(
                    predicate,
                    &self.trigger_event_object(permanent),
                    permanent.card.id,
                    false,
                    Some(player),
                )
            })
            .map(|permanent| permanent.card.id)
            .collect()
    }

    /// The options a payment decision offers: declining, and then one entry
    /// per way of paying. Everything but a matching discard has exactly one
    /// way, so it stays the single option it has always been.
    pub(super) fn payment_options(
        &self,
        player: PlayerId,
        payment: ResolvedEffectPayment,
        can_pay: bool,
        decline: &str,
    ) -> Vec<DecisionOption> {
        let mut options = vec![DecisionOption {
            id: 0,
            label: decline.into(),
            card: None,
            members: Vec::new(),
            ability_text: None,
            zone: DecisionZone::None,
        }];
        if !can_pay {
            return options;
        }
        match payment {
            // One option per amount the payer can actually afford, with the
            // amount as the option id.
            ResolvedEffectPayment::ChosenGenericMana => {
                for amount in 1..=self.maximum_generic_payment(player) {
                    options.push(DecisionOption {
                        id: u32::from(amount),
                        label: format!("Pay {{{amount}}}"),
                        card: None,
                        members: Vec::new(),
                        ability_text: None,
                        zone: DecisionZone::None,
                    });
                }
            }
            ResolvedEffectPayment::ReturnPermanentMatching(predicate)
            | ResolvedEffectPayment::SacrificePermanentMatching(predicate) => {
                let returning =
                    matches!(payment, ResolvedEffectPayment::ReturnPermanentMatching(_));
                for (index, permanent) in self
                    .matching_permanents_controlled(player, predicate)
                    .into_iter()
                    .enumerate()
                {
                    let name = self
                        .permanent_card_name(permanent)
                        .map_or_else(|| "a permanent".to_string(), ToOwned::to_owned);
                    options.push(DecisionOption {
                        id: u32::try_from(index + 1).unwrap_or(u32::MAX),
                        label: if returning {
                            format!("Return {name}")
                        } else {
                            format!("Sacrifice {name}")
                        },
                        card: self
                            .battlefield
                            .iter()
                            .find(|candidate| candidate.card.id == permanent)
                            .map(|candidate| (permanent, candidate.card.definition)),
                        members: Vec::new(),
                        ability_text: None,
                        zone: DecisionZone::Battlefield,
                    });
                }
            }
            ResolvedEffectPayment::DiscardMatching(predicate) => {
                for (index, card) in self
                    .matching_cards_in_hand(player, predicate)
                    .into_iter()
                    .enumerate()
                {
                    let name = self
                        .catalog
                        .get(card.definition)
                        .map_or_else(|| "a card".to_string(), |card| card.name.clone());
                    options.push(DecisionOption {
                        id: u32::try_from(index + 1).unwrap_or(u32::MAX),
                        label: format!("Discard {name}"),
                        card: Some((card.id, card.definition)),
                        members: Vec::new(),
                        ability_text: None,
                        zone: DecisionZone::Hand,
                    });
                }
            }
            payment => options.push(DecisionOption {
                id: 1,
                label: Self::effect_payment_label(payment),
                card: None,
                members: Vec::new(),
                ability_text: None,
                zone: DecisionZone::None,
            }),
        }
        options
    }

    pub(super) fn pay_effect_payment(
        &mut self,
        player: PlayerId,
        payment: ResolvedEffectPayment,
    ) -> bool {
        if !self.can_pay_effect_payment(player, payment) {
            return false;
        }
        match payment {
            ResolvedEffectPayment::Mana(cost) => {
                self.activate_mana_for_cost(player, cost, 0);
                let _spent = self.pay_player_cost(player, cost, 0);
            }
            ResolvedEffectPayment::Life(amount) => self.lose_life(player, amount),
            ResolvedEffectPayment::Mill(amount) => {
                let milled = self.take_top_of_library(player, usize::from(amount));
                self.bury_cards(player, milled);
            }
            // Queued rather than resolved here: the payer has already chosen
            // to pay, and which cards go is a separate choice that the branch
            // taken above does not depend on.
            ResolvedEffectPayment::Discard(amount) => self.queue_effect_discards(
                vec![player],
                i32::from(amount),
                ZoneMoveCause::Effect { controller: player },
            ),
            // Both are paid by [`Self::settle_payment_decision`], which knows
            // which card was named or how much was chosen. Reaching here
            // means a caller lost that answer.
            ResolvedEffectPayment::DiscardMatching(_)
            | ResolvedEffectPayment::ChosenGenericMana
            | ResolvedEffectPayment::ReturnPermanentMatching(_)
            | ResolvedEffectPayment::SacrificePermanentMatching(_) => return false,
        }
        true
    }

    /// Pays a matching discard with the card the payer named. The card is
    /// checked against the predicate again rather than trusted: the option
    /// list was built before the decision was answered.
    pub(super) fn pay_matching_discard(
        &mut self,
        player: PlayerId,
        predicate: ObjectPredicateDef,
        card: GameObjectId,
    ) -> bool {
        if !self
            .matching_cards_in_hand(player, predicate)
            .iter()
            .any(|candidate| candidate.id == card)
        {
            return false;
        }
        self.discard_cards_with_cause(
            player,
            &[card],
            ZoneMoveCause::Effect { controller: player },
        );
        true
    }

    pub(super) fn effect_payment_label(payment: ResolvedEffectPayment) -> String {
        match payment {
            ResolvedEffectPayment::Mana(_) => "Pay the cost".to_string(),
            ResolvedEffectPayment::Life(amount) => format!("Pay {amount} life"),
            ResolvedEffectPayment::Mill(amount) => format!("Mill {amount} cards"),
            ResolvedEffectPayment::Discard(amount) => format!("Discard {amount} cards"),
            // Every candidate carries its own label, so this one only names
            // the prompt the decision is introduced with.
            ResolvedEffectPayment::DiscardMatching(_) => "Discard a matching card".to_string(),
            ResolvedEffectPayment::ChosenGenericMana => "Pay {X}".to_string(),
            ResolvedEffectPayment::ReturnPermanentMatching(_) => {
                "Return a matching permanent".to_string()
            }
            ResolvedEffectPayment::SacrificePermanentMatching(_) => {
                "Sacrifice a matching permanent".to_string()
            }
        }
    }

    /// Offers an effect its controller may decline, resolving it only on a
    /// yes. Declining is always available, which is what "may" means.
    pub(super) fn queue_optional_effect(
        &mut self,
        player: PlayerId,
        object: &StackObject,
        context: EffectResolutionContext,
        effect: ScopedEffect,
    ) {
        self.queue_decision(
            player,
            object.ability_text().unwrap_or("Use this optional effect?"),
            DecisionVisibility::Public,
            DecisionPreference::PreferOption(1),
            1..=1,
            false,
            vec![
                DecisionOption {
                    id: 0,
                    label: "Decline".into(),
                    card: None,
                    members: Vec::new(),
                    ability_text: None,
                    zone: DecisionZone::None,
                },
                DecisionOption {
                    id: 1,
                    label: "Do it".into(),
                    card: None,
                    members: Vec::new(),
                    ability_text: None,
                    zone: DecisionZone::None,
                },
            ],
            DecisionContinuation::OptionalEffect {
                object: Box::new(object.clone()),
                context,
                effect,
            },
        );
    }

    pub(super) fn target_label(&self, viewer: PlayerId, target: Target) -> String {
        match target {
            Target::Player(player) if player == viewer => "you".into(),
            Target::Player(_) => "your opponent".into(),
            Target::Card(id) => self
                .card_in_nonbattlefield_zone(id)
                .and_then(|(_, card)| self.catalog.get(card.definition))
                .map_or_else(|| "that card".into(), |card| card.name.clone()),
            Target::Permanent(id) => self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == id)
                .and_then(|permanent| self.catalog.get(permanent.card.definition))
                .map_or_else(|| "that permanent".into(), |card| card.name.clone()),
            Target::Spell(id) => self
                .stack
                .iter()
                .find(|object| object.id == id)
                .and_then(|object| self.catalog.get(object.card.definition))
                .map_or_else(|| "that spell".into(), |card| card.name.clone()),
        }
    }

    pub(super) fn queue_chain_lightning_decision(&mut self, player: PlayerId, spell: StackObject) {
        // Without RR to spend there is nothing to decide, and a prompt whose
        // only answer is "no" is worse than no prompt at all.
        if !self.can_pay_cost(player, ManaCost::new(0, 2), 0) {
            return;
        }
        let mut targets = self.damage_targets();
        if let Some(target) = spell.first_target()
            && !targets.contains(&target)
        {
            targets.push(target);
        }
        let mut options = vec![DecisionOption {
            id: 0,
            label: "Don't copy Chain Lightning".into(),
            card: None,
            members: Vec::new(),
            ability_text: None,
            zone: DecisionZone::None,
        }];
        options.extend(
            targets
                .iter()
                .enumerate()
                .map(|(index, target)| DecisionOption {
                    id: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    label: format!(
                        "Copy Chain Lightning → {}",
                        self.target_label(player, *target)
                    ),
                    card: None,
                    members: Vec::new(),
                    ability_text: None,
                    zone: DecisionZone::None,
                }),
        );
        self.queue_decision(
            player,
            "Copy Chain Lightning?",
            DecisionVisibility::Private,
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::ChainLightning {
                player,
                spell,
                targets,
            },
        );
    }

    pub(super) fn queue_fork_decision(&mut self, player: PlayerId, spell: StackObject) {
        self.queue_copy_decision(player, spell, Some(FORK_COPY_COLOR), "Fork's copy");
    }

    /// Offers a copy of `spell` under `player`, letting them retarget it. Fork
    /// repaints what it copies and a card copying itself does not, so the
    /// colours are the caller's to decide.
    pub(super) fn queue_copy_decision(
        &mut self,
        player: PlayerId,
        spell: StackObject,
        colors: Option<ColorSet>,
        described: &str,
    ) {
        self.queue_copy_decision_chain(player, spell, colors, described, 1);
    }

    /// The same, several times over. Each copy is targeted before the next is
    /// offered, which is what storm's "you may choose new targets for the
    /// copies" means: the copies are separate objects with separate choices.
    pub(super) fn queue_copy_decision_chain(
        &mut self,
        player: PlayerId,
        spell: StackObject,
        colors: Option<ColorSet>,
        described: &str,
        copies: u16,
    ) {
        if copies == 0 {
            return;
        }
        let remaining = copies - 1;
        let target_lists = self.copy_target_choices(&spell, player);
        if spell
            .signature
            .as_ref()
            .is_some_and(|signature| signature.targets().is_empty())
        {
            for _ in 0..copies {
                self.push_copy_with_colors(spell.clone(), player, Vec::new(), colors);
            }
            return;
        }
        let original_targets = spell.targets();
        let options = target_lists
            .iter()
            .enumerate()
            .map(|(index, targets)| DecisionOption {
                id: u32::try_from(index).unwrap_or(u32::MAX),
                label: if flatten_target_selections(targets) == original_targets {
                    "Keep original targets".into()
                } else {
                    let labels = flatten_target_selections(targets)
                        .iter()
                        .map(|target| self.target_label(player, *target))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("Copy with targets {labels}")
                },
                card: None,
                members: Vec::new(),
                ability_text: None,
                zone: DecisionZone::None,
            })
            .collect();
        self.queue_decision(
            player,
            format!("Choose targets for {described}"),
            DecisionVisibility::Private,
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::Fork {
                colors,
                remaining,
                player,
                spell,
                target_lists,
            },
        );
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn copy_target_choices(
        &self,
        spell: &StackObject,
        player: PlayerId,
    ) -> Vec<Vec<TargetSelection>> {
        let Some(signature) = &spell.signature else {
            return Vec::new();
        };
        if signature.targets().is_empty() {
            return vec![Vec::new()];
        }
        let Some(definition) = self.catalog.get(spell.card.definition) else {
            return vec![signature.targets().to_vec()];
        };
        let Some(option) = definition.play_option(signature.play_option()) else {
            return vec![signature.targets().to_vec()];
        };
        let declarative_slots = spell
            .ability
            .as_ref()
            .map(|ability| ability.target_defs.clone())
            .filter(|slots| !slots.is_empty())
            .or_else(|| {
                Self::spell_ability(definition, option).and_then(|(_, ability)| {
                    let DeclarativeAbilityDef::Spell(spell) = ability.definition else {
                        return None;
                    };
                    Self::selected_spell_plan(spell, signature.modes())
                        .map(|plan| plan.target_defs)
                        .filter(|targets| !targets.is_empty())
                })
            });
        if let Some(slots) = declarative_slots {
            let context = spell
                .ability
                .as_ref()
                .map_or_else(TriggerContext::empty, |ability| ability.context.trigger);
            let mut choices = vec![Vec::new()];
            for original in signature.targets() {
                let Some(slot) = slots.get(original.slot().index()) else {
                    return vec![signature.targets().to_vec()];
                };
                let mut replacements = target_combinations(
                    &self.ability_targets_matching(slot.predicate, player, spell.id, context),
                    original.targets().len(),
                )
                .into_iter()
                .map(|targets| TargetSelection::new(original.slot(), targets))
                .collect::<Vec<_>>();
                // Copy effects may keep the original target even if it has
                // since become illegal; normal resolution will then apply
                // the usual target-legality rules to the copy.
                replacements.push(original.clone());
                replacements.sort_unstable_by_key(|selection| selection.targets().to_vec());
                replacements.dedup();
                let mut combined = Vec::new();
                for prefix in &choices {
                    for replacement in &replacements {
                        let mut selected = prefix.clone();
                        selected.push(replacement.clone());
                        combined.push(selected);
                    }
                }
                choices = combined;
            }
            return choices;
        }
        let slots = Self::target_slots_for(option, signature.modes());
        if Self::uses_legacy_behavior_targets(definition, option) {
            let Some(behavior) = Self::play_option_behavior(definition, option) else {
                return vec![signature.targets().to_vec()];
            };
            let mut choices = self
                .legal_target_lists(behavior, player, Some(signature.iter_targets().count()))
                .into_iter()
                .map(|targets| {
                    if targets.is_empty() {
                        Vec::new()
                    } else {
                        vec![TargetSelection::new(TargetSlotId(0), targets)]
                    }
                })
                .collect::<Vec<_>>();
            choices.push(signature.targets().to_vec());
            choices.sort_unstable_by_key(|targets| flatten_target_selections(targets));
            choices.dedup();
            return choices;
        }

        let mut choices = vec![Vec::new()];
        for original in signature.targets() {
            let Some(slot) = slots.iter().find(|slot| slot.id == original.slot()) else {
                return vec![signature.targets().to_vec()];
            };
            let mut replacements = target_combinations(
                &self.targets_matching(slot.predicate),
                original.targets().len(),
            )
            .into_iter()
            .map(|targets| TargetSelection::new(slot.id, targets))
            .collect::<Vec<_>>();
            replacements.push(original.clone());
            replacements.sort_unstable_by_key(|selection| selection.targets().to_vec());
            replacements.dedup();
            let mut combined = Vec::new();
            for prefix in &choices {
                for replacement in &replacements {
                    let mut selected = prefix.clone();
                    selected.push(replacement.clone());
                    combined.push(selected);
                }
            }
            choices = combined;
        }
        choices
    }

    pub(super) fn push_copy(
        &mut self,
        spell: StackObject,
        player: PlayerId,
        targets: Vec<TargetSelection>,
    ) {
        self.push_copy_with_colors(spell, player, targets, None);
    }

    /// A copy effect may repaint what it copies, as Fork does. The override
    /// replaces the printed colours outright rather than adding to them.
    pub(super) fn push_copy_with_colors(
        &mut self,
        mut spell: StackObject,
        player: PlayerId,
        targets: Vec<TargetSelection>,
        colors: Option<ColorSet>,
    ) {
        spell.colors = colors;
        let definition = spell.card.definition;
        let card = self.unbacked_object(definition, player, CharacteristicSource::Copy(definition));
        spell.id = card.id;
        spell.card = card;
        spell.source = None;
        spell.controller = player;
        if let Some(ability) = &mut spell.ability {
            ability.targets.clone_from(&targets);
        }
        spell.signature = spell.signature.as_ref().map(|signature| {
            signature
                .copy_with_targets(targets)
                .expect("copy replacement retains target slots and cardinality")
        });
        // Effects attached by mana spent on the original spell are not
        // copiable values. The copy keeps printed static abilities through
        // its definition, but it was not paid for with that mana.
        spell.applied_effects.clear();
        // Text-changing effects are not copiable values.
        spell.text_changes.clear();
        spell.is_copy = true;
        self.stack.push(spell);
    }
}
