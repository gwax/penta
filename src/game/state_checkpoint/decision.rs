use serde_json::Value;

use crate::card::{
    CardType, CardTypeSet, EffectDef, EffectPaymentCostDef, EffectPaymentDef, ReplacementChoiceDef,
    ReplacementEventDef, TurnKindDef, ZonePlacement,
};
use crate::{CardDefinitionId, GameObjectId, ManaCost, PlayerId};

use super::super::decision_offers::effect_choice_visibility;
use super::super::{
    AbilitySourceRef, ApplicableBeginTurnReplacement, BalanceAction, BalancePhase, BalanceTask,
    DecisionContinuation, DecisionKind, DecisionObservation, DecisionOption,
    DecisionOrderSemantics, DecisionPreference, DecisionVisibility, DecisionZone,
    DeferredBeginTurnEffect, PendingDecision, PendingTrigger, PileSplit, SacrificeFollowup,
    ScopedEffect, Target, TriggerPlacementBatch,
};
use super::model::{
    AbilityLocator, AbilitySourceSnapshot, ApplicableBeginTurnReplacementSnapshot,
    BalanceActionSnapshot, BalancePhaseSnapshot, BalanceTaskSnapshot, DecisionCardOriginSnapshot,
    DecisionCardSnapshot, DecisionContinuationSnapshot, DecisionOptionSnapshot,
    DecisionPreferenceSnapshot, DecisionStateSnapshot, DecisionZoneSnapshot,
    DeferredBeginTurnEffectSnapshot, DetachedCardSnapshot, DiscardChoiceSnapshot,
    EffectContinuationSnapshot, PendingTriggerSnapshot, PileSplitSnapshot,
    ReplacementEffectContextSnapshot, ReplacementEffectLocator, TriggerPlacementBatchSnapshot,
    TurnKindSnapshot, ZoneMoveCauseSnapshot, ZonePlacementSnapshot,
};
mod option;
use option::parse_option;

use super::procedure::{draw_replacement_snapshot_allowing, parse_draw_replacement};
use super::semantics::{
    ability_locator, ability_target_defs, catalog_ability, catalog_replacement_effect,
    catalog_scoped_effect, replacement_effect_locator_matches_source, replacement_effects,
    resolved_replacement_effect_locator, scoped_effect_snapshot,
};
use super::stack::{
    detached_stack_snapshot_allowing, effect_resolution_context_snapshot,
    object_reference_requires_hidden_rebinding, parse_detached_stack,
    parse_effect_resolution_context, parse_target, parse_target_selection, referenced_object_ids,
    resolution_context_referenced_object_ids, stack_ability_snapshot_allowing,
    target_selection_snapshot, target_selections_referenced_object_ids, target_snapshot,
    trigger_capture_has_unrebindable_hidden_reference,
    trigger_capture_has_unrebindable_hidden_reference_except,
};
use super::{
    DeclarativeAbilityDef, Game, ReplacementEffectContext, ReplacementEffectDef, ZoneMoveCause,
    ability_origin_from_snapshot, ability_origin_snapshot, applicable_replacement_snapshot, array,
    bool_field, card, field, parse_applicable_replacement, parse_zone_kind, seat_value, str_field,
    u32_field, usize_field, zone_kind_snapshot,
};

pub(super) fn decision_snapshot(
    game: &Game,
    viewer: PlayerId,
    pending: &PendingDecision,
) -> Option<DecisionStateSnapshot> {
    // A private decision is absent from this viewer's ordinary observation.
    // Serializing its continuation anyway would expose raw candidate ids and
    // effect-local bindings through the checkpoint, so fail reconstruction
    // closed for the non-choosing seat instead.
    if pending.observation.visibility == DecisionVisibility::Private
        && pending.observation.player != viewer
    {
        return None;
    }
    let card_origins = visible_decision_card_origins(game, viewer, pending);
    if decision_referenced_object_ids(&pending.continuation)
        .into_iter()
        .any(|object| {
            object_reference_requires_hidden_rebinding(game, viewer, object)
                && !card_origins
                    .iter()
                    .any(|origin| origin.object_id == object.0)
        })
    {
        return None;
    }
    let visible_rebindings = card_origins
        .iter()
        .map(|origin| GameObjectId(origin.object_id))
        .collect::<Vec<_>>();
    Some(DecisionStateSnapshot {
        preference: preference_snapshot(pending.observation.preference),
        card_origins,
        continuation: continuation_snapshot(
            game,
            viewer,
            &pending.continuation,
            &visible_rebindings,
        )?,
    })
}

