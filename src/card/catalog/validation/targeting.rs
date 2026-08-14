use crate::card::catalog::GrantedAbilityValidationError;
use crate::card::{
    AbilityProcedureDef, AbilityProgramDef, AbilityTargetDef, AppliedEffectDef,
    CharacteristicOperationDef, ConditionDef, DamageEventMatcherDef, DamagePreventionCapacityDef,
    DamageRecipientMatcherDef, DamageSourceMatcherDef, DeclarativeAbilityDef, EffectDef,
    EffectPaymentDef, EffectRecipientDef, EffectRecipientSetDef, ObjectQueryDef, ObjectRefDef,
    ObjectSetDef, PlayerRefDef, PlayerRelation, PlayerSetDef, PowerToughnessOperationDef,
    ReplacementEffectDef, TriggerConditionDef, ValueDef,
};
use crate::{ObjectBindingIndex, ObjectSetBindingIndex, TargetIndex};

#[cfg(test)]
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
    validate_effect_references(effect, targets.len(), BindingScope::EMPTY)
}

#[cfg(test)]
pub(in crate::card::catalog) fn validate_replacement_ability_targets(
    targets: &[AbilityTargetDef],
    effect: ReplacementEffectDef,
) -> Result<(), GrantedAbilityValidationError> {
    validate_target_definitions(targets)?;
    validate_replacement_effect_target_references(effect, targets.len(), BindingScope::EMPTY)
}

pub(super) fn validate_ability_program_targets(
    targets: &[AbilityTargetDef],
    program: AbilityProgramDef,
) -> Result<(), GrantedAbilityValidationError> {
    validate_target_definitions(targets)?;
    validate_program_references(program, targets.len(), BindingScope::EMPTY)
}

fn validate_program_references(
    program: AbilityProgramDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match program {
        AbilityProgramDef::Effects(effect) => {
            validate_effect_references(effect, target_count, scope)
        }
        AbilityProgramDef::Replacement(effect) => {
            validate_replacement_effect_target_references(effect, target_count, scope)
        }
    }
}

fn validate_target_definitions(
    targets: &[AbilityTargetDef],
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
    Ok(())
}

#[derive(Clone, Copy)]
struct BindingScope {
    objects: u8,
    object_sets: u8,
}

impl BindingScope {
    const EMPTY: Self = Self {
        objects: 0,
        object_sets: 0,
    };

    fn with_object(
        self,
        binding: ObjectBindingIndex,
    ) -> Result<Self, GrantedAbilityValidationError> {
        let bit = 1 << binding.index();
        if self.objects & bit != 0 {
            Err(GrantedAbilityValidationError::ObjectBindingAlreadyInScope { binding })
        } else {
            Ok(Self {
                objects: self.objects | bit,
                ..self
            })
        }
    }

    fn with_object_set(
        self,
        binding: ObjectSetBindingIndex,
    ) -> Result<Self, GrantedAbilityValidationError> {
        let bit = 1 << binding.index();
        if self.object_sets & bit != 0 {
            Err(GrantedAbilityValidationError::ObjectSetBindingAlreadyInScope { binding })
        } else {
            Ok(Self {
                object_sets: self.object_sets | bit,
                ..self
            })
        }
    }
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
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match reference {
        ObjectRefDef::Target(target) => validate_target_index(target, target_count),
        ObjectRefDef::Binding(binding) => {
            if scope.objects & (1 << binding.index()) != 0 {
                Ok(())
            } else {
                Err(GrantedAbilityValidationError::ObjectBindingReferenceOutOfScope { binding })
            }
        }
        ObjectRefDef::Source
        | ObjectRefDef::ResolvingObject
        | ObjectRefDef::AttachedToSource
        | ObjectRefDef::TriggeringObject => Ok(()),
    }
}

