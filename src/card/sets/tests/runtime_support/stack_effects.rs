use super::*;
use crate::CostDef;
use crate::card::{
    ChooseDef, EffectPaymentDef, ObjectChoiceBindingDef, PartitionItemsDef, SplitIntoPilesDef,
};

pub(in super::super) fn shared_stack_effect(effect: EffectDef) -> bool {
    shared_stack_effect_at_position(effect, true)
}

fn shared_effect_payment(payment: EffectPaymentDef) -> bool {
    match payment {
        EffectPaymentDef::Costs(payment) => {
            !matches!(
                payment.payer,
                PlayerRelation::Any | PlayerRelation::ChosenPlayer | PlayerRelation::EventPlayer
            ) && matches!(payment.costs, [CostDef::Mana(_)])
        }
        EffectPaymentDef::Mana { payer, .. } | EffectPaymentDef::GenericMana { payer, .. } => {
            shared_effect_recipient(EffectRecipientDef::player(payer))
        }
    }
}

fn shared_choose(choice: ChooseDef) -> bool {
    choice.maximum > 0
        && choice.minimum <= choice.maximum
        && match choice.binding {
            ObjectChoiceBindingDef::Object(_) => choice.maximum == 1,
            ObjectChoiceBindingDef::Objects(_) => true,
        }
        && shared_effect_recipient(EffectRecipientDef::player(choice.chooser))
        && shared_effect_recipient(EffectRecipientDef::objects(choice.candidates))
        && choice
            .exclude
            .is_none_or(|object| shared_effect_recipient(EffectRecipientDef::object(object)))
}

fn shared_partition(partition: SplitIntoPilesDef) -> bool {
    let items_are_shared = match partition.items {
        PartitionItemsDef::Objects(objects) => {
            shared_effect_recipient(EffectRecipientDef::objects(objects))
        }
        PartitionItemsDef::TopOfLibrary { player, .. } => {
            shared_effect_recipient(EffectRecipientDef::player(player))
        }
    };
    items_are_shared
        && !matches!(
            partition.divider,
            PlayerSetDef::All | PlayerSetDef::Related(PlayerRelation::Any)
        )
        && !matches!(
            partition.chooser,
            PlayerSetDef::All | PlayerSetDef::Related(PlayerRelation::Any)
        )
        && shared_effect_recipient(EffectRecipientDef::players(partition.divider))
        && shared_effect_recipient(EffectRecipientDef::players(partition.chooser))
}

/// Resolving sequences preserve their unprocessed tail, so a queued decision
/// may suspend at any sequence component. Other callers still pass false when
/// their own continuation cannot be suspended.
/// The effects whose whole procedure is a decision the shared runtime
/// asks for. Their callers have already established that a deferred
/// decision is allowed where they sit; this checks only their arguments.
fn shared_decision_effect(effect: EffectDef) -> bool {
    match effect {
        // Looking is private and the offer is the only visible part, and
        // the chosen card comes from the named player's own library.
        EffectDef::LookAtTopAndMayTake { player, object } => {
            shared_effect_recipient(player) && shared_object_predicate(object)
        }
        EffectDef::LookAtTopAndSelect { player, selection } => {
            let supported_zone = |zone| {
                matches!(
                    zone,
                    ZoneKind::Hand | ZoneKind::Library | ZoneKind::Graveyard | ZoneKind::Exile
                )
            };
            shared_effect_recipient(player)
                && selection.minimum <= selection.maximum
                && supported_zone(selection.selected_zone)
                && supported_zone(selection.rest_zone)
                && selection
                    .then
                    .is_none_or(|effect| shared_stack_effect_at_position(*effect, true))
        }
        _ => false,
    }
}

/// The chooser is a player and the choices are their own battlefield, so
/// only the predicate needs checking. The follow-up runs inside the
/// sacrifice's continuation, which can establish a fresh decision.
fn shared_sacrifice_of_choice(effect: EffectDef) -> bool {
    let EffectDef::SacrificeOfChoice {
        player,
        object,
        then,
        ..
    } = effect
    else {
        return false;
    };
    shared_effect_recipient(player)
        && shared_object_predicate(object)
        && then.is_none_or(|effect| shared_stack_effect_at_position(*effect, true))
}