fn visible_decision_card_origins(
    game: &Game,
    viewer: PlayerId,
    pending: &PendingDecision,
) -> Vec<DecisionCardOriginSnapshot> {
    if pending.observation.visibility != DecisionVisibility::Public
        && pending.observation.player != viewer
    {
        return Vec::new();
    }

    let mut origins = Vec::new();
    for object in pending
        .observation
        .options
        .iter()
        .flat_map(|option| option.card.iter().chain(option.members.iter()))
        .map(|(object, _)| *object)
    {
        if origins
            .iter()
            .any(|origin: &DecisionCardOriginSnapshot| origin.object_id == object.0)
        {
            continue;
        }
        if let Some((seat, zone, index)) = hidden_card_origin(game, object) {
            origins.push(DecisionCardOriginSnapshot {
                object_id: object.0,
                seat: seat.index(),
                zone,
                index,
            });
        }
    }
    origins
}

fn hidden_card_origin(
    game: &Game,
    object: GameObjectId,
) -> Option<(PlayerId, DecisionZoneSnapshot, usize)> {
    for seat in [PlayerId::One, PlayerId::Two] {
        let player = &game.players[seat.index()];
        for (zone, cards) in [
            (DecisionZoneSnapshot::Hand, &player.hand),
            (DecisionZoneSnapshot::Library, &player.library),
            (DecisionZoneSnapshot::OutsideGame, &player.outside_game),
        ] {
            if let Some(index) = cards.iter().position(|card| card.id == object) {
                return Some((seat, zone, index));
            }
        }
    }
    None
}

