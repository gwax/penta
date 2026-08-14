use crate::TargetIndex;
use crate::card::catalog::GrantedAbilityValidationError;
use crate::card::{
    AbilityTargetDef, AppliedEffectDef, ConditionDef, EffectDef, EffectRecipientDef,
    EffectRecipientSetDef, ObjectQueryDef, ObjectRefDef, ObjectSetDef, PlayerRefDef, PlayerSetDef,
    ReplacementEffectDef, TriggerConditionDef, ValueDef,
};

pub(in crate::card::catalog) fn validate_ability_targets(
    targets: &[AbilityTargetDef],
    effect: EffectDef,
) -> Result<(), GrantedAbilityValidationError> {
    if targets.len() > usize::from(u8::MAX) + 1 {
        return Err(GrantedAbilityValidationError::TooManyTargets {
            count: targets.len(),
        });
    }
    for (position, definition) in targets.iter().enumerate() {
        let target = TargetIndex::from_index(position)
            .expect("the target count was validated before assigning positional indices");
        if definition.minimum > definition.maximum {
            return Err(GrantedAbilityValidationError::InvalidTargetBounds {
                target,
                minimum: definition.minimum,
                maximum: definition.maximum,
            });
        }
    }
    validate_effect_references(effect, targets.len(), 0)
}

fn validate_target_index(
    target: TargetIndex,
    target_count: usize,
) -> Result<(), GrantedAbilityValidationError> {
    if target.index() < target_count {
        Ok(())
    } else {
        Err(GrantedAbilityValidationError::TargetReferenceOutOfBounds {
            target,
            target_count,
        })
    }
}

fn validate_object_reference(
    reference: ObjectRefDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match reference {
        ObjectRefDef::Target(target) => validate_target_index(target, target_count),
        ObjectRefDef::Choice(choice) => {
            if choices_in_scope & (1 << choice.index()) != 0 {
                Ok(())
            } else {
                Err(GrantedAbilityValidationError::ChoiceReferenceOutOfScope { choice })
            }
        }
        ObjectRefDef::Source | ObjectRefDef::AttachedToSource | ObjectRefDef::TriggeringObject => {
            Ok(())
        }
    }
}

fn validate_player_reference(
    reference: PlayerRefDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match reference {
        PlayerRefDef::Target(target) => validate_target_index(target, target_count),
        PlayerRefDef::ControllerOf(reference) | PlayerRefDef::OwnerOf(reference) => {
            validate_object_reference(reference, target_count, choices_in_scope)
        }
        PlayerRefDef::EffectController | PlayerRefDef::EventPlayer => Ok(()),
    }
}

fn validate_player_set(
    players: PlayerSetDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match players {
        PlayerSetDef::One(reference) => {
            validate_player_reference(reference, target_count, choices_in_scope)
        }
        PlayerSetDef::All | PlayerSetDef::Related(_) => Ok(()),
    }
}

fn validate_query(
    query: ObjectQueryDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    if let Some(controller) = query.controller {
        validate_player_set(controller, target_count, choices_in_scope)?;
    }
    if let Some(owner) = query.owner {
        validate_player_set(owner, target_count, choices_in_scope)?;
    }
    if let Some(related_player) = query.related_player {
        validate_player_set(related_player, target_count, choices_in_scope)?;
    }
    Ok(())
}

fn validate_condition(
    condition: ConditionDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match condition {
        ConditionDef::Exists(query) => validate_query(query, target_count, choices_in_scope),
    }
}

fn validate_trigger_condition(
    condition: TriggerConditionDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match condition {
        TriggerConditionDef::ObjectCount { query, .. } => {
            validate_query(query, target_count, choices_in_scope)
        }
        TriggerConditionDef::TargetMatches { slot, .. } => {
            validate_target_index(slot, target_count)
        }
        TriggerConditionDef::SourceOnBattlefield
        | TriggerConditionDef::SourceUntapped
        | TriggerConditionDef::ActivePlayer(_)
        | TriggerConditionDef::SpellsCastLastTurn { .. }
        | TriggerConditionDef::ControlsGreatestPowerCreature
        | TriggerConditionDef::AttachedPermanentMatches { .. }
        | TriggerConditionDef::SourceCounters { .. }
        | TriggerConditionDef::SourceLoyalty { .. }
        | TriggerConditionDef::SourceActivationsThisTurn { .. }
        | TriggerConditionDef::SourceDealtDamageToOpponentThisTurn
        | TriggerConditionDef::SourceIsTapped => Ok(()),
    }
}