#[allow(clippy::too_many_lines)]
fn shared_stack_effect_at_position(effect: EffectDef, deferred_decision_allowed: bool) -> bool {
    match effect {
        EffectDef::Sequence(effects) => {
            !effects.is_empty()
                && effects.iter().copied().all(|effect| {
                    shared_stack_effect_at_position(effect, deferred_decision_allowed)
                })
        }
        EffectDef::Randomized {
            on_success,
            on_failure,
            ..
        } => {
            let branch_is_shared = |branch: EffectDef| {
                branch == EffectDef::None
                    || shared_stack_effect_at_position(branch, deferred_decision_allowed)
            };
            branch_is_shared(*on_success) && branch_is_shared(*on_failure)
        }
        EffectDef::PreventNextDamageFromSource { object, source, .. } => {
            shared_effect_recipient(object) && shared_effect_recipient(source)
        }
        EffectDef::PreventDamageToPlayerAndControlledCreaturesThisTurn { player }
        | EffectDef::PreventDamageToPlayerFromThisTurn { player, .. }
        | EffectDef::RedirectTargetDamageToSourceThisTurn { player, .. } => {
            shared_effect_recipient(player)
        }
        EffectDef::PreventAllCombatDamageExceptSourceThisTurn { source } => {
            shared_effect_recipient(source)
        }
        EffectDef::Choose(choice) => {
            deferred_decision_allowed
                && shared_choose(choice)
                && shared_stack_effect_at_position(*choice.then, true)
        }
        EffectDef::PayOr(payment) => {
            deferred_decision_allowed
                && shared_effect_payment(payment.payment)
                && (payment.if_paid.is_some() || payment.otherwise.is_some())
                && payment
                    .if_paid
                    .iter()
                    .chain(payment.otherwise.iter())
                    .all(|effect| shared_stack_effect_at_position(**effect, true))
        }
        EffectDef::SplitIntoPiles(partition) => {
            deferred_decision_allowed
                && shared_partition(partition)
                && shared_stack_effect_at_position(*partition.then, true)
        }
        EffectDef::AddMana(_) => shared_mana_effect(effect, false),
        EffectDef::DealDamage { recipient, .. }
        | EffectDef::DrainLife { recipient, .. }
        | EffectDef::GainLife { recipient, .. }
        | EffectDef::AddPoisonCounters { recipient, .. }
        | EffectDef::DrawCards { recipient, .. }
        | EffectDef::Discard { recipient, .. }
        | EffectDef::ShuffleLibrary { player: recipient }
        | EffectDef::EmptyManaPool { player: recipient }
        | EffectDef::TakeExtraTurn { player: recipient }
        | EffectDef::LoseLife { recipient, .. }
        | EffectDef::Mill {
            player: recipient, ..
        }
        | EffectDef::CannotCastNoncreatureSpellsThisTurn { player: recipient }
        | EffectDef::LoseTheGame { player: recipient }
        | EffectDef::LookAtHand { player: recipient } => shared_effect_recipient(recipient),
        EffectDef::SacrificeOfChoice { .. } => shared_sacrifice_of_choice(effect),
        EffectDef::LookAtTopAndMayTake { .. } | EffectDef::LookAtTopAndSelect { .. } => {
            deferred_decision_allowed && shared_decision_effect(effect)
        }
        EffectDef::SearchZone {
            player,
            source,
            object,
            minimum,
            maximum,
            destination,
            shuffle,
            ..
        } => {
            deferred_decision_allowed
                && shared_effect_recipient(player)
                && shared_object_predicate(object)
                && minimum <= maximum
                && (destination != ZoneKind::Library || maximum <= 1)
                && (destination != ZoneKind::Battlefield || maximum <= 1)
                && (!shuffle || source == ZoneKind::Library)
                && matches!(
                    source,
                    ZoneKind::Library | ZoneKind::Hand | ZoneKind::Graveyard | ZoneKind::Exile
                )
                && matches!(
                    destination,
                    ZoneKind::Library
                        | ZoneKind::Hand
                        | ZoneKind::Battlefield
                        | ZoneKind::Graveyard
                        | ZoneKind::Exile
                )
        }
        EffectDef::ChooseCards {
            player,
            sources,
            object,
            minimum,
            maximum,
            destination,
            ..
        } => {
            deferred_decision_allowed
                && shared_effect_recipient(player)
                && shared_object_predicate(object)
                && minimum <= maximum
                && !sources.is_empty()
                && sources.iter().all(|source| {
                    matches!(
                        source,
                        CardChoiceSourceDef::OutsideGame
                            | CardChoiceSourceDef::Zone(ZoneKind::Exile)
                    )
                })
                && destination == ZoneKind::Hand
        }
        EffectDef::ReplaceNextDrawThisTurn { player, effect } => {
            shared_effect_recipient(player) && shared_stack_effect_at_position(*effect, true)
        }
        EffectDef::IfFormat {
            then, otherwise, ..
        } => {
            shared_stack_effect_at_position(*then, deferred_decision_allowed)
                && shared_stack_effect_at_position(*otherwise, deferred_decision_allowed)
        }
        // Only the two destinations the return path knows.
        EffectDef::ReturnLinkedExiles { zone, .. } => {
            matches!(zone, ZoneKind::Battlefield | ZoneKind::Hand)
        }
        // Populate copies whatever the choice landed on, so like the rest of
        // these only its recipient has to be one the runtime understands.
        EffectDef::CreateTokenCopyOf { object }
        | EffectDef::Regenerate { object }
        | EffectDef::Tap { object }
        | EffectDef::RemoveFromCombat { object }
        | EffectDef::SetColor { object, .. }
        | EffectDef::DestroyAtEndOfCombat { object, .. }
        | EffectDef::SkipNextUntapSteps { object, .. }
        | EffectDef::DoesNotUntapWhileSourceTapped { object }
        | EffectDef::RemoveAllCounters { object, .. }
        | EffectDef::Untap { object }
        | EffectDef::PreventNextDamage { object, .. }
        | EffectDef::PreventAllDamageThisTurn { object }
        | EffectDef::PreventCombatDamageThisTurn { object }
        | EffectDef::PreventCombatDamageDealtByThisTurn { object }
        | EffectDef::PreventDamageDealtByThisTurn { object }
        | EffectDef::Destroy { object, .. }
        | EffectDef::Sacrifice { object }
        | EffectDef::ExileLinkedToSource { object }
        | EffectDef::Detain { object }
        | EffectDef::CannotRegenerateThisTurn { object }
        | EffectDef::MakeUnblockableThisTurn { object }
        | EffectDef::GainControlWhileSourceRemains { object, .. }
        | EffectDef::GainControlThisTurn { object }
        | EffectDef::AddCounters { object, .. }
        | EffectDef::Attach { object }
        | EffectDef::ChangeTextBasicLandType { object }
        | EffectDef::BecomeCopyOf { object, .. } => shared_effect_recipient(object),
        EffectDef::Counter { object, zone } => {
            matches!(zone, ZoneKind::Graveyard | ZoneKind::Exile) && shared_effect_recipient(object)
        }
        // Neither needs a recipient: both concern the resolving controller.
        // The amount is computed when the effect resolves, so nothing has
        // to read it ahead of time the way a mana ability does.
        EffectDef::AddManaEqualTo { .. }
        | EffectDef::CreateToken { .. }
        | EffectDef::CreateEmblem { .. }
        | EffectDef::Transform { .. }
        | EffectDef::AdditionalCombatPhase
        | EffectDef::PreventAllCombatDamageThisTurn
        | EffectDef::GrantFlashToNextSorcery => true,
        // Each of these asks a question and then runs an inner effect,
        // so the question has to be allowed here and the answer has to be
        // something the shared procedure can carry out.
        EffectDef::May { player, effect } => {
            deferred_decision_allowed
                && shared_effect_recipient(player)
                && shared_stack_effect_at_position(*effect, true)
        }
        // Scheduling creates a fresh resolution boundary. A decision may
        // therefore be the delayed effect's root even when scheduling it
        // is itself one component of a sequence.
        EffectDef::IfCondition { then: effect, .. } => {
            shared_stack_effect_at_position(*effect, deferred_decision_allowed)
        }
        EffectDef::AtNextStep { effect, .. } => shared_stack_effect_at_position(*effect, true),
        // Installing an ability is a resolution like any other; what it
        // installs has to be an ability the shared runtime can fire.
        EffectDef::TriggerUntilYourNextTurn { ability } => shared_definition_ability(ability),
        EffectDef::Apply {
            recipient,
            effect,
            duration,
        } => shared_resolving_apply(recipient, effect, duration),
        // Only the moves the runtime actually performs are inside the
        // boundary. A move to the stack or command zone is still a seam.
        EffectDef::MoveToZone { object, zone, .. } => {
            matches!(
                zone,
                ZoneKind::Battlefield
                    | ZoneKind::Hand
                    | ZoneKind::Graveyard
                    | ZoneKind::Exile
                    | ZoneKind::Library
            ) && shared_effect_recipient(object)
        }
        EffectDef::None
        | EffectDef::CannotBeForcedToSacrifice
        | EffectDef::ReduceGenericCostBy(_)
        | EffectDef::PlayersCantPlay(_)
        | EffectDef::LandwalkCanBeBlocked(_)
        | EffectDef::CannotAttackUnless(_)
        | EffectDef::MultiplyEventAmount(_)
        | EffectDef::Replacement(_)
        | EffectDef::ChooseCardName { .. }
        | EffectDef::ChoosePlayer { .. }
        | EffectDef::CopyPermanentAsItEnters { .. }
        | EffectDef::ChooseCreatureType { .. }
        | EffectDef::Special(_) => false,
    }
}