#[allow(clippy::too_many_lines)]
fn continuation_snapshot(
    game: &Game,
    viewer: PlayerId,
    continuation: &DecisionContinuation,
    visible_rebindings: &[GameObjectId],
) -> Option<DecisionContinuationSnapshot> {
    let value = match continuation {
        DecisionContinuation::BeginTurn {
            player,
            kind,
            applied,
            replacements,
            deferred,
        } => DecisionContinuationSnapshot::BeginTurn {
            player: player.index(),
            turn_kind: turn_kind_snapshot(*kind),
            applied: applied
                .iter()
                .copied()
                .map(ability_source_snapshot)
                .collect(),
            replacements: replacements
                .iter()
                .map(|replacement| begin_turn_replacement_snapshot(game, *replacement))
                .collect::<Option<Vec<_>>>()?,
            deferred: deferred
                .iter()
                .map(|effect| deferred_begin_turn_effect_snapshot(game, effect))
                .collect::<Option<Vec<_>>>()?,
        },
        DecisionContinuation::SearchZone {
            controller,
            source,
            destination,
            placement,
            reveal,
            shuffle,
            enters_tapped,
        } => DecisionContinuationSnapshot::SearchZone {
            controller: controller.index(),
            source: zone_kind_snapshot(*source),
            destination: zone_kind_snapshot(*destination),
            placement: zone_placement_snapshot(*placement),
            reveal: *reveal,
            shuffle: *shuffle,
            enters_tapped: *enters_tapped,
        },
        DecisionContinuation::ChooseCards {
            controller,
            destination,
            placement,
            reveal,
        } => DecisionContinuationSnapshot::ChooseCards {
            controller: controller.index(),
            destination: zone_kind_snapshot(*destination),
            placement: zone_placement_snapshot(*placement),
            reveal: *reveal,
        },
        DecisionContinuation::DrawReplacement {
            player,
            replacements,
        } => DecisionContinuationSnapshot::DrawReplacement {
            player: player.index(),
            replacements: replacements
                .iter()
                .map(|replacement| {
                    draw_replacement_snapshot_allowing(
                        game,
                        viewer,
                        replacement,
                        visible_rebindings,
                    )
                })
                .collect::<Option<Vec<_>>>()?,
        },
        DecisionContinuation::DiscardForEffect {
            player,
            amount,
            remaining,
            chosen,
            cause,
        } => DecisionContinuationSnapshot::DiscardForEffect {
            player: player.index(),
            amount: *amount,
            remaining: remaining.iter().copied().map(PlayerId::index).collect(),
            chosen: chosen
                .iter()
                .map(|(player, cards)| DiscardChoiceSnapshot {
                    player: player.index(),
                    cards: (*player == viewer).then(|| ids(cards)),
                    count: cards.len(),
                })
                .collect(),
            cause: cause_snapshot(*cause),
        },
        DecisionContinuation::BasicLandTypeTextChange { target } => {
            DecisionContinuationSnapshot::BasicLandTypeTextChange {
                target: target_snapshot(*target),
            }
        }
        DecisionContinuation::GrislySalvage { player, revealed } => {
            DecisionContinuationSnapshot::GrislySalvage {
                player: player.index(),
                revealed: revealed.iter().map(detached_card_snapshot).collect(),
            }
        }
        DecisionContinuation::AugurOfBolas { player, revealed } => {
            DecisionContinuationSnapshot::AugurOfBolas {
                player: player.index(),
                revealed: revealed.iter().map(detached_card_snapshot).collect(),
            }
        }
        DecisionContinuation::TopCardSelection {
            player,
            revealed,
            object,
            context,
            effect,
            ..
        } => DecisionContinuationSnapshot::TopCardSelection {
            player: player.index(),
            revealed: revealed.iter().map(detached_card_snapshot).collect(),
            continuation: effect_continuation_snapshot(
                game,
                viewer,
                object,
                context,
                *effect,
                visible_rebindings,
            )?,
        },
        DecisionContinuation::ChainLightning {
            player,
            spell,
            targets,
        } => DecisionContinuationSnapshot::ChainLightning {
            player: player.index(),
            spell: detached_stack_snapshot_allowing(game, viewer, spell, visible_rebindings)?,
            targets: targets.iter().copied().map(target_snapshot).collect(),
        },
        DecisionContinuation::Fork {
            player,
            spell,
            target_lists,
        } => DecisionContinuationSnapshot::Fork {
            player: player.index(),
            spell: detached_stack_snapshot_allowing(game, viewer, spell, visible_rebindings)?,
            target_lists: target_lists
                .iter()
                .map(|targets| targets.iter().map(target_selection_snapshot).collect())
                .collect(),
        },
        DecisionContinuation::OptionalEffect {
            object,
            context,
            effect,
        } => {
            let continuation = effect_continuation_snapshot(
                game,
                viewer,
                object,
                context,
                *effect,
                visible_rebindings,
            )?;
            DecisionContinuationSnapshot::OptionalEffect {
                object: continuation.object,
                ability: continuation.ability,
                context: continuation.context,
                effect: continuation.effect,
            }
        }
        DecisionContinuation::ChooseForEffect {
            definition,
            object,
            context,
            ..
        } => {
            if !matches!(definition.effect, EffectDef::Choose(_)) {
                return None;
            }
            DecisionContinuationSnapshot::ChooseForEffect {
                continuation: effect_continuation_snapshot(
                    game,
                    viewer,
                    object,
                    context,
                    *definition,
                    visible_rebindings,
                )?,
            }
        }
        DecisionContinuation::PayOr {
            player,
            payment,
            definition: scoped,
            object,
            context,
            ..
        } => {
            if trigger_capture_has_unrebindable_hidden_reference_except(
                game,
                viewer,
                &[],
                context,
                visible_rebindings,
            ) {
                return None;
            }
            let ability =
                stack_ability_snapshot_allowing(game, viewer, object, visible_rebindings)?
                    .ability_locator?;
            let definition = catalog_ability(&game.catalog, &ability)?;
            DecisionContinuationSnapshot::PayOr {
                player: player.index(),
                payment: resolved_effect_payment_snapshot(*payment),
                object: detached_stack_snapshot_allowing(game, viewer, object, visible_rebindings)?,
                ability,
                context: effect_resolution_context_snapshot(context),
                definition: scoped_effect_snapshot(&definition, *scoped)?,
            }
        }
        DecisionContinuation::SplitForEffect {
            definition,
            object,
            context,
            ..
        } => {
            if !matches!(definition.effect, EffectDef::SplitIntoPiles(_)) {
                return None;
            }
            DecisionContinuationSnapshot::SplitForEffect {
                continuation: effect_continuation_snapshot(
                    game,
                    viewer,
                    object,
                    context,
                    *definition,
                    visible_rebindings,
                )?,
            }
        }
        DecisionContinuation::ChoosePileForEffect {
            definition,
            first,
            second,
            object,
            context,
            ..
        } => {
            if !matches!(definition.effect, EffectDef::SplitIntoPiles(_)) {
                return None;
            }
            DecisionContinuationSnapshot::ChoosePileForEffect {
                first: first.iter().copied().map(target_snapshot).collect(),
                second: second.iter().copied().map(target_snapshot).collect(),
                continuation: effect_continuation_snapshot(
                    game,
                    viewer,
                    object,
                    context,
                    *definition,
                    visible_rebindings,
                )?,
            }
        }
        DecisionContinuation::BattlefieldEntryPayment {
            context,
            player,
            payment,
            definition,
        } => DecisionContinuationSnapshot::BattlefieldEntryPayment {
            context: replacement_context_snapshot(*context),
            player: player.index(),
            payment: resolved_effect_payment_snapshot(*payment),
            effect: resolved_replacement_effect_locator(
                &game.catalog,
                context.source,
                *definition,
            )?,
        },
        DecisionContinuation::BattlefieldEntryReplacement { candidates } => {
            DecisionContinuationSnapshot::BattlefieldEntryReplacement {
                candidates: candidates
                    .iter()
                    .map(|candidate| applicable_replacement_snapshot(&game.catalog, candidate))
                    .collect::<Option<Vec<_>>>()?,
            }
        }
        DecisionContinuation::BattlefieldEntryOptional { context, effect } => {
            DecisionContinuationSnapshot::BattlefieldEntryOptional {
                context: replacement_context_snapshot(*context),
                effect: resolved_replacement_effect_locator(
                    &game.catalog,
                    context.source,
                    *effect,
                )?,
            }
        }
        DecisionContinuation::BattlefieldEntryScalarChoice {
            context,
            choice,
            choices,
        } => DecisionContinuationSnapshot::BattlefieldEntryScalarChoice {
            context: replacement_context_snapshot(*context),
            effect: resolved_replacement_effect_locator(
                &game.catalog,
                context.source,
                ReplacementEffectDef::Choose(ReplacementChoiceDef::Scalar(*choice)),
            )?,
            choices: choices.clone(),
        },
        DecisionContinuation::BattlefieldEntryCopy {
            choices,
            added_types,
        } => DecisionContinuationSnapshot::BattlefieldEntryCopy {
            choices: ids(choices),
            added_types: CardType::ALL.map(|card_type| added_types.contains(card_type)),
        },
        DecisionContinuation::TriggerOrder { batch, remaining } => {
            DecisionContinuationSnapshot::TriggerOrder {
                batch: trigger_batch_snapshot(game, viewer, batch)?,
                remaining: remaining
                    .iter()
                    .map(|batch| trigger_batch_snapshot(game, viewer, batch))
                    .collect::<Option<Vec<_>>>()?,
            }
        }
        DecisionContinuation::TriggerPlacement {
            trigger,
            pending,
            remaining,
            candidates,
        } => DecisionContinuationSnapshot::TriggerPlacement {
            trigger: pending_trigger_snapshot(game, viewer, trigger)?,
            pending: pending
                .iter()
                .map(|trigger| pending_trigger_snapshot(game, viewer, trigger))
                .collect::<Option<Vec<_>>>()?,
            remaining: remaining
                .iter()
                .map(|batch| trigger_batch_snapshot(game, viewer, batch))
                .collect::<Option<Vec<_>>>()?,
            candidates: candidates.iter().copied().map(target_snapshot).collect(),
        },
        DecisionContinuation::MiracleReveal { card } => {
            DecisionContinuationSnapshot::MiracleReveal { card: card.0 }
        }
        DecisionContinuation::SeparateIntoPiles {
            resolving_controller,
            subject,
            items,
            on_complete,
        } => DecisionContinuationSnapshot::SeparateIntoPiles {
            resolving_controller: resolving_controller.index(),
            subject: subject.index(),
            items: items.iter().map(decision_option_snapshot).collect(),
            on_complete: on_complete.key().to_owned(),
        },
        DecisionContinuation::ChoosePile { piles, on_complete } => {
            DecisionContinuationSnapshot::ChoosePile {
                piles: pile_split_snapshot(piles),
                on_complete: on_complete.key().to_owned(),
            }
        }
        DecisionContinuation::ChooseColor {
            object,
            context,
            scoped,
            targets,
            ..
        } => DecisionContinuationSnapshot::ChooseColor {
            continuation: Box::new(effect_continuation_snapshot(
                game,
                viewer,
                object,
                context,
                *scoped,
                visible_rebindings,
            )?),
            targets: targets.iter().copied().map(target_snapshot).collect(),
        },
        DecisionContinuation::SacrificeOfChoice {
            followup,
            declined,
            optional,
        } => {
            DecisionContinuationSnapshot::SacrificeOfChoice {
                followup: match followup {
                    Some(followup) => {
                        let mut snapshot = effect_continuation_snapshot(
                            game,
                            viewer,
                            &followup.object,
                            &followup.context,
                            followup.effect,
                            visible_rebindings,
                        )?;
                        // The only continuation that reads a characteristic
                        // off what was sacrificed, so the only one that has
                        // to say which.
                        snapshot.reads_toughness =
                            followup.amount == crate::card::SacrificedAmountDef::Toughness;
                        Some(Box::new(snapshot))
                    }
                    None => None,
                },
                declined: match declined {
                    Some(declined) => Some(Box::new(effect_continuation_snapshot(
                        game,
                        viewer,
                        &declined.object,
                        &declined.context,
                        declined.effect,
                        visible_rebindings,
                    )?)),
                    None => None,
                },
                optional: *optional,
            }
        }
        DecisionContinuation::RecallDiscard { player } => {
            DecisionContinuationSnapshot::RecallDiscard {
                player: player.index(),
            }
        }
        DecisionContinuation::RecallReturn { player } => {
            DecisionContinuationSnapshot::RecallReturn {
                player: player.index(),
            }
        }
        DecisionContinuation::Balance {
            controller,
            phase,
            task,
            remaining,
        } => DecisionContinuationSnapshot::Balance {
            controller: controller.index(),
            phase: balance_phase_snapshot(*phase),
            task: balance_task_snapshot(viewer, task),
            remaining: remaining
                .iter()
                .map(|task| balance_task_snapshot(viewer, task))
                .collect(),
        },
        DecisionContinuation::SylvanOffer { player } => DecisionContinuationSnapshot::SylvanOffer {
            player: player.index(),
        },
        DecisionContinuation::SylvanSelect {
            player,
            candidates,
            choices_left,
        } => DecisionContinuationSnapshot::SylvanSelect {
            player: player.index(),
            candidates: ids(candidates),
            choices_left: *choices_left,
        },
        DecisionContinuation::SylvanMode {
            player,
            card,
            candidates,
            choices_left,
        } => DecisionContinuationSnapshot::SylvanMode {
            player: player.index(),
            card: card.0,
            candidates: ids(candidates),
            choices_left: *choices_left,
        },
        DecisionContinuation::TetravusDetach { source } => {
            DecisionContinuationSnapshot::TetravusDetach { source: source.0 }
        }
        DecisionContinuation::TetravusAssemble { source } => {
            DecisionContinuationSnapshot::TetravusAssemble { source: source.0 }
        }
        DecisionContinuation::BattlefieldExitReplacement { .. } => return None,
    };
    Some(value)
}

