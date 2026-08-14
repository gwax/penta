use super::{
    CharacteristicSource, ColorSet, DecisionContinuation, DecisionKind, DecisionObservation,
    DecisionOption, DecisionPreference, DecisionVisibility, DecisionZone, DeclarativeAbilityDef,
    EffectResolutionContext, FORK_COPY_COLOR, Game, ManaCost, PendingDecision, PlayerId,
    ResolvedEffectPayment, ScopedEffect, StackObject, Target, TargetSelection, TargetSlotId,
    TriggerContext, flatten_target_selections, target_combinations,
};
use crate::card::ChoiceVisibilityDef;

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
        if !can_pay {
            if let Some(effect) = otherwise {
                self.resolve_effect_def(effect, object, context);
                return;
            }
        }
        let mut options = vec![DecisionOption {
            id: 0,
            label: "Decline".into(),
            card: None,
            members: Vec::new(),
            ability_text: None,
            zone: DecisionZone::None,
        }];
        if can_pay {
            options.push(DecisionOption {
                id: 1,
                label: Self::effect_payment_label(payment),
                card: None,
                members: Vec::new(),
                ability_text: None,
                zone: DecisionZone::None,
            });
        }
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

    pub(super) fn can_pay_effect_payment(
        &self,
        player: PlayerId,
        payment: ResolvedEffectPayment,
    ) -> bool {
        match payment {
            ResolvedEffectPayment::Mana(cost) => self.can_pay_cost(player, cost, 0),
            ResolvedEffectPayment::Life(amount) => i16::try_from(amount)
                .is_ok_and(|amount| self.players[player.index()].life >= amount),
        }
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
        }
        true
    }

    pub(super) fn effect_payment_label(payment: ResolvedEffectPayment) -> String {
        match payment {
            ResolvedEffectPayment::Mana(_) => "Pay the cost".to_string(),
            ResolvedEffectPayment::Life(amount) => format!("Pay {amount} life"),
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
        let target_lists = self.copy_target_choices(&spell, player);
        if spell
            .signature
            .as_ref()
            .is_some_and(|signature| signature.targets().is_empty())
        {
            self.push_copy_with_colors(spell, player, Vec::new(), Some(FORK_COPY_COLOR));
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
            "Choose targets for Fork's copy",
            DecisionVisibility::Private,
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::Fork {
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