fn validate_recipient_target_references(
    recipient: EffectRecipientDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match recipient.0 {
        EffectRecipientSetDef::LegalTargets(target) => validate_target_index(target, target_count),
        EffectRecipientSetDef::Objects(ObjectSetDef::One(reference))
        | EffectRecipientSetDef::Objects(ObjectSetDef::SharingNameWith(reference)) => {
            validate_object_reference(reference, target_count, choices_in_scope)
        }
        EffectRecipientSetDef::Objects(ObjectSetDef::Query(query)) => {
            validate_query(query, target_count, choices_in_scope)
        }
        EffectRecipientSetDef::Players(players) => {
            validate_player_set(players, target_count, choices_in_scope)
        }
    }
}

fn validate_value_target_references(
    value: ValueDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match value {
        ValueDef::Negate(value) => {
            validate_value_target_references(*value, target_count, choices_in_scope)
        }
        ValueDef::Scaled(scaled) => {
            validate_value_target_references(scaled.value, target_count, choices_in_scope)
        }
        ValueDef::IfCreatureDiedThisTurn(condition) => {
            validate_value_target_references(condition.then, target_count, choices_in_scope)?;
            validate_value_target_references(condition.otherwise, target_count, choices_in_scope)
        }
        ValueDef::IfTargetMatches(condition) => {
            validate_target_index(condition.slot, target_count)?;
            validate_value_target_references(condition.then, target_count, choices_in_scope)?;
            validate_value_target_references(condition.otherwise, target_count, choices_in_scope)
        }
        ValueDef::IfMatchingObjectCount(condition) => {
            validate_query(condition.query, target_count, choices_in_scope)?;
            validate_value_target_references(condition.then, target_count, choices_in_scope)?;
            validate_value_target_references(condition.otherwise, target_count, choices_in_scope)
        }
        ValueDef::CountMatchingObjects(query) | ValueDef::AnyMatchingObject(query) => {
            validate_query(*query, target_count, choices_in_scope)
        }
        ValueDef::TargetPower(target)
        | ValueDef::TargetManaValue(target) => validate_target_index(target, target_count),
        ValueDef::Constant(_)
        | ValueDef::ChosenX
        | ValueDef::SourcePower
        | ValueDef::TriggeringObjectPower
        | ValueDef::SourceToughness
        | ValueDef::TriggerEventAmount
        | ValueDef::CardsInHandAbove { .. }
        | ValueDef::CountersOnSource(_)
        // This reads the share assigned to the target currently being
        // affected; the surrounding recipient carries the slot reference.
        | ValueDef::DividedAmongTargets => Ok(()),
    }
}

fn validate_applied_effect_target_references(
    effect: AppliedEffectDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        AppliedEffectDef::Composite(effects) => {
            for effect in effects {
                validate_applied_effect_target_references(*effect, target_count, choices_in_scope)?;
            }
            Ok(())
        }
        AppliedEffectDef::ModifyPowerToughness { power, toughness } => {
            validate_value_target_references(power, target_count, choices_in_scope)?;
            validate_value_target_references(toughness, target_count, choices_in_scope)
        }
        // A granted ability introduces its own target scope and is validated
        // separately when the grant tree is traversed.
        AppliedEffectDef::GrantAbility(_)
        | AppliedEffectDef::CannotBeCountered
        | AppliedEffectDef::DoesNotUntapDuringUntapStep
        | AppliedEffectDef::MayChooseNotToUntap
        | AppliedEffectDef::CannotBlock
        | AppliedEffectDef::CannotAttack
        | AppliedEffectDef::CannotBeBlocked
        | AppliedEffectDef::CannotBeEnchanted
        | AppliedEffectDef::CannotBecomeEnchanted
        | AppliedEffectDef::CannotChangeController
        | AppliedEffectDef::RemainsAttachedThroughProtection
        | AppliedEffectDef::CannotBeBlockedBy(_)
        | AppliedEffectDef::CanBlockOnly(_)
        | AppliedEffectDef::PreventDamageFrom(_)
        | AppliedEffectDef::PreventCombatDamageFrom(_)
        | AppliedEffectDef::RedirectPlayerDamageToThis(_)
        | AppliedEffectDef::PreventCombatDamage
        | AppliedEffectDef::PreventCombatDamageDealtBy
        | AppliedEffectDef::AddLandTypes(_)
        | AppliedEffectDef::SetLandTypes(_)
        | AppliedEffectDef::RemoveAbilities(_)
        | AppliedEffectDef::Animate(_)
        | AppliedEffectDef::Special(_) => Ok(()),
    }
}