pub(super) fn parse_pending_decision(
    observation: &Value,
    state: Option<&DecisionStateSnapshot>,
    hidden: &Value,
    game: &Game,
) -> Result<Option<PendingDecision>, String> {
    let Some(visible) = observation.get("decision").filter(|value| !value.is_null()) else {
        if state.is_some() {
            return Err("checkpoint decision is not visible to its viewer".into());
        }
        return Ok(None);
    };
    let state = state.ok_or("decision continuation lacks a semantic checkpoint encoding")?;
    let observation = parse_decision_observation(visible, &state.preference)?;
    let continuation = parse_continuation(&state.continuation, &observation, hidden, game)?;
    Ok(Some(PendingDecision {
        observation,
        continuation,
    }))
}

fn parse_decision_observation(
    value: &Value,
    preference: &DecisionPreferenceSnapshot,
) -> Result<DecisionObservation, String> {
    Ok(DecisionObservation {
        id: u32_field(value, "id")?,
        player: seat_value(field(value, "seat")?)?,
        kind: match str_field(value, "kind")? {
            "Choice" => DecisionKind::Choice,
            "TriggerOrder" => DecisionKind::TriggerOrder,
            "TriggerPlacement" => DecisionKind::TriggerPlacement,
            other => return Err(format!("unknown decision kind {other}")),
        },
        order_semantics: value
            .get("orderSemantics")
            .filter(|value| !value.is_null())
            .map(|value| match value.as_str() {
                Some("resolution") => Ok(DecisionOrderSemantics::Resolution),
                _ => Err("unknown decision order semantics".to_owned()),
            })
            .transpose()?,
        prompt: str_field(value, "prompt")?.to_owned(),
        visibility: match str_field(value, "visibility")? {
            "Public" => DecisionVisibility::Public,
            "Private" => DecisionVisibility::Private,
            other => return Err(format!("unknown decision visibility {other}")),
        },
        preference: parse_preference(preference)?,
        minimum: usize_field(value, "minimum")?,
        maximum: usize_field(value, "maximum")?,
        cancellable: bool_field(value, "cancellable")?,
        options: array(field(value, "options")?)?
            .iter()
            .map(parse_option)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

include!("decision/continuation.rs");

include!("decision/validation.rs");

mod begin_turn;
mod support;

#[allow(clippy::wildcard_imports)]
use begin_turn::*;
pub(super) use support::decision_referenced_object_ids;
#[allow(clippy::wildcard_imports)]
use support::*;
pub(super) use support::{parse_pending_trigger, pending_trigger_snapshot};
