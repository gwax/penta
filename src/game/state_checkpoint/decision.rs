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
                .map(|effect| deferred_begin_turn_effect_snapshot(game, *effect))
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
        DecisionContinuation::SacrificeOfChoice { followup, optional } => {
            DecisionContinuationSnapshot::SacrificeOfChoice {
                followup: match followup {
                    Some(followup) => Some(effect_continuation_snapshot(
                        game,
                        viewer,
                        &followup.object,
                        &followup.context,
                        followup.effect,
                        visible_rebindings,
                    )?),
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

#[allow(clippy::too_many_lines)]
fn parse_continuation(
    value: &DecisionContinuationSnapshot,
    observation: &DecisionObservation,
    hidden: &Value,
    game: &Game,
) -> Result<DecisionContinuation, String> {
    Ok(match value {
        DecisionContinuationSnapshot::BeginTurn {
            player: prospective_player,
            turn_kind,
            applied,
            replacements,
            deferred,
        } => DecisionContinuation::BeginTurn {
            player: player(*prospective_player)?,
            kind: parse_turn_kind(*turn_kind),
            applied: applied.iter().copied().map(parse_ability_source).collect(),
            replacements: replacements
                .iter()
                .map(|replacement| parse_begin_turn_replacement(replacement, game))
                .collect::<Result<Vec<_>, _>>()?,
            deferred: deferred
                .iter()
                .map(|effect| parse_deferred_begin_turn_effect(effect, game))
                .collect::<Result<Vec<_>, _>>()?,
        },
        DecisionContinuationSnapshot::SearchZone {
            controller,
            source,
            destination,
            placement,
            reveal,
            shuffle,
            enters_tapped,
        } => DecisionContinuation::SearchZone {
            controller: player(*controller)?,
            source: parse_zone_kind(*source),
            destination: parse_zone_kind(*destination),
            placement: parse_zone_placement(*placement),
            reveal: *reveal,
            shuffle: *shuffle,
            enters_tapped: *enters_tapped,
        },
        DecisionContinuationSnapshot::ChooseCards {
            controller,
            destination,
            placement,
            reveal,
        } => DecisionContinuation::ChooseCards {
            controller: player(*controller)?,
            destination: parse_zone_kind(*destination),
            placement: parse_zone_placement(*placement),
            reveal: *reveal,
        },
        DecisionContinuationSnapshot::DrawReplacement {
            player: owner,
            replacements,
        } => DecisionContinuation::DrawReplacement {
            player: player(*owner)?,
            replacements: replacements
                .iter()
                .map(|replacement| parse_draw_replacement(replacement, game))
                .collect::<Result<Vec<_>, _>>()?,
        },
        DecisionContinuationSnapshot::DiscardForEffect {
            player: current,
            amount,
            remaining,
            chosen,
            cause,
        } => DecisionContinuation::DiscardForEffect {
            player: player(*current)?,
            amount: *amount,
            remaining: remaining
                .iter()
                .copied()
                .map(player)
                .collect::<Result<Vec<_>, _>>()?,
            chosen: chosen
                .iter()
                .map(|choice| {
                    let owner = player(choice.player)?;
                    let cards = match &choice.cards {
                        Some(cards) => game_ids(cards),
                        None => hidden_discard_choices(hidden, owner, choice.count, game)?,
                    };
                    Ok((owner, cards))
                })
                .collect::<Result<Vec<_>, String>>()?,
            cause: parse_cause(*cause)?,
        },
        DecisionContinuationSnapshot::BasicLandTypeTextChange { target } => {
            DecisionContinuation::BasicLandTypeTextChange {
                target: parse_target(*target),
            }
        }
        DecisionContinuationSnapshot::GrislySalvage {
            player: owner,
            revealed,
        } => DecisionContinuation::GrislySalvage {
            player: player(*owner)?,
            revealed: parse_detached_cards(revealed, game)?,
        },
        DecisionContinuationSnapshot::AugurOfBolas {
            player: owner,
            revealed,
        } => DecisionContinuation::AugurOfBolas {
            player: player(*owner)?,
            revealed: parse_detached_cards(revealed, game)?,
        },
        DecisionContinuationSnapshot::TopCardSelection {
            player: owner,
            revealed,
            continuation,
        } => {
            let owner = player(*owner)?;
            if owner != observation.player {
                return Err("top-card selection player disagrees with the visible decision".into());
            }
            let continuation = parse_effect_continuation(continuation, game)?;
            let EffectDef::LookAtTopAndSelect {
                player: recipient,
                selection,
            } = continuation.effect.effect
            else {
                return Err("top-card selection locator is not a top-card selection".into());
            };
            let players = game.effect_recipients(
                recipient,
                &continuation.object,
                &continuation.context,
                continuation.effect,
            );
            if players.as_slice() != [Target::Player(owner)] {
                return Err("top-card selection player disagrees with its authored effect".into());
            }
            let revealed = parse_detached_cards(revealed, game)?;
            validate_top_card_selection_observation(
                game,
                observation,
                owner,
                &revealed,
                selection,
                &continuation.object,
                &continuation.context,
                continuation.effect,
            )?;
            DecisionContinuation::TopCardSelection {
                player: owner,
                revealed,
                selection,
                object: continuation.object,
                context: continuation.context,
                effect: continuation.effect,
            }
        }
        DecisionContinuationSnapshot::ChainLightning {
            player: owner,
            spell,
            targets,
        } => DecisionContinuation::ChainLightning {
            player: player(*owner)?,
            spell: parse_detached_stack(spell, game)?,
            targets: targets.iter().copied().map(parse_target).collect(),
        },
        DecisionContinuationSnapshot::Fork {
            player: owner,
            spell,
            target_lists,
        } => DecisionContinuation::Fork {
            player: player(*owner)?,
            spell: parse_detached_stack(spell, game)?,
            target_lists: target_lists
                .iter()
                .map(|targets| {
                    targets
                        .iter()
                        .map(parse_target_selection)
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?,
        },
        DecisionContinuationSnapshot::OptionalEffect {
            object,
            ability,
            context,
            effect,
        } => DecisionContinuation::OptionalEffect {
            object: Box::new(parse_detached_stack(object, game)?),
            context: parse_effect_resolution_context(context.clone())?,
            effect: catalog_scoped_effect(&game.catalog, ability, effect)
                .ok_or("optional effect locator is absent from this catalog")?,
        },
        DecisionContinuationSnapshot::ChooseForEffect {
            continuation: snapshot,
        } => {
            let continuation = parse_effect_continuation(snapshot, game)?;
            if !ability_locator_matches_origin(&snapshot.ability, &continuation.object) {
                return Err("object-choice locator disagrees with its resolving ability".into());
            }
            let EffectDef::Choose(definition) = continuation.effect.effect else {
                return Err("object-choice locator does not identify an authored choice".into());
            };
            let state = game
                .effect_choice_decision_state(
                    definition,
                    &continuation.object,
                    &continuation.context,
                    continuation.effect,
                )
                .ok_or("object-choice authored chooser is not singular")?;
            if definition.minimum > 0 && state.candidates.len() <= definition.minimum {
                return Err(
                    "object-choice checkpoint encodes a choice that would resolve automatically"
                        .into(),
                );
            }
            validate_authored_decision(
                observation,
                state.chooser,
                "Choose objects",
                effect_choice_visibility(definition.visibility),
                state.preference,
                state.minimum,
                state.maximum,
                &state.options,
                "object choice",
            )?;
            DecisionContinuation::ChooseForEffect {
                definition: continuation.effect,
                binding: definition.binding,
                object: continuation.object,
                context: continuation.context,
                candidates: state.candidates,
                effect: continuation.effect.with_effect(*definition.then),
            }
        }
        DecisionContinuationSnapshot::PayOr {
            player: payer,
            payment,
            object,
            ability,
            context,
            definition,
        } => {
            let payer = player(*payer)?;
            if payer != observation.player {
                return Err("pay-or payer disagrees with the visible decision".into());
            }
            let payment = parse_resolved_effect_payment(payment)?;
            let object = Box::new(parse_detached_stack(object, game)?);
            let context = parse_effect_resolution_context(context.clone())?;
            if !ability_locator_matches_origin(ability, &object) {
                return Err("pay-or locator disagrees with its resolving ability".into());
            }
            let scoped = catalog_scoped_effect(&game.catalog, ability, definition)
                .ok_or("pay-or locator is absent from this catalog")?;
            let EffectDef::PayOr(authored) = scoped.effect else {
                return Err("pay-or locator does not identify an optional payment".into());
            };
            let expected =
                resolved_effect_payment(game, authored.payment, &object, &context, scoped)
                    .ok_or("pay-or authored payment no longer has exactly one payer")?;
            if expected != (payer, payment) {
                return Err("pay-or payer or payment disagrees with its authored effect".into());
            }
            let can_pay = game.can_pay_effect_payment(payer, payment);
            if authored.if_paid.is_none() && authored.otherwise.is_none()
                || (!can_pay && authored.otherwise.is_some())
            {
                return Err(
                    "pay-or checkpoint encodes a choice that would resolve automatically".into(),
                );
            }
            let options = payment_decision_options(payment, can_pay, "Decline");
            validate_authored_decision(
                observation,
                payer,
                object.ability_text().unwrap_or("Pay the cost?"),
                effect_choice_visibility(authored.visibility),
                DecisionPreference::Neutral,
                1,
                1,
                &options,
                "pay-or",
            )?;
            DecisionContinuation::PayOr {
                player: payer,
                payment,
                definition: scoped,
                object,
                context,
                if_paid: authored.if_paid.map(|effect| scoped.with_effect(*effect)),
                otherwise: authored.otherwise.map(|effect| scoped.with_effect(*effect)),
            }
        }
        DecisionContinuationSnapshot::SplitForEffect {
            continuation: snapshot,
        } => {
            let continuation = parse_effect_continuation(snapshot, game)?;
            if !ability_locator_matches_origin(&snapshot.ability, &continuation.object) {
                return Err("pile-split locator disagrees with its resolving ability".into());
            }
            let EffectDef::SplitIntoPiles(definition) = continuation.effect.effect else {
                return Err("pile-split locator does not identify an authored partition".into());
            };
            let state = game
                .effect_pile_split_state(
                    definition,
                    &continuation.object,
                    &continuation.context,
                    continuation.effect,
                )
                .ok_or("pile-split authored divider or chooser is not singular")?;
            validate_authored_decision(
                observation,
                state.divider,
                "Separate the objects into two piles",
                DecisionVisibility::Public,
                DecisionPreference::BalancedPartition,
                0,
                state.items.len(),
                &state.options,
                "pile split",
            )?;
            DecisionContinuation::SplitForEffect {
                definition: continuation.effect,
                chooser: state.chooser,
                items: state.items,
                object: continuation.object,
                context: continuation.context,
            }
        }
        DecisionContinuationSnapshot::ChoosePileForEffect {
            first,
            second,
            continuation: snapshot,
        } => {
            let continuation = parse_effect_continuation(snapshot, game)?;
            if !ability_locator_matches_origin(&snapshot.ability, &continuation.object) {
                return Err("pile-choice locator disagrees with its resolving ability".into());
            }
            let EffectDef::SplitIntoPiles(definition) = continuation.effect.effect else {
                return Err("pile-choice locator does not identify an authored partition".into());
            };
            let authored = game
                .effect_pile_split_state(
                    definition,
                    &continuation.object,
                    &continuation.context,
                    continuation.effect,
                )
                .ok_or("pile-choice authored divider or chooser is not singular")?;
            let first = first.iter().copied().map(parse_target).collect::<Vec<_>>();
            let second = second.iter().copied().map(parse_target).collect::<Vec<_>>();
            validate_exact_partition(&authored.items, &first, &second)?;
            let state =
                game.effect_pile_choice_state(&first, &second, definition, continuation.effect);
            validate_authored_decision(
                observation,
                authored.chooser,
                "Choose a pile",
                DecisionVisibility::Public,
                state.preference,
                1,
                1,
                &state.options,
                "pile choice",
            )?;
            DecisionContinuation::ChoosePileForEffect {
                definition: continuation.effect,
                first,
                second,
                chosen: definition.chosen,
                unchosen: definition.unchosen,
                object: continuation.object,
                context: continuation.context,
                effect: continuation.effect.with_effect(*definition.then),
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryPayment {
            context,
            player: payer,
            payment,
            effect,
        } => {
            let context = parse_replacement_context(*context)?;
            validate_entry_decision_context(game, context, effect)?;
            let definition = catalog_replacement_effect(&game.catalog, effect)
                .ok_or("battlefield entry payment locator is absent from this catalog")?;
            let ReplacementEffectDef::PayOr { .. } = definition else {
                return Err("battlefield entry payment locator is not an optional payment".into());
            };
            let payer = player(*payer)?;
            let payment = parse_resolved_effect_payment(payment)?;
            let pending = game
                .pending_events
                .front()
                .ok_or("battlefield entry payment lacks its pending event")?;
            if payer != observation.player
                || game.pending_resolved_payment(
                    pending,
                    context,
                    match definition {
                        ReplacementEffectDef::PayOr { payment, .. } => payment,
                        _ => unreachable!(),
                    },
                ) != Some((payer, payment))
            {
                return Err(
                    "battlefield entry payer or payment disagrees with its authored effect".into(),
                );
            }
            if !game.can_pay_effect_payment(payer, payment) {
                return Err("battlefield entry payment is no longer payable".into());
            }
            let name = game.pending_entry_name(pending);
            let payment_label = Game::effect_payment_label(payment);
            let options = payment_decision_options(payment, true, "Do not pay");
            validate_authored_decision(
                observation,
                payer,
                &format!("{payment_label} as {name} enters the battlefield?"),
                DecisionVisibility::Public,
                DecisionPreference::Neutral,
                1,
                1,
                &options,
                "battlefield entry payment",
            )?;
            DecisionContinuation::BattlefieldEntryPayment {
                context,
                player: payer,
                payment,
                definition,
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryReplacement { candidates } => {
            DecisionContinuation::BattlefieldEntryReplacement {
                candidates: candidates
                    .iter()
                    .map(|candidate| parse_applicable_replacement(candidate, &game.catalog))
                    .collect::<Result<Vec<_>, _>>()?,
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryOptional { context, effect } => {
            let context = parse_replacement_context(*context)?;
            validate_entry_decision_context(game, context, effect)?;
            let definition = catalog_replacement_effect(&game.catalog, effect)
                .ok_or("optional entry replacement locator is absent from this catalog")?;
            let pending = game
                .pending_events
                .front()
                .ok_or("optional entry replacement lacks its pending event")?;
            let mut before_selection = pending.clone();
            before_selection
                .applied
                .retain(|source| *source != context.source);
            let candidate = game
                .applicable_replacements(&before_selection)
                .into_iter()
                .find(|candidate| candidate.context == context && candidate.effect == definition)
                .ok_or("optional entry replacement is not applicable to its pending event")?;
            if !candidate.optional {
                return Err("optional entry replacement locator names a mandatory ability".into());
            }
            let owner = Game::pending_event_controller(pending);
            let name = game.pending_entry_name(pending);
            validate_authored_decision(
                observation,
                owner,
                &format!("Apply the optional replacement for {name}?"),
                DecisionVisibility::Public,
                DecisionPreference::Neutral,
                1,
                1,
                &Game::optional_entry_replacement_options(),
                "optional entry replacement",
            )?;
            DecisionContinuation::BattlefieldEntryOptional {
                context,
                effect: definition,
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryScalarChoice {
            context,
            effect,
            choices,
        } => {
            let context = parse_replacement_context(*context)?;
            validate_entry_decision_context(game, context, effect)?;
            let ReplacementEffectDef::Choose(ReplacementChoiceDef::Scalar(choice)) =
                catalog_replacement_effect(&game.catalog, effect)
                    .ok_or("entry scalar choice locator is absent from this catalog")?
            else {
                return Err("entry scalar choice locator is not a scalar choice".into());
            };
            let pending = game
                .pending_events
                .front()
                .ok_or("entry scalar choice lacks its pending event")?;
            let owner = Game::pending_event_controller(pending);
            let (prompt, authored_choices) = game.entry_scalar_choices(owner, choice);
            if *choices != authored_choices {
                return Err(
                    "entry scalar choice vocabulary disagrees with its authored choice".into(),
                );
            }
            let options = authored_choices
                .iter()
                .enumerate()
                .map(|(index, label)| DecisionOption {
                    id: u32::try_from(index).unwrap_or(u32::MAX),
                    label: label.clone(),
                    card: None,
                    members: Vec::new(),
                    ability_text: None,
                    zone: DecisionZone::None,
                })
                .collect::<Vec<_>>();
            validate_authored_decision(
                observation,
                owner,
                prompt,
                DecisionVisibility::Public,
                DecisionPreference::Neutral,
                1,
                1,
                &options,
                "entry scalar choice",
            )?;
            DecisionContinuation::BattlefieldEntryScalarChoice {
                context,
                choice,
                choices: choices.clone(),
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryCopy {
            choices,
            added_types,
        } => DecisionContinuation::BattlefieldEntryCopy {
            choices: game_ids(choices),
            added_types: parse_card_type_set(*added_types),
        },
        DecisionContinuationSnapshot::TriggerOrder { batch, remaining } => {
            DecisionContinuation::TriggerOrder {
                batch: parse_trigger_batch(batch, game)?,
                remaining: remaining
                    .iter()
                    .map(|batch| parse_trigger_batch(batch, game))
                    .collect::<Result<Vec<_>, _>>()?,
            }
        }
        DecisionContinuationSnapshot::TriggerPlacement {
            trigger,
            pending,
            remaining,
            candidates,
        } => DecisionContinuation::TriggerPlacement {
            trigger: parse_pending_trigger(trigger, game)?,
            pending: pending
                .iter()
                .map(|trigger| parse_pending_trigger(trigger, game))
                .collect::<Result<Vec<_>, _>>()?,
            remaining: remaining
                .iter()
                .map(|batch| parse_trigger_batch(batch, game))
                .collect::<Result<Vec<_>, _>>()?,
            candidates: candidates.iter().copied().map(parse_target).collect(),
        },
        DecisionContinuationSnapshot::MiracleReveal { card } => {
            DecisionContinuation::MiracleReveal {
                card: GameObjectId(*card),
            }
        }
        DecisionContinuationSnapshot::SeparateIntoPiles {
            resolving_controller,
            subject,
            items,
            on_complete,
        } => DecisionContinuation::SeparateIntoPiles {
            resolving_controller: player(*resolving_controller)?,
            subject: player(*subject)?,
            items: items.iter().map(parse_decision_option_snapshot).collect(),
            on_complete: crate::card::sets::piles_separated_resolver(on_complete)
                .ok_or("unknown piles-separated resolver")?,
        },
        DecisionContinuationSnapshot::ChoosePile { piles, on_complete } => {
            DecisionContinuation::ChoosePile {
                piles: parse_pile_split_snapshot(piles)?,
                on_complete: crate::card::sets::pile_chosen_resolver(on_complete)
                    .ok_or("unknown pile-chosen resolver")?,
            }
        }
        DecisionContinuationSnapshot::SacrificeOfChoice { followup, optional } => {
            DecisionContinuation::SacrificeOfChoice {
                followup: followup
                    .as_ref()
                    .map(|followup| parse_effect_continuation(followup, game))
                    .transpose()?,
                optional: *optional,
            }
        }
        DecisionContinuationSnapshot::RecallDiscard { player: owner } => {
            DecisionContinuation::RecallDiscard {
                player: player(*owner)?,
            }
        }
        DecisionContinuationSnapshot::RecallReturn { player: owner } => {
            DecisionContinuation::RecallReturn {
                player: player(*owner)?,
            }
        }
        DecisionContinuationSnapshot::Balance {
            controller,
            phase,
            task,
            remaining,
        } => DecisionContinuation::Balance {
            controller: player(*controller)?,
            phase: parse_balance_phase(*phase),
            task: parse_balance_task(task, game)?,
            remaining: remaining
                .iter()
                .map(|task| parse_balance_task(task, game))
                .collect::<Result<Vec<_>, _>>()?,
        },
        DecisionContinuationSnapshot::SylvanOffer { player: owner } => {
            DecisionContinuation::SylvanOffer {
                player: player(*owner)?,
            }
        }
        DecisionContinuationSnapshot::SylvanSelect {
            player: owner,
            candidates,
            choices_left,
        } => DecisionContinuation::SylvanSelect {
            player: player(*owner)?,
            candidates: game_ids(candidates),
            choices_left: *choices_left,
        },
        DecisionContinuationSnapshot::SylvanMode {
            player: owner,
            card,
            candidates,
            choices_left,
        } => DecisionContinuation::SylvanMode {
            player: player(*owner)?,
            card: GameObjectId(*card),
            candidates: game_ids(candidates),
            choices_left: *choices_left,
        },
        DecisionContinuationSnapshot::TetravusDetach { source } => {
            DecisionContinuation::TetravusDetach {
                source: GameObjectId(*source),
            }
        }
        DecisionContinuationSnapshot::TetravusAssemble { source } => {
            DecisionContinuation::TetravusAssemble {
                source: GameObjectId(*source),
            }
        }
    })
}

fn resolved_effect_payment(
    game: &Game,
    payment: EffectPaymentDef,
    object: &super::super::StackObject,
    context: &super::super::EffectResolutionContext,
    scoped: ScopedEffect,
) -> Option<(PlayerId, super::super::ResolvedEffectPayment)> {
    let payers = game.effect_players(payment.payer, object, context, scoped);
    let [player] = payers.as_slice() else {
        return None;
    };
    let payment = match payment.cost {
        EffectPaymentCostDef::Mana(cost) => super::super::ResolvedEffectPayment::Mana(cost),
        EffectPaymentCostDef::GenericMana(amount) => {
            let amount = game
                .effect_value(amount, object, context, scoped)
                .max(0)
                .try_into()
                .unwrap_or(u16::MAX);
            super::super::ResolvedEffectPayment::Mana(ManaCost::new(amount, 0))
        }
        EffectPaymentCostDef::Life(amount) => super::super::ResolvedEffectPayment::Life(amount),
    };
    Some((*player, payment))
}

fn payment_decision_options(
    payment: super::super::ResolvedEffectPayment,
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
    if can_pay {
        options.push(DecisionOption {
            id: 1,
            label: Game::effect_payment_label(payment),
            card: None,
            members: Vec::new(),
            ability_text: None,
            zone: DecisionZone::None,
        });
    }
    options
}

#[allow(clippy::too_many_arguments)]
fn validate_top_card_selection_observation(
    game: &Game,
    observation: &DecisionObservation,
    player: PlayerId,
    revealed: &[super::super::CardInstance],
    selection: &'static crate::card::TopCardSelectionDef,
    object: &super::super::StackObject,
    context: &super::super::EffectResolutionContext,
    scoped: ScopedEffect,
) -> Result<(), String> {
    let requested = game
        .effect_value(selection.count, object, context, scoped)
        .max(0);
    let requested = usize::try_from(requested).unwrap_or(usize::MAX);
    let available_before_inspection = game.players[player.index()]
        .library
        .len()
        .saturating_add(revealed.len());
    if revealed.len() != requested.min(available_before_inspection) {
        return Err("top-card selection inspected count disagrees with its authored effect".into());
    }
    let source = object.source.unwrap_or(object.id);
    let eligible = revealed
        .iter()
        .filter(|card| {
            selection.object.is_none_or(|predicate| {
                game.card_object_matches(predicate, card, crate::card::ZoneKind::Library, source)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let inspected = revealed
        .iter()
        .map(|card| (card.id, card.definition))
        .collect::<Vec<_>>();
    let mut expected = game.card_decision_options(&eligible, DecisionZone::Library);
    for option in &mut expected {
        option.members = inspected.clone();
    }
    let no_selection = expected.is_empty();
    if no_selection {
        expected.push(DecisionOption {
            id: 0,
            label: "No inspected card is eligible".into(),
            card: None,
            members: inspected,
            ability_text: None,
            zone: DecisionZone::Library,
        });
    }
    let (minimum, maximum, preference) = if no_selection {
        (0, 0, DecisionPreference::Neutral)
    } else {
        (
            usize::from(selection.minimum).min(expected.len()),
            usize::from(selection.maximum),
            if selection.selected_zone == crate::card::ZoneKind::Hand {
                DecisionPreference::HigherCardValue
            } else {
                DecisionPreference::LowerCardValue
            },
        )
    };
    validate_authored_decision(
        observation,
        player,
        "Choose cards from the top of the library",
        DecisionVisibility::Private,
        preference,
        minimum,
        maximum,
        &expected,
        "top-card selection",
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_authored_decision(
    observation: &DecisionObservation,
    player: PlayerId,
    prompt: &str,
    visibility: DecisionVisibility,
    preference: DecisionPreference,
    minimum: usize,
    maximum: usize,
    options: &[DecisionOption],
    description: &str,
) -> Result<(), String> {
    let expected_minimum = minimum.min(options.len());
    let expected_maximum = maximum.max(expected_minimum);
    let mismatch = if observation.player != player {
        "player"
    } else if observation.kind != DecisionKind::Choice || observation.order_semantics.is_some() {
        "kind"
    } else if observation.prompt != prompt {
        "prompt"
    } else if observation.visibility != visibility {
        "visibility"
    } else if observation.preference != preference {
        "preference"
    } else if observation.minimum != expected_minimum || observation.maximum != expected_maximum {
        "bounds"
    } else if observation.cancellable {
        "cancellability"
    } else if observation.options != options {
        return Err(format!(
            "{description} decision options disagree with its authored effect: observed {:?}, expected {options:?}",
            observation.options,
        ));
    } else {
        return Ok(());
    };
    Err(format!(
        "{description} decision {mismatch} disagrees with its authored effect"
    ))
}

fn validate_exact_partition(
    authored: &[Target],
    first: &[Target],
    second: &[Target],
) -> Result<(), String> {
    let combined = first.iter().chain(second).copied().collect::<Vec<_>>();
    if combined.len() != authored.len()
        || combined
            .iter()
            .enumerate()
            .any(|(index, item)| combined[..index].contains(item))
        || combined.iter().any(|item| !authored.contains(item))
        || authored.iter().any(|item| !combined.contains(item))
    {
        return Err(
            "pile-choice checkpoint is not an exact disjoint partition of authored items".into(),
        );
    }
    let canonical_first = authored
        .iter()
        .filter(|item| first.contains(item))
        .copied()
        .collect::<Vec<_>>();
    let canonical_second = authored
        .iter()
        .filter(|item| second.contains(item))
        .copied()
        .collect::<Vec<_>>();
    if canonical_first != first || canonical_second != second {
        return Err("pile-choice checkpoint changed the authored item order".into());
    }
    Ok(())
}

fn ability_locator_matches_origin(
    locator: &AbilityLocator,
    object: &super::super::StackObject,
) -> bool {
    let Some(payload) = &object.ability else {
        return false;
    };
    let expected = match payload.origin {
        super::super::AbilityOrigin::Printed {
            definition,
            part,
            ability,
        } => (definition.0, part.0, ability.0),
        super::super::AbilityOrigin::Granted {
            source_definition,
            source_part,
            source_ability,
            ..
        } => (source_definition.0, source_part.0, source_ability.0),
        super::super::AbilityOrigin::IntrinsicBasicLand(_) => return false,
    };
    (locator.definition, locator.part_id, locator.ability_id) == expected
}

fn validate_entry_decision_context(
    game: &Game,
    context: ReplacementEffectContext,
    locator: &ReplacementEffectLocator,
) -> Result<(), String> {
    if !replacement_effect_locator_matches_source(locator, context.source) {
        return Err("entry decision locator disagrees with its replacement source".into());
    }
    let pending = game
        .pending_events
        .front()
        .ok_or("entry decision lacks its pending event")?;
    if !pending.applied.contains(&context.source)
        || Game::pending_event_controller(pending) != context.controller
    {
        return Err("entry decision context disagrees with its pending event".into());
    }
    Ok(())
}

mod begin_turn;
mod support;

#[allow(clippy::wildcard_imports)]
use begin_turn::*;
pub(super) use support::decision_referenced_object_ids;
#[allow(clippy::wildcard_imports)]
use support::*;
pub(super) use support::{parse_pending_trigger, pending_trigger_snapshot};