fn validate_player_reference(
    reference: PlayerRefDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match reference {
        PlayerRefDef::Target(target) => validate_target_index(target, target_count),
        PlayerRefDef::ControllerOf(reference) | PlayerRefDef::OwnerOf(reference) => {
            validate_object_reference(reference, target_count, scope)
        }
        PlayerRefDef::EffectController | PlayerRefDef::EventPlayer => Ok(()),
    }
}

fn validate_payment_references(
    payment: EffectPaymentDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match payment {
        EffectPaymentDef::Costs(_) => Ok(()),
        EffectPaymentDef::Mana { payer, .. } => {
            validate_player_reference(payer, target_count, scope)
        }
        EffectPaymentDef::GenericMana { payer, amount } => {
            validate_player_reference(payer, target_count, scope)?;
            validate_value_target_references(amount, target_count, scope)
        }
    }
}

fn validate_damage_matcher_references(
    matcher: DamageEventMatcherDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match matcher.source {
        DamageSourceMatcherDef::Object(reference) | DamageSourceMatcherDef::Except(reference) => {
            validate_object_reference(reference, target_count, scope)?;
        }
        DamageSourceMatcherDef::Any
        | DamageSourceMatcherDef::Group(_)
        | DamageSourceMatcherDef::AffectedObject
        | DamageSourceMatcherDef::Matching(_) => {}
    }
    match matcher.recipient {
        DamageRecipientMatcherDef::Recipients(recipient) => {
            validate_recipient_target_references(recipient, target_count, scope)
        }
        DamageRecipientMatcherDef::PlayerAndCreaturesControlledBy(player) => {
            validate_player_reference(player, target_count, scope)
        }
        DamageRecipientMatcherDef::Any | DamageRecipientMatcherDef::AffectedObject => Ok(()),
    }
}

fn validate_player_set(
    players: PlayerSetDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match players {
        PlayerSetDef::One(reference) => validate_player_reference(reference, target_count, scope),
        PlayerSetDef::All | PlayerSetDef::Related(_) => Ok(()),
    }
}

fn validate_pile_role(
    role: &'static str,
    players: PlayerSetDef,
) -> Result<(), GrantedAbilityValidationError> {
    if matches!(
        players,
        PlayerSetDef::All | PlayerSetDef::Related(PlayerRelation::Any)
    ) {
        Err(GrantedAbilityValidationError::InvalidPileRole { role, players })
    } else {
        Ok(())
    }
}

fn validate_query(
    query: ObjectQueryDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    if let Some(controller) = query.controller {
        validate_player_set(controller, target_count, scope)?;
    }
    if let Some(owner) = query.owner {
        validate_player_set(owner, target_count, scope)?;
    }
    if let Some(related_player) = query.related_player {
        validate_player_set(related_player, target_count, scope)?;
    }
    Ok(())
}

fn validate_condition(
    condition: ConditionDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match condition {
        ConditionDef::Exists(query) => validate_query(query, target_count, scope),
    }
}

