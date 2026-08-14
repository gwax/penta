use crate::card::{
    ChooseDef, EffectDef, EffectRecipientDef, EffectRecipientSetDef, ObjectChoiceBindingDef,
    ObjectSetDef, PartitionItemsDef, SplitIntoPilesDef, ZoneKind,
};
use crate::{GameObjectId, ObjectSetBindingIndex};

use super::decision_offers::effect_choice_visibility;
use super::{
    DecisionContinuation, DecisionOption, DecisionPreference, DecisionVisibility, DecisionZone,
    EffectResolutionContext, Game, ScopedEffect, StackObject, Target,
};

impl Game {
    /// Offers one generic bounded, non-targeting object choice and resumes its
    /// nested effect with the selected object or group in the resolution
    /// context. Candidate `Target` values are retained beside the observation
    /// so choosing a spell cannot later be reconstructed as a permanent.
    pub(super) fn queue_effect_choice(
        &mut self,
        definition: ChooseDef,
        object: &StackObject,
        context: EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let Some(chooser) =
            self.effect_player_reference(definition.chooser, object, &context, scoped)
        else {
            return;
        };
        let excluded = definition.exclude.and_then(|reference| {
            self.effect_object_reference_id(reference, object, &context, scoped)
        });
        let mut candidates = self.effect_objects(definition.candidates, object, &context, scoped);
        candidates.retain(|candidate| target_object_id(*candidate) != excluded);

        // A mandatory single-object instruction has no decision when zero or
        // one legal objects exist: Magic does as much as possible, binding
        // none or the only object. An optional instruction still asks, because
        // declining is itself the player's choice even with one candidate.
        if definition.minimum > 0 && candidates.len() <= definition.minimum {
            let mut context = context;
            Self::bind_effect_choice(&mut context, definition.binding, candidates);
            self.resolve_effect_def(scoped.with_effect(*definition.then), object, context);
            return;
        }

        let minimum = definition.minimum.min(candidates.len());
        let maximum = definition.maximum.min(candidates.len()).max(minimum);
        let options = candidates
            .iter()
            .copied()
            .enumerate()
            .map(|(index, candidate)| self.effect_target_option(index, candidate))
            .collect();
        let preference = if effect_removes_binding(*definition.then, definition.binding) {
            if candidates.iter().all(|candidate| {
                matches!(candidate, Target::Permanent(id)
                    if self.permanent_controller(*id) == Some(chooser))
            }) {
                DecisionPreference::LowerCardValue
            } else {
                DecisionPreference::RemovalChoice
            }
        } else {
            DecisionPreference::Neutral
        };
        self.queue_decision(
            chooser,
            "Choose objects",
            effect_choice_visibility(definition.visibility),
            preference,
            minimum..=maximum,
            false,
            options,
            DecisionContinuation::ChooseForEffect {
                binding: definition.binding,
                object: Box::new(object.clone()),
                context,
                candidates,
                effect: scoped.with_effect(*definition.then),
            },
        );
    }

    pub(super) fn bind_effect_choice(
        context: &mut EffectResolutionContext,
        binding: ObjectChoiceBindingDef,
        selected: Vec<Target>,
    ) {
        match binding {
            ObjectChoiceBindingDef::Object(binding) => {
                context.bind_single_object(binding, selected.first().copied());
            }
            ObjectChoiceBindingDef::Objects(binding) => {
                context.bind_object_group(binding, selected);
            }
        }
    }