#[allow(clippy::too_many_lines)]
fn validate_effect_references(
    effect: EffectDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        EffectDef::Sequence(effects) => {
            for effect in effects {
                validate_effect_references(*effect, target_count, choices_in_scope)?;
            }
            Ok(())
        }
        EffectDef::Randomized {
            on_success,
            on_failure,
            ..
        } => {
            validate_effect_references(*on_success, target_count, choices_in_scope)?;
            validate_effect_references(*on_failure, target_count, choices_in_scope)
        }
        EffectDef::ChooseDamageSource {
            choice,
            chooser,
            then,
            ..
        }
        | EffectDef::ChoosePermanent {
            choice,
            chooser,
            then,
            ..
        } => {
            validate_recipient_target_references(chooser, target_count, choices_in_scope)?;
            let bit = 1 << choice.index();
            if choices_in_scope & bit != 0 {
                return Err(GrantedAbilityValidationError::ChoiceBindingAlreadyInScope { choice });
            }
            validate_effect_references(*then, target_count, choices_in_scope | bit)
        }
        EffectDef::DealDamage { recipient, amount }
        | EffectDef::DrainLife { recipient, amount }
        | EffectDef::GainLife { recipient, amount }
        | EffectDef::AddPoisonCounters { recipient, amount }
        | EffectDef::DrawCards { recipient, amount }
        | EffectDef::Discard {
            recipient, amount, ..
        }
        | EffectDef::LoseLife { recipient, amount } => {
            validate_recipient_target_references(recipient, target_count, choices_in_scope)?;
            validate_value_target_references(amount, target_count, choices_in_scope)
        }
        EffectDef::LoseTheGame { player: object }
        | EffectDef::ShuffleLibrary { player: object }
        | EffectDef::EmptyManaPool { player: object }
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
        | EffectDef::PreventNextDamageFromSource { object, .. }
        | EffectDef::PreventCombatDamageThisTurn { object }
        | EffectDef::PreventCombatDamageDealtByThisTurn { object }
        | EffectDef::PreventDamageDealtByThisTurn { object }
        | EffectDef::PreventDamageToPlayerAndControlledCreaturesThisTurn { player: object }
        | EffectDef::PreventDamageToPlayerFromThisTurn { player: object, .. }
        | EffectDef::PreventAllCombatDamageExceptSourceThisTurn { source: object }
        | EffectDef::RedirectTargetDamageToSourceThisTurn { player: object, .. }
        | EffectDef::Attach { object }
        | EffectDef::Destroy { object, .. }
        | EffectDef::Sacrifice { object }
        | EffectDef::ChangeTextBasicLandType { object }
        | EffectDef::BecomeCopyOf { object, .. }
        | EffectDef::ExileLinkedToSource { object }
        | EffectDef::Detain { object }
        | EffectDef::CannotRegenerateThisTurn { object }
        | EffectDef::MakeUnblockableThisTurn { object }
        | EffectDef::GainControlWhileSourceRemains { object, .. }
        | EffectDef::GainControlThisTurn { object }
        | EffectDef::Transform { object }
        | EffectDef::MoveToZone { object, .. }
        | EffectDef::Counter { object, .. }
        | EffectDef::ChooseCardName { object }
        | EffectDef::ChooseCreatureType { object }
        | EffectDef::CreateTokenCopyOf { object } => {
            validate_recipient_target_references(object, target_count, choices_in_scope)
        }
        // A reveal always comes off the resolving object's controller's own
        // library, so its count is the only part that could name a target.
        EffectDef::RevealAndSplitIntoPiles { count, .. }
        | EffectDef::CreateToken { count, .. }
        | EffectDef::ReduceGenericCostBy(count) => {
            validate_value_target_references(count, target_count, choices_in_scope)
        }
        EffectDef::SacrificeOfChoice { player, then, .. } => {
            validate_recipient_target_references(player, target_count, choices_in_scope)?;
            if let Some(effect) = then {
                validate_effect_references(*effect, target_count, choices_in_scope)?;
            }
            Ok(())
        }
        EffectDef::DestroyOfChoice { player, .. }
        | EffectDef::SplitPermanentsAndSacrificeAPile { player }
        | EffectDef::CannotCastNoncreatureSpellsThisTurn { player }
        | EffectDef::SearchZone { player, .. }
        | EffectDef::ChooseCards { player, .. }
        | EffectDef::TakeExtraTurn { player }
        | EffectDef::LookAtHand { player }
        | EffectDef::LookAtTopAndMayTake { player, .. } => {
            validate_recipient_target_references(player, target_count, choices_in_scope)
        }
        EffectDef::LookAtTopAndSelect { player, selection } => {
            validate_recipient_target_references(player, target_count, choices_in_scope)?;
            validate_value_target_references(selection.count, target_count, choices_in_scope)?;
            if let Some(effect) = selection.then {
                validate_effect_references(*effect, target_count, choices_in_scope)?;
            }
            Ok(())
        }
        EffectDef::May { player, effect }
        | EffectDef::ReplaceNextDrawThisTurn { player, effect } => {
            validate_recipient_target_references(player, target_count, choices_in_scope)?;
            validate_effect_references(*effect, target_count, choices_in_scope)
        }
        EffectDef::Mill { player, amount } => {
            validate_recipient_target_references(player, target_count, choices_in_scope)?;
            validate_value_target_references(amount, target_count, choices_in_scope)
        }
        EffectDef::CounterUnlessPaid { object, amount, .. }
        | EffectDef::AddCounters { object, amount, .. } => {
            validate_recipient_target_references(object, target_count, choices_in_scope)?;
            validate_value_target_references(amount, target_count, choices_in_scope)
        }
        EffectDef::OptionalPayment {
            if_paid: effect, ..
        }
        | EffectDef::UnlessPaid {
            otherwise: effect, ..
        }
        | EffectDef::AtNextStep { effect, .. } => {
            validate_effect_references(*effect, target_count, choices_in_scope)
        }
        EffectDef::IfCondition { condition, then } => {
            validate_trigger_condition(*condition, target_count, choices_in_scope)?;
            validate_effect_references(*then, target_count, choices_in_scope)
        }
        EffectDef::IfFormat {
            then, otherwise, ..
        } => {
            validate_effect_references(*then, target_count, choices_in_scope)?;
            validate_effect_references(*otherwise, target_count, choices_in_scope)
        }
        EffectDef::Apply {
            recipient, effect, ..
        } => {
            validate_recipient_target_references(recipient, target_count, choices_in_scope)?;
            validate_applied_effect_target_references(effect, target_count, choices_in_scope)
        }
        // An installed ability chooses its own targets when it triggers, so
        // nothing in it can refer to this ability's target slots.
        // The chosen player is recorded on the permanent, not read from a
        // target slot.
        // A prohibition names a card shape, never a target.
        EffectDef::PlayersCantPlay(_)
        | EffectDef::LandwalkCanBeBlocked(_)
        | EffectDef::CannotAttackUnless(_)
        | EffectDef::ChoosePlayer { .. }
        | EffectDef::CopyPermanentAsItEnters { .. }
        | EffectDef::TriggerUntilYourNextTurn { .. }
        | EffectDef::None
        | EffectDef::AddMana(_)
        | EffectDef::AddManaEqualTo { .. }
        | EffectDef::CreateEmblem { .. }
        | EffectDef::GrantFlashToNextSorcery
        | EffectDef::ReturnLinkedExiles { .. }
        | EffectDef::CannotBeForcedToSacrifice
        | EffectDef::AdditionalCombatPhase
        | EffectDef::PreventAllCombatDamageThisTurn
        | EffectDef::MultiplyEventAmount(_)
        | EffectDef::Special(_) => Ok(()),
        EffectDef::Replacement(effect) => {
            validate_replacement_effect_target_references(effect, target_count, choices_in_scope)
        }
    }
}