fn validate_trigger_condition(
    condition: TriggerConditionDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match condition {
        TriggerConditionDef::ObjectCount { query, .. } => {
            validate_query(query, target_count, scope)
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
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match recipient.0 {
        EffectRecipientSetDef::LegalTargets(target) => validate_target_index(target, target_count),
        EffectRecipientSetDef::Objects(ObjectSetDef::One(reference))
        | EffectRecipientSetDef::Objects(ObjectSetDef::SharingNameWith(reference)) => {
            validate_object_reference(reference, target_count, scope)
        }
        EffectRecipientSetDef::Objects(ObjectSetDef::Binding(binding)) => {
            if scope.object_sets & (1 << binding.index()) != 0 {
                Ok(())
            } else {
                Err(GrantedAbilityValidationError::ObjectSetBindingReferenceOutOfScope { binding })
            }
        }
        EffectRecipientSetDef::Objects(ObjectSetDef::Query(query)) => {
            validate_query(query, target_count, scope)
        }
        EffectRecipientSetDef::Players(players) => {
            validate_player_set(players, target_count, scope)
        }
    }
}

fn validate_value_target_references(
    value: ValueDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match value {
        ValueDef::Negate(value) => {
            validate_value_target_references(*value, target_count, scope)
        }
        ValueDef::Scaled(scaled) => {
            validate_value_target_references(scaled.value, target_count, scope)
        }
        ValueDef::IfCreatureDiedThisTurn(condition) => {
            validate_value_target_references(condition.then, target_count, scope)?;
            validate_value_target_references(condition.otherwise, target_count, scope)
        }
        ValueDef::IfTargetMatches(condition) => {
            validate_target_index(condition.slot, target_count)?;
            validate_value_target_references(condition.then, target_count, scope)?;
            validate_value_target_references(condition.otherwise, target_count, scope)
        }
        ValueDef::IfMatchingObjectCount(condition) => {
            validate_query(condition.query, target_count, scope)?;
            validate_value_target_references(condition.then, target_count, scope)?;
            validate_value_target_references(condition.otherwise, target_count, scope)
        }
        ValueDef::CountMatchingObjects(query) | ValueDef::AnyMatchingObject(query) => {
            validate_query(*query, target_count, scope)
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
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        AppliedEffectDef::Composite(effects) => {
            for effect in effects {
                validate_applied_effect_target_references(*effect, target_count, scope)?;
            }
            Ok(())
        }
        AppliedEffectDef::Characteristic(CharacteristicOperationDef::PowerToughness(
            PowerToughnessOperationDef::SetBase { power, toughness }
            | PowerToughnessOperationDef::Modify { power, toughness },
        )) => {
            validate_value_target_references(power, target_count, scope)?;
            validate_value_target_references(toughness, target_count, scope)
        }
        AppliedEffectDef::PreventDamage(matcher) => {
            validate_damage_matcher_references(matcher, target_count, scope)
        }
        // A granted ability introduces its own target scope and is validated
        // separately when the grant tree is traversed.
        AppliedEffectDef::CannotBeCountered
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
        | AppliedEffectDef::RedirectPlayerDamageToThis(_)
        | AppliedEffectDef::Characteristic(_)
        | AppliedEffectDef::Special(_) => Ok(()),
    }
}

#[allow(clippy::too_many_lines)]
fn validate_effect_references(
    effect: EffectDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        EffectDef::Sequence(effects) => {
            for effect in effects {
                validate_effect_references(*effect, target_count, scope)?;
            }
            Ok(())
        }
        EffectDef::Randomized {
            on_success,
            on_failure,
            ..
        } => {
            validate_effect_references(*on_success, target_count, scope)?;
            validate_effect_references(*on_failure, target_count, scope)
        }
        EffectDef::Choose(choice) => {
            validate_player_reference(choice.chooser, target_count, scope)?;
            validate_recipient_target_references(
                EffectRecipientDef::objects(choice.candidates),
                target_count,
                scope,
            )?;
            if let Some(excluded) = choice.exclude {
                validate_object_reference(excluded, target_count, scope)?;
            }
            if choice.minimum > choice.maximum
                || matches!(
                    choice.binding,
                    crate::card::ObjectChoiceBindingDef::Object(_)
                ) && choice.maximum > 1
            {
                return Err(GrantedAbilityValidationError::InvalidObjectChoiceBounds {
                    binding: choice.binding,
                    minimum: choice.minimum,
                    maximum: choice.maximum,
                });
            }
            let nested = match choice.binding {
                crate::card::ObjectChoiceBindingDef::Object(binding) => {
                    scope.with_object(binding)?
                }
                crate::card::ObjectChoiceBindingDef::Objects(binding) => {
                    scope.with_object_set(binding)?
                }
            };
            validate_effect_references(*choice.then, target_count, nested)
        }
        EffectDef::PayOr(payment) => {
            validate_payment_references(payment.payment, target_count, scope)?;
            for branch in payment.if_paid.iter().chain(payment.otherwise.iter()) {
                validate_effect_references(**branch, target_count, scope)?;
            }
            Ok(())
        }
        EffectDef::PreventDamage { prevention, .. } => {
            validate_damage_matcher_references(prevention.matcher, target_count, scope)?;
            if let DamagePreventionCapacityDef::Amount(amount) = prevention.capacity {
                validate_value_target_references(amount, target_count, scope)?;
            }
            Ok(())
        }
        EffectDef::SplitIntoPiles(partition) => {
            validate_pile_role("divider", partition.divider)?;
            validate_pile_role("chooser", partition.chooser)?;
            match partition.items {
                crate::card::PartitionItemsDef::Objects(objects) => {
                    validate_recipient_target_references(
                        EffectRecipientDef::objects(objects),
                        target_count,
                        scope,
                    )?;
                }
                crate::card::PartitionItemsDef::TopOfLibrary { player, count } => {
                    validate_player_reference(player, target_count, scope)?;
                    validate_value_target_references(count, target_count, scope)?;
                }
            }
            validate_player_set(partition.divider, target_count, scope)?;
            validate_player_set(partition.chooser, target_count, scope)?;
            let nested = scope
                .with_object_set(partition.chosen)?
                .with_object_set(partition.unchosen)?;
            validate_effect_references(*partition.then, target_count, nested)
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
            validate_recipient_target_references(recipient, target_count, scope)?;
            validate_value_target_references(amount, target_count, scope)
        }
        EffectDef::LoseTheGame { player: object }
        | EffectDef::ShuffleLibrary { player: object }
        | EffectDef::EmptyManaPool { player: object }
        | EffectDef::Regenerate { object }
        | EffectDef::Tap { object }
        | EffectDef::RemoveFromCombat { object }
        | EffectDef::DestroyAtEndOfCombat { object, .. }
        | EffectDef::SkipNextUntapSteps { object, .. }
        | EffectDef::DoesNotUntapWhileSourceTapped { object }
        | EffectDef::RemoveAllCounters { object, .. }
        | EffectDef::Untap { object }
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
        | EffectDef::CreateTokenCopyOf { object } => {
            validate_recipient_target_references(object, target_count, scope)
        }
        EffectDef::CreateToken { count, .. } | EffectDef::ReduceGenericCostBy(count) => {
            validate_value_target_references(count, target_count, scope)
        }
        EffectDef::SacrificeOfChoice { player, then, .. } => {
            validate_recipient_target_references(player, target_count, scope)?;
            if let Some(effect) = then {
                validate_effect_references(*effect, target_count, scope)?;
            }
            Ok(())
        }
        EffectDef::CannotCastNoncreatureSpellsThisTurn { player }
        | EffectDef::SearchZone { player, .. }
        | EffectDef::ChooseCards { player, .. }
        | EffectDef::TakeExtraTurn { player }
        | EffectDef::LookAtHand { player }
        | EffectDef::LookAtTopAndMayTake { player, .. } => {
            validate_recipient_target_references(player, target_count, scope)
        }
        EffectDef::LookAtTopAndSelect { player, selection } => {
            validate_recipient_target_references(player, target_count, scope)?;
            validate_value_target_references(selection.count, target_count, scope)?;
            if let Some(effect) = selection.then {
                validate_effect_references(*effect, target_count, scope)?;
            }
            Ok(())
        }
        EffectDef::May { player, effect }
        | EffectDef::ReplaceNextDrawThisTurn { player, effect } => {
            validate_recipient_target_references(player, target_count, scope)?;
            validate_effect_references(*effect, target_count, scope)
        }
        EffectDef::Mill { player, amount } => {
            validate_recipient_target_references(player, target_count, scope)?;
            validate_value_target_references(amount, target_count, scope)
        }
        EffectDef::AddCounters { object, amount, .. } => {
            validate_recipient_target_references(object, target_count, scope)?;
            validate_value_target_references(amount, target_count, scope)
        }
        EffectDef::InstallTrigger(trigger) => {
            let DeclarativeAbilityDef::Triggered(definition) = trigger.ability.definition else {
                return Err(GrantedAbilityValidationError::UnsupportedInstalledTriggerAbility);
            };
            if definition.procedure != AbilityProcedureDef::Shared
                || !definition.targets.is_empty()
                || trigger.ability.declarative_effect().is_none()
            {
                return Err(GrantedAbilityValidationError::UnsupportedInstalledTriggerAbility);
            }
            if let Some(condition) = definition.condition {
                validate_trigger_condition(*condition, target_count, scope)?;
            }
            if let crate::card::InstalledTriggerLifetimeDef::UntilNextTurn(player) =
                trigger.lifetime
            {
                validate_player_reference(player, target_count, scope)?;
            }
            validate_program_references(trigger.ability.effect.definition, target_count, scope)
        }
        EffectDef::IfCondition { condition, then } => {
            validate_trigger_condition(*condition, target_count, scope)?;
            validate_effect_references(*then, target_count, scope)
        }
        EffectDef::IfFormat {
            then, otherwise, ..
        } => {
            validate_effect_references(*then, target_count, scope)?;
            validate_effect_references(*otherwise, target_count, scope)
        }
        EffectDef::StaticApply {
            recipient, effect, ..
        }
        | EffectDef::Apply {
            recipient, effect, ..
        } => {
            validate_recipient_target_references(recipient, target_count, scope)?;
            validate_applied_effect_target_references(effect, target_count, scope)
        }
        // The chosen player is recorded on the permanent, not read from a
        // target slot.
        // A prohibition names a card shape, never a target.
        EffectDef::PlayersCantPlay(_)
        | EffectDef::LandwalkCanBeBlocked(_)
        | EffectDef::CannotAttackUnless(_)
        | EffectDef::None
        | EffectDef::AddMana(_)
        | EffectDef::AddManaEqualTo { .. }
        | EffectDef::CreateEmblem { .. }
        | EffectDef::GrantFlashToNextSorcery
        | EffectDef::ReturnLinkedExiles { .. }
        | EffectDef::CannotBeForcedToSacrifice
        | EffectDef::AdditionalCombatPhase
        | EffectDef::Special(_) => Ok(()),
    }
}

fn validate_replacement_effect_target_references(
    effect: ReplacementEffectDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        ReplacementEffectDef::Sequence(effects) => {
            for effect in effects {
                validate_replacement_effect_target_references(*effect, target_count, scope)?;
            }
            Ok(())
        }
        ReplacementEffectDef::Conditional {
            condition,
            if_true,
            if_false,
        } => {
            validate_condition(condition, target_count, scope)?;
            for effect in if_true.iter().chain(if_false.iter()) {
                validate_replacement_effect_target_references(*effect, target_count, scope)?;
            }
            Ok(())
        }
        ReplacementEffectDef::PayOr {
            payment,
            if_paid,
            if_declined,
        } => {
            validate_payment_references(payment, target_count, scope)?;
            for effect in if_paid.iter().chain(if_declined.iter()) {
                validate_replacement_effect_target_references(*effect, target_count, scope)?;
            }
            Ok(())
        }
        ReplacementEffectDef::Perform(effect) => {
            validate_effect_references(*effect, target_count, scope)
        }
        ReplacementEffectDef::ReplaceEventWithNothing
        | ReplacementEffectDef::MoveToZone(_)
        | ReplacementEffectDef::ModifyBattlefieldEntry(_)
        | ReplacementEffectDef::MultiplyEventAmount(_)
        | ReplacementEffectDef::Choose(_)
        | ReplacementEffectDef::CopyEntering { .. } => Ok(()),
    }
}