    /// Starts a public two-decision partition procedure. Top-of-library cards
    /// remain in their library throughout: the continuations retain typed
    /// references, not detached `CardInstance`s.
    pub(super) fn queue_effect_pile_split(
        &mut self,
        definition: SplitIntoPilesDef,
        object: &StackObject,
        mut context: EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let dividers = self.effect_players(definition.divider, object, &context, scoped);
        let [divider] = dividers.as_slice() else {
            return;
        };
        let divider = *divider;
        let choosers = self.effect_players(definition.chooser, object, &context, scoped);
        let [chooser] = choosers.as_slice() else {
            return;
        };
        let chooser = *chooser;
        let items = match definition.items {
            PartitionItemsDef::Objects(objects) => {
                self.effect_objects(objects, object, &context, scoped)
            }
            PartitionItemsDef::TopOfLibrary { player, count } => {
                let Some(player) = self.effect_player_reference(player, object, &context, scoped)
                else {
                    return;
                };
                let Ok(count) =
                    usize::try_from(self.effect_value(count, object, &context, scoped).max(0))
                else {
                    return;
                };
                self.players[player.index()]
                    .library
                    .iter()
                    .rev()
                    .take(count)
                    .map(|card| Target::Card(card.id))
                    .collect()
            }
        };

        if items.is_empty() {
            context.bind_object_group(definition.chosen, Vec::new());
            context.bind_object_group(definition.unchosen, Vec::new());
            self.resolve_effect_def(scoped.with_effect(*definition.then), object, context);
            return;
        }

        let options = items
            .iter()
            .copied()
            .enumerate()
            .map(|(index, item)| self.effect_target_option(index, item))
            .collect();
        self.queue_decision(
            divider,
            "Separate the objects into two piles",
            DecisionVisibility::Public,
            DecisionPreference::BalancedPartition,
            0..=items.len(),
            false,
            options,
            DecisionContinuation::SplitForEffect {
                chooser,
                items,
                chosen: definition.chosen,
                unchosen: definition.unchosen,
                object: Box::new(object.clone()),
                context,
                effect: scoped.with_effect(*definition.then),
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn queue_effect_pile_choice(
        &mut self,
        chooser: super::PlayerId,
        first: Vec<Target>,
        second: Vec<Target>,
        chosen: ObjectSetBindingIndex,
        unchosen: ObjectSetBindingIndex,
        object: Box<StackObject>,
        context: EffectResolutionContext,
        effect: ScopedEffect,
    ) {
        let options = [&first, &second]
            .into_iter()
            .enumerate()
            .map(|(index, pile)| {
                let names = pile
                    .iter()
                    .copied()
                    .map(|target| self.effect_target_option(0, target).label)
                    .collect::<Vec<_>>();
                DecisionOption {
                    id: u32::try_from(index).unwrap_or(u32::MAX),
                    label: format!(
                        "Choose pile {} ({})",
                        index + 1,
                        if names.is_empty() {
                            "empty".into()
                        } else {
                            names.join(", ")
                        }
                    ),
                    card: None,
                    members: pile
                        .iter()
                        .filter_map(|target| self.effect_target_card(*target))
                        .collect(),
                    ability_text: None,
                    zone: DecisionZone::None,
                }
            })
            .collect();
        let preference = if effect_moves_group_to_hand(effect.effect, chosen) {
            DecisionPreference::HigherCardValue
        } else if effect_sacrifices_group(effect.effect, chosen) {
            DecisionPreference::LowerCardValue
        } else {
            DecisionPreference::Neutral
        };
        self.queue_decision(
            chooser,
            "Choose a pile",
            DecisionVisibility::Public,
            preference,
            1..=1,
            false,
            options,
            DecisionContinuation::ChoosePileForEffect {
                first,
                second,
                chosen,
                unchosen,
                object,
                context,
                effect,
            },
        );
    }

    fn effect_target_option(&self, index: usize, target: Target) -> DecisionOption {
        let (label, card, zone) = match target {
            Target::Permanent(id) => self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == id)
                .map(|permanent| {
                    (
                        self.effective_permanent_name(permanent)
                            .map_or_else(|| "Unknown permanent".into(), str::to_owned),
                        Some((id, permanent.card.definition)),
                        DecisionZone::Battlefield,
                    )
                })
                .unwrap_or_else(|| ("Unknown permanent".into(), None, DecisionZone::Battlefield)),
            Target::Spell(id) => self
                .stack
                .iter()
                .find(|candidate| candidate.id == id)
                .map(|candidate| {
                    (
                        self.catalog
                            .get(candidate.card.definition)
                            .map_or_else(|| "Unknown spell".into(), |card| card.name.clone()),
                        Some((id, candidate.card.definition)),
                        DecisionZone::Stack,
                    )
                })
                .unwrap_or_else(|| ("Unknown spell".into(), None, DecisionZone::Stack)),
            Target::Card(id) => self
                .card_in_nonbattlefield_zone(id)
                .map(|(zone, card)| {
                    (
                        self.catalog.get(card.definition).map_or_else(
                            || "Unknown card".into(),
                            |definition| definition.name.clone(),
                        ),
                        Some((id, card.definition)),
                        decision_zone(zone),
                    )
                })
                .unwrap_or_else(|| ("Unknown card".into(), None, DecisionZone::None)),
            Target::Player(player) => (
                format!("Player {}", player.index() + 1),
                None,
                DecisionZone::None,
            ),
        };
        DecisionOption {
            id: u32::try_from(index).unwrap_or(u32::MAX),
            label,
            card,
            members: Vec::new(),
            ability_text: None,
            zone,
        }
    }

    fn effect_target_card(
        &self,
        target: Target,
    ) -> Option<(GameObjectId, crate::CardDefinitionId)> {
        match target {
            Target::Permanent(id) => self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == id)
                .map(|permanent| (id, permanent.card.definition)),
            Target::Spell(id) => self
                .stack
                .iter()
                .find(|candidate| candidate.id == id)
                .map(|candidate| (id, candidate.card.definition)),
            Target::Card(id) => self
                .card_in_nonbattlefield_zone(id)
                .map(|(_, card)| (id, card.definition)),
            Target::Player(_) => None,
        }
    }
}

fn target_object_id(target: Target) -> Option<GameObjectId> {
    match target {
        Target::Player(_) => None,
        Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => Some(id),
    }
}

const fn decision_zone(zone: ZoneKind) -> DecisionZone {
    match zone {
        ZoneKind::Library => DecisionZone::Library,
        ZoneKind::Hand => DecisionZone::Hand,
        ZoneKind::Battlefield => DecisionZone::Battlefield,
        ZoneKind::Graveyard => DecisionZone::Graveyard,
        ZoneKind::Stack => DecisionZone::Stack,
        ZoneKind::Exile => DecisionZone::Exile,
        ZoneKind::Command => DecisionZone::Command,
    }
}

fn recipient_uses_binding(recipient: EffectRecipientDef, binding: ObjectChoiceBindingDef) -> bool {
    match (recipient.0, binding) {
        (
            EffectRecipientSetDef::Objects(ObjectSetDef::One(crate::card::ObjectRefDef::Binding(
                recipient,
            ))),
            ObjectChoiceBindingDef::Object(binding),
        ) => recipient == binding,
        (
            EffectRecipientSetDef::Objects(ObjectSetDef::Binding(recipient)),
            ObjectChoiceBindingDef::Objects(binding),
        ) => recipient == binding,
        _ => false,
    }
}

fn effect_removes_binding(effect: EffectDef, binding: ObjectChoiceBindingDef) -> bool {
    match effect {
        EffectDef::Destroy { object, .. }
        | EffectDef::Sacrifice { object }
        | EffectDef::MoveToZone {
            object,
            zone: ZoneKind::Graveyard | ZoneKind::Exile,
            ..
        } => recipient_uses_binding(object, binding),
        EffectDef::Sequence(effects) => effects
            .iter()
            .copied()
            .any(|effect| effect_removes_binding(effect, binding)),
        EffectDef::Randomized {
            on_success,
            on_failure,
            ..
        } => {
            effect_removes_binding(*on_success, binding)
                || effect_removes_binding(*on_failure, binding)
        }
        EffectDef::Choose(definition) => effect_removes_binding(*definition.then, binding),
        EffectDef::PayOr(definition) => {
            definition
                .if_paid
                .is_some_and(|effect| effect_removes_binding(*effect, binding))
                || definition
                    .otherwise
                    .is_some_and(|effect| effect_removes_binding(*effect, binding))
        }
        EffectDef::SplitIntoPiles(definition) => effect_removes_binding(*definition.then, binding),
        EffectDef::May { effect, .. }
        | EffectDef::IfCondition { then: effect, .. }
        | EffectDef::ReplaceNextDrawThisTurn { effect, .. } => {
            effect_removes_binding(*effect, binding)
        }
        EffectDef::InstallTrigger(installed) => installed
            .ability
            .declarative_effect()
            .is_some_and(|effect| effect_removes_binding(effect, binding)),
        EffectDef::IfFormat {
            then, otherwise, ..
        } => effect_removes_binding(*then, binding) || effect_removes_binding(*otherwise, binding),
        EffectDef::SacrificeOfChoice {
            then: Some(effect), ..
        } => effect_removes_binding(*effect, binding),
        _ => false,
    }
}

fn effect_moves_group_to_hand(effect: EffectDef, binding: ObjectSetBindingIndex) -> bool {
    effect_matches_group_operation(effect, binding, GroupOperation::MoveToHand)
}

fn effect_sacrifices_group(effect: EffectDef, binding: ObjectSetBindingIndex) -> bool {
    effect_matches_group_operation(effect, binding, GroupOperation::Sacrifice)
}

#[derive(Clone, Copy)]
enum GroupOperation {
    MoveToHand,
    Sacrifice,
}

fn effect_matches_group_operation(
    effect: EffectDef,
    binding: ObjectSetBindingIndex,
    operation: GroupOperation,
) -> bool {
    let recipient_matches = |recipient: EffectRecipientDef| {
        recipient_uses_binding(recipient, ObjectChoiceBindingDef::Objects(binding))
    };
    match effect {
        EffectDef::MoveToZone {
            object,
            zone: ZoneKind::Hand,
            ..
        } if matches!(operation, GroupOperation::MoveToHand) => recipient_matches(object),
        EffectDef::Sacrifice { object } if matches!(operation, GroupOperation::Sacrifice) => {
            recipient_matches(object)
        }
        EffectDef::Sequence(effects) => effects
            .iter()
            .copied()
            .any(|effect| effect_matches_group_operation(effect, binding, operation)),
        EffectDef::Randomized {
            on_success,
            on_failure,
            ..
        } => {
            effect_matches_group_operation(*on_success, binding, operation)
                || effect_matches_group_operation(*on_failure, binding, operation)
        }
        EffectDef::Choose(definition) => {
            effect_matches_group_operation(*definition.then, binding, operation)
        }
        EffectDef::PayOr(definition) => {
            definition
                .if_paid
                .is_some_and(|effect| effect_matches_group_operation(*effect, binding, operation))
                || definition.otherwise.is_some_and(|effect| {
                    effect_matches_group_operation(*effect, binding, operation)
                })
        }
        EffectDef::SplitIntoPiles(definition) => {
            effect_matches_group_operation(*definition.then, binding, operation)
        }
        EffectDef::May { effect, .. }
        | EffectDef::IfCondition { then: effect, .. }
        | EffectDef::ReplaceNextDrawThisTurn { effect, .. } => {
            effect_matches_group_operation(*effect, binding, operation)
        }
        EffectDef::InstallTrigger(installed) => installed
            .ability
            .declarative_effect()
            .is_some_and(|effect| effect_matches_group_operation(effect, binding, operation)),
        EffectDef::IfFormat {
            then, otherwise, ..
        } => {
            effect_matches_group_operation(*then, binding, operation)
                || effect_matches_group_operation(*otherwise, binding, operation)
        }
        EffectDef::SacrificeOfChoice {
            then: Some(effect), ..
        } => effect_matches_group_operation(*effect, binding, operation),
        _ => false,
    }
}