fn validate_replacement_effect_target_references(
    effect: ReplacementEffectDef,
    target_count: usize,
    choices_in_scope: u8,
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        ReplacementEffectDef::Sequence(effects) => {
            for effect in effects {
                validate_replacement_effect_target_references(
                    *effect,
                    target_count,
                    choices_in_scope,
                )?;
            }
            Ok(())
        }
        ReplacementEffectDef::Conditional {
            condition,
            if_true,
            if_false,
        } => {
            validate_condition(condition, target_count, choices_in_scope)?;
            for effect in if_true.iter().chain(if_false.iter()) {
                validate_replacement_effect_target_references(
                    *effect,
                    target_count,
                    choices_in_scope,
                )?;
            }
            Ok(())
        }
        ReplacementEffectDef::OptionalPayment {
            if_paid,
            if_declined,
            ..
        } => {
            for effect in if_paid.iter().chain(if_declined.iter()) {
                validate_replacement_effect_target_references(
                    *effect,
                    target_count,
                    choices_in_scope,
                )?;
            }
            Ok(())
        }
        ReplacementEffectDef::Perform(effect) => {
            validate_effect_references(*effect, target_count, choices_in_scope)
        }
        ReplacementEffectDef::None
        | ReplacementEffectDef::ReplaceEventWithNothing
        | ReplacementEffectDef::MoveToZone(_)
        | ReplacementEffectDef::ModifyBattlefieldEntry(_) => Ok(()),
    }
}
