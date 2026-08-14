use crate::card::catalog::{EffectSubjectKind, GrantedAbilityValidationError};
use crate::card::{
    AbilityOperationDef, AbilityProcedureDef, AbilityProgramDef, AbilityTargetDef,
    AbilityTargetPredicate, AlternativeCastKindDef, AppliedEffectDef, AppliedRuleDef,
    BattlefieldEntryChoiceDestinationDef, CharacteristicOperationDef, ConditionDef,
    DamageEventMatcherDef, DamagePreventionCapacityDef, DamageRecipientMatcherDef,
    DamageSourceMatcherDef, DeclarativeAbilityDef, EffectDef, EffectPaymentCostDef,
    EffectPaymentDef, EffectRecipientDef, EffectRecipientSetDef, ObjectPredicateDef,
    ObjectQueryDef, ObjectRefDef, ObjectSetDef, PlayerRefDef, PlayerRelation, PlayerSetDef,
    PowerToughnessOperationDef, ReplacementChoiceDef, ReplacementEffectDef,
    ResolvedEffectDurationDef, ScalarChoiceListDef, TriggerConditionDef, TriggerEventDef, ValueDef,
    ZoneKind,
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
    validate_effect_references(effect, targets.len(), BindingScope::EMPTY)?;
    validate_effect_target_shapes(effect, targets, None)
}

#[cfg(test)]
pub(in crate::card::catalog) fn validate_replacement_ability_targets(
    targets: &[AbilityTargetDef],
    effect: ReplacementEffectDef,
) -> Result<(), GrantedAbilityValidationError> {
    validate_target_definitions(targets)?;
    validate_replacement_effect_target_references(effect, targets.len(), BindingScope::EMPTY)?;
    validate_replacement_effect_target_shapes(effect, targets)
}

pub(super) fn validate_ability_program_targets(
    targets: &[AbilityTargetDef],
    program: AbilityProgramDef,
    trigger_event: Option<TriggerEventDef>,
) -> Result<(), GrantedAbilityValidationError> {
    validate_target_definitions(targets)?;
    validate_program_references(program, targets.len(), BindingScope::EMPTY)?;
    validate_program_target_shapes(program, targets, trigger_event)
}

pub(super) fn validate_ability_trigger_event(
    event: TriggerEventDef,
    target_count: usize,
) -> Result<(), GrantedAbilityValidationError> {
    validate_trigger_event_references(event, target_count, BindingScope::EMPTY)
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
    validate_single_payment_payer(payment.payer)?;
    validate_player_set(payment.payer, target_count, scope)?;
    if let EffectPaymentCostDef::GenericMana(amount) = payment.cost {
        validate_value_target_references(amount, target_count, scope)?;
    }
    Ok(())
}

fn validate_single_payment_payer(
    players: PlayerSetDef,
) -> Result<(), GrantedAbilityValidationError> {
    if matches!(
        players,
        PlayerSetDef::All | PlayerSetDef::Related(PlayerRelation::Any)
    ) {
        Err(GrantedAbilityValidationError::InvalidPaymentPayer { players })
    } else {
        Ok(())
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

fn unsupported_trigger_event(event: TriggerEventDef) -> GrantedAbilityValidationError {
    GrantedAbilityValidationError::UnsupportedTriggerEvent { event }
}

fn validate_trigger_object_predicate(
    predicate: ObjectPredicateDef,
    event: TriggerEventDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match predicate {
        ObjectPredicateDef::All(predicates) | ObjectPredicateDef::AnyOf(predicates) => {
            for predicate in predicates {
                validate_trigger_object_predicate(*predicate, event, target_count, scope)?;
            }
            Ok(())
        }
        ObjectPredicateDef::Not(predicate) | ObjectPredicateDef::AttachedTo(predicate) => {
            validate_trigger_object_predicate(*predicate, event, target_count, scope)
        }
        ObjectPredicateDef::ManaValueEqualTo(value)
        | ObjectPredicateDef::ManaValueAtMostValue(value)
        | ObjectPredicateDef::ToughnessLessThan(value)
        | ObjectPredicateDef::PowerGreaterThan(value)
        | ObjectPredicateDef::ToughnessGreaterThan(value)
        | ObjectPredicateDef::PowerLessThan(value) => {
            validate_value_target_references(value, target_count, scope)?;
            if matches!(
                value,
                ValueDef::Constant(_)
                    | ValueDef::ChosenX
                    | ValueDef::SourcePower
                    | ValueDef::SourceToughness
                    | ValueDef::CountersOnSource(_)
            ) {
                Ok(())
            } else {
                Err(unsupported_trigger_event(event))
            }
        }
        ObjectPredicateDef::ControlledBy(
            PlayerRelation::ChosenPlayer | PlayerRelation::EventPlayer,
        )
        | ObjectPredicateDef::Special(_) => Err(unsupported_trigger_event(event)),
        ObjectPredicateDef::Any
        | ObjectPredicateDef::Source
        | ObjectPredicateDef::Token
        | ObjectPredicateDef::Tapped
        | ObjectPredicateDef::HasType(_)
        | ObjectPredicateDef::HasAnyBasicLandType(_)
        | ObjectPredicateDef::Spell
        | ObjectPredicateDef::NoncreatureSpell
        | ObjectPredicateDef::Color(_)
        | ObjectPredicateDef::ColorCount(_)
        | ObjectPredicateDef::Subtype(_)
        | ObjectPredicateDef::ManaValueAtMost(_)
        | ObjectPredicateDef::PowerAtLeast(_)
        | ObjectPredicateDef::PowerExactly(_)
        | ObjectPredicateDef::ToughnessExactly(_)
        | ObjectPredicateDef::HasCounter(_)
        | ObjectPredicateDef::ControlledBy(_)
        | ObjectPredicateDef::Supertype(_)
        | ObjectPredicateDef::DebutSet(_)
        | ObjectPredicateDef::SharesNameWithSource
        | ObjectPredicateDef::AttackingOrBlocking
        | ObjectPredicateDef::HasKeyword(_)
        | ObjectPredicateDef::HasNonManaActivatedAbility
        | ObjectPredicateDef::Attacking
        | ObjectPredicateDef::AttachedToSource
        | ObjectPredicateDef::Blocking
        | ObjectPredicateDef::BlockedBySource
        | ObjectPredicateDef::Enchanted
        | ObjectPredicateDef::AttackedThisTurn => Ok(()),
    }
}

fn trigger_predicate_requires_live_battlefield(predicate: ObjectPredicateDef) -> bool {
    match predicate {
        ObjectPredicateDef::All(predicates) | ObjectPredicateDef::AnyOf(predicates) => predicates
            .iter()
            .copied()
            .any(trigger_predicate_requires_live_battlefield),
        ObjectPredicateDef::Not(predicate) => {
            trigger_predicate_requires_live_battlefield(*predicate)
        }
        ObjectPredicateDef::HasNonManaActivatedAbility | ObjectPredicateDef::AttachedTo(_) => true,
        ObjectPredicateDef::Any
        | ObjectPredicateDef::Source
        | ObjectPredicateDef::Token
        | ObjectPredicateDef::Tapped
        | ObjectPredicateDef::HasType(_)
        | ObjectPredicateDef::HasAnyBasicLandType(_)
        | ObjectPredicateDef::Spell
        | ObjectPredicateDef::NoncreatureSpell
        | ObjectPredicateDef::Color(_)
        | ObjectPredicateDef::ColorCount(_)
        | ObjectPredicateDef::Subtype(_)
        | ObjectPredicateDef::ManaValueAtMost(_)
        | ObjectPredicateDef::ManaValueEqualTo(_)
        | ObjectPredicateDef::ManaValueAtMostValue(_)
        | ObjectPredicateDef::PowerAtLeast(_)
        | ObjectPredicateDef::PowerExactly(_)
        | ObjectPredicateDef::ToughnessExactly(_)
        | ObjectPredicateDef::ToughnessLessThan(_)
        | ObjectPredicateDef::PowerGreaterThan(_)
        | ObjectPredicateDef::ToughnessGreaterThan(_)
        | ObjectPredicateDef::PowerLessThan(_)
        | ObjectPredicateDef::HasCounter(_)
        | ObjectPredicateDef::ControlledBy(_)
        | ObjectPredicateDef::Supertype(_)
        | ObjectPredicateDef::DebutSet(_)
        | ObjectPredicateDef::SharesNameWithSource
        | ObjectPredicateDef::AttackingOrBlocking
        | ObjectPredicateDef::HasKeyword(_)
        | ObjectPredicateDef::AttachedToSource
        | ObjectPredicateDef::Attacking
        | ObjectPredicateDef::Blocking
        | ObjectPredicateDef::BlockedBySource
        | ObjectPredicateDef::Enchanted
        | ObjectPredicateDef::AttackedThisTurn
        | ObjectPredicateDef::Special(_) => false,
    }
}

fn validate_trigger_object_reference(
    reference: ObjectRefDef,
    event: TriggerEventDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    validate_object_reference(reference, target_count, scope)?;
    if matches!(
        reference,
        ObjectRefDef::Source | ObjectRefDef::AttachedToSource | ObjectRefDef::TriggeringObject
    ) {
        Ok(())
    } else {
        Err(unsupported_trigger_event(event))
    }
}

fn validate_trigger_player_reference(
    reference: PlayerRefDef,
    event: TriggerEventDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    validate_player_reference(reference, target_count, scope)?;
    match reference {
        PlayerRefDef::EffectController | PlayerRefDef::EventPlayer => Ok(()),
        PlayerRefDef::ControllerOf(reference) | PlayerRefDef::OwnerOf(reference) => {
            validate_trigger_object_reference(reference, event, target_count, scope)
        }
        PlayerRefDef::Target(_) => Err(unsupported_trigger_event(event)),
    }
}

fn validate_trigger_player_set(
    players: PlayerSetDef,
    event: TriggerEventDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match players {
        PlayerSetDef::All | PlayerSetDef::Related(_) => Ok(()),
        PlayerSetDef::LegalTargets(_) => Err(unsupported_trigger_event(event)),
        PlayerSetDef::One(reference) => {
            validate_trigger_player_reference(reference, event, target_count, scope)
        }
    }
}

fn validate_trigger_damage_matcher(
    matcher: DamageEventMatcherDef,
    event: TriggerEventDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match matcher.source {
        DamageSourceMatcherDef::Any => {}
        // `AffectedObject` belongs to static prevention rules, whose applied
        // recipient is resolved outside an event. A triggered listener has no
        // such anchor and must name Source or another event reference.
        DamageSourceMatcherDef::AffectedObject => {
            return Err(unsupported_trigger_event(event));
        }
        DamageSourceMatcherDef::Matching(predicate) => {
            if trigger_predicate_requires_live_battlefield(predicate) {
                return Err(unsupported_trigger_event(event));
            }
            validate_trigger_object_predicate(predicate, event, target_count, scope)?;
        }
        DamageSourceMatcherDef::Object(reference) | DamageSourceMatcherDef::Except(reference) => {
            validate_trigger_object_reference(reference, event, target_count, scope)?;
        }
    }
    match matcher.recipient {
        DamageRecipientMatcherDef::Any => Ok(()),
        DamageRecipientMatcherDef::AffectedObject => Err(unsupported_trigger_event(event)),
        DamageRecipientMatcherDef::Recipients(EffectRecipientDef(
            EffectRecipientSetDef::Objects(ObjectSetDef::One(reference)),
        )) => validate_trigger_object_reference(reference, event, target_count, scope),
        DamageRecipientMatcherDef::Recipients(EffectRecipientDef(
            EffectRecipientSetDef::Players(players),
        )) => validate_trigger_player_set(players, event, target_count, scope),
        DamageRecipientMatcherDef::PlayerAndCreaturesControlledBy(player) => {
            validate_trigger_player_reference(player, event, target_count, scope)
        }
        DamageRecipientMatcherDef::Recipients(_) => Err(unsupported_trigger_event(event)),
    }
}

fn validate_trigger_event_references(
    event: TriggerEventDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match event {
        TriggerEventDef::ZoneChanged(matcher) => {
            const COMMITTED_TRANSITIONS: [(ZoneKind, ZoneKind); 9] = [
                (ZoneKind::Library, ZoneKind::Battlefield),
                (ZoneKind::Hand, ZoneKind::Battlefield),
                (ZoneKind::Graveyard, ZoneKind::Battlefield),
                (ZoneKind::Exile, ZoneKind::Battlefield),
                (ZoneKind::Stack, ZoneKind::Battlefield),
                (ZoneKind::Battlefield, ZoneKind::Graveyard),
                (ZoneKind::Battlefield, ZoneKind::Exile),
                (ZoneKind::Battlefield, ZoneKind::Hand),
                (ZoneKind::Battlefield, ZoneKind::Library),
            ];
            if !COMMITTED_TRANSITIONS.iter().any(|(from, to)| {
                matcher.from.is_none_or(|expected| expected == *from)
                    && matcher.to.is_none_or(|expected| expected == *to)
            }) {
                return Err(unsupported_trigger_event(event));
            }
            let can_match_departure = COMMITTED_TRANSITIONS.iter().any(|(from, to)| {
                *from == ZoneKind::Battlefield
                    && *to != ZoneKind::Battlefield
                    && matcher.from.is_none_or(|expected| expected == *from)
                    && matcher.to.is_none_or(|expected| expected == *to)
            });
            if can_match_departure && trigger_predicate_requires_live_battlefield(matcher.object) {
                return Err(unsupported_trigger_event(event));
            }
            validate_trigger_object_predicate(matcher.object, event, target_count, scope)?;
            if let Some(reference) = matcher.previously_damaged_by {
                if matcher
                    .from
                    .is_some_and(|from| from != ZoneKind::Battlefield)
                    || matcher.to.is_some_and(|to| to != ZoneKind::Graveyard)
                {
                    return Err(unsupported_trigger_event(event));
                }
                validate_trigger_object_reference(reference, event, target_count, scope)?;
            }
            Ok(())
        }
        TriggerEventDef::Tapped(matcher) => {
            validate_trigger_object_predicate(matcher.object, event, target_count, scope)
        }
        TriggerEventDef::Attacks(matcher) => {
            if matcher.declaration.minimum == 0
                || matcher
                    .declaration
                    .maximum
                    .is_some_and(|maximum| matcher.declaration.minimum > maximum)
                || matcher.attack_number == Some(0)
            {
                return Err(unsupported_trigger_event(event));
            }
            validate_trigger_object_predicate(matcher.attacker, event, target_count, scope)
        }
        TriggerEventDef::SpellCast(predicate)
            if trigger_predicate_requires_live_battlefield(predicate) =>
        {
            Err(unsupported_trigger_event(event))
        }
        TriggerEventDef::AttacksAndIsNotBlocked {
            attacker: predicate,
        }
        | TriggerEventDef::BecomesBlocked(predicate)
        | TriggerEventDef::BlocksOrBecomesBlockedBy { object: predicate }
        | TriggerEventDef::SpellCast(predicate)
        | TriggerEventDef::Transforms(predicate) => {
            validate_trigger_object_predicate(predicate, event, target_count, scope)
        }
        TriggerEventDef::DamageDealt(matcher) => {
            validate_trigger_damage_matcher(matcher, event, target_count, scope)
        }
        TriggerEventDef::LifeGained(PlayerRelation::ChosenPlayer) => {
            Err(unsupported_trigger_event(event))
        }
        TriggerEventDef::StepBegins { .. }
        | TriggerEventDef::LifeGained(_)
        | TriggerEventDef::StateCondition => Ok(()),
    }
}

fn validate_player_set(
    players: PlayerSetDef,
    target_count: usize,
    scope: BindingScope,
) -> Result<(), GrantedAbilityValidationError> {
    match players {
        PlayerSetDef::One(reference) => validate_player_reference(reference, target_count, scope),
        PlayerSetDef::LegalTargets(target) => validate_target_index(target, target_count),
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
        EffectRecipientSetDef::Objects(ObjectSetDef::LegalTargets(target)) => {
            validate_target_index(target, target_count)
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
        AppliedEffectDef::Rule(AppliedRuleDef::PreventDamage(matcher)) => {
            validate_damage_matcher_references(matcher, target_count, scope)
        }
        AppliedEffectDef::Rule(AppliedRuleDef::RedirectDamageFromTo {
            source,
            destination,
        }) => {
            validate_object_reference(source, target_count, scope)?;
            validate_object_reference(destination, target_count, scope)
        }
        // A granted ability introduces its own target scope and is validated
        // separately when the grant tree is traversed.
        AppliedEffectDef::Rule(_) | AppliedEffectDef::Characteristic(_) => Ok(()),
    }
}

fn validate_resolving_applied_effect(
    recipient: EffectRecipientDef,
    effect: AppliedEffectDef,
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        AppliedEffectDef::Composite(effects) => {
            if effects.is_empty() {
                return Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect);
            }
            for effect in effects {
                validate_resolving_applied_effect(recipient, *effect)?;
            }
            Ok(())
        }
        AppliedEffectDef::Rule(
            AppliedRuleDef::CannotPlay(_) | AppliedRuleDef::RedirectDamageFromTo { .. },
        ) => {
            if matches!(recipient.0, EffectRecipientSetDef::Objects(_)) {
                Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect)
            } else {
                Ok(())
            }
        }
        AppliedEffectDef::Rule(
            AppliedRuleDef::CannotBeCountered | AppliedRuleDef::PreventDamage(_),
        ) => Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect),
        AppliedEffectDef::Rule(_) | AppliedEffectDef::Characteristic(_) => {
            if matches!(recipient.0, EffectRecipientSetDef::Players(_)) {
                Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect)
            } else {
                Ok(())
            }
        }
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
        | EffectDef::RemoveAllCounters { object, .. }
        | EffectDef::Untap { object }
        | EffectDef::Attach { object }
        | EffectDef::Destroy { object, .. }
        | EffectDef::Sacrifice { object }
        | EffectDef::DiscardCards { object }
        | EffectDef::ChangeTextBasicLandType { object }
        | EffectDef::BecomeCopyOf { object, .. }
        | EffectDef::ExileLinkedToSource { object }
        | EffectDef::Detain { object }
        | EffectDef::GainControl { object, .. }
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
        EffectDef::SearchZone { player, .. }
        | EffectDef::ChooseCards { player, .. }
        | EffectDef::TakeExtraTurn { player }
        | EffectDef::LookAtHand { player } => {
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
                || definition.source_zones != [ZoneKind::Battlefield]
                || !definition.targets.is_empty()
                || trigger.ability.declarative_effect().is_none()
            {
                return Err(GrantedAbilityValidationError::UnsupportedInstalledTriggerAbility);
            }
            if definition.event == TriggerEventDef::StateCondition {
                return Err(unsupported_trigger_event(definition.event));
            }
            if let Some(condition) = definition.condition {
                validate_trigger_condition(*condition, target_count, scope)?;
            }
            validate_trigger_event_references(definition.event, target_count, scope)?;
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
        EffectDef::StaticApply { recipient, effect } => {
            validate_recipient_target_references(recipient, target_count, scope)?;
            validate_applied_effect_target_references(effect, target_count, scope)
        }
        EffectDef::Apply {
            recipient, effect, ..
        } => {
            validate_recipient_target_references(recipient, target_count, scope)?;
            validate_resolving_applied_effect(recipient, effect)?;
            validate_applied_effect_target_references(effect, target_count, scope)
        }
        // The chosen player is recorded on the permanent, not read from a
        // target slot.
        // A prohibition names a card shape, never a target.
        EffectDef::LandwalkCanBeBlocked(_)
        | EffectDef::CannotAttackUnless(_)
        | EffectDef::None
        | EffectDef::AddMana(_)
        | EffectDef::AddManaEqualTo { .. }
        | EffectDef::CreateEmblem { .. }
        | EffectDef::GrantFlashToNextSorcery
        | EffectDef::ReturnLinkedExiles { .. }
        | EffectDef::CannotBeForcedToSacrifice
        | EffectDef::ScheduleTurnPhases(_)
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

#[derive(Clone, Copy)]
enum RecipientExpectation {
    Any,
    Object,
    Player,
}

fn validate_program_target_shapes(
    program: AbilityProgramDef,
    targets: &[AbilityTargetDef],
    trigger_event: Option<TriggerEventDef>,
) -> Result<(), GrantedAbilityValidationError> {
    let triggering_object_zone = trigger_event.and_then(trigger_event_object_zone);
    match program {
        AbilityProgramDef::Effects(effect) => {
            validate_effect_target_shapes(effect, targets, triggering_object_zone)
        }
        AbilityProgramDef::Replacement(effect) => {
            validate_replacement_effect_target_shapes(effect, targets)
        }
    }
}

fn trigger_event_object_zone(event: TriggerEventDef) -> Option<ZoneKind> {
    match event {
        TriggerEventDef::ZoneChanged(matcher) => matcher.to,
        TriggerEventDef::Tapped(_)
        | TriggerEventDef::Attacks(_)
        | TriggerEventDef::AttacksAndIsNotBlocked { .. }
        | TriggerEventDef::BecomesBlocked(_)
        | TriggerEventDef::BlocksOrBecomesBlockedBy { .. }
        | TriggerEventDef::Transforms(_) => Some(ZoneKind::Battlefield),
        TriggerEventDef::SpellCast(_) => Some(ZoneKind::Stack),
        TriggerEventDef::StepBegins { .. }
        | TriggerEventDef::DamageDealt(_)
        | TriggerEventDef::StateCondition
        | TriggerEventDef::LifeGained(_) => None,
    }
}

fn target_matches_expectation(
    predicate: AbilityTargetPredicate,
    expected: RecipientExpectation,
) -> bool {
    match expected {
        RecipientExpectation::Any => true,
        RecipientExpectation::Object => matches!(
            predicate,
            AbilityTargetPredicate::Object { .. }
                | AbilityTargetPredicate::ControlledByTargetOf { .. }
        ),
        RecipientExpectation::Player => matches!(predicate, AbilityTargetPredicate::Player(_)),
    }
}

fn target_can_project(predicate: AbilityTargetPredicate, expected: RecipientExpectation) -> bool {
    match expected {
        RecipientExpectation::Any => true,
        RecipientExpectation::Object => !matches!(predicate, AbilityTargetPredicate::Player(_)),
        RecipientExpectation::Player => !matches!(
            predicate,
            AbilityTargetPredicate::Object { .. }
                | AbilityTargetPredicate::ControlledByTargetOf { .. }
        ),
    }
}

fn validate_target_projection(
    target: TargetIndex,
    targets: &[AbilityTargetDef],
    expected: RecipientExpectation,
) -> Result<(), GrantedAbilityValidationError> {
    let Some(definition) = targets.get(target.index()) else {
        return Err(GrantedAbilityValidationError::TargetReferenceOutOfBounds {
            target,
            target_count: targets.len(),
        });
    };
    if target_can_project(definition.predicate, expected) {
        Ok(())
    } else {
        Err(GrantedAbilityValidationError::TargetReferenceKindMismatch {
            target,
            predicate: definition.predicate,
            expected: public_subject_kind(expected),
        })
    }
}

fn public_subject_kind(expected: RecipientExpectation) -> EffectSubjectKind {
    match expected {
        RecipientExpectation::Object => EffectSubjectKind::Object,
        RecipientExpectation::Player => EffectSubjectKind::Player,
        RecipientExpectation::Any => unreachable!("an any-target expectation never errors"),
    }
}

fn validate_target_shape(
    target: TargetIndex,
    targets: &[AbilityTargetDef],
    expected: RecipientExpectation,
    singular: bool,
) -> Result<(), GrantedAbilityValidationError> {
    let Some(definition) = targets.get(target.index()) else {
        return Err(GrantedAbilityValidationError::TargetReferenceOutOfBounds {
            target,
            target_count: targets.len(),
        });
    };
    if singular && definition.maximum > 1 {
        return Err(
            GrantedAbilityValidationError::TargetReferenceRequiresSingular {
                target,
                maximum: definition.maximum,
            },
        );
    }
    if !target_matches_expectation(definition.predicate, expected) {
        return Err(GrantedAbilityValidationError::TargetReferenceKindMismatch {
            target,
            predicate: definition.predicate,
            expected: public_subject_kind(expected),
        });
    }
    Ok(())
}

fn validate_object_reference_shape(
    reference: ObjectRefDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    if let ObjectRefDef::Target(target) = reference {
        validate_target_shape(target, targets, RecipientExpectation::Object, true)?;
    }
    Ok(())
}

fn validate_player_reference_shape(
    reference: PlayerRefDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    match reference {
        PlayerRefDef::Target(target) => {
            validate_target_shape(target, targets, RecipientExpectation::Player, true)
        }
        // The runtime can derive a controller from both halves of an any-target
        // slot: a player is their own controller, while an object has one.
        PlayerRefDef::ControllerOf(ObjectRefDef::Target(target)) => {
            validate_target_shape(target, targets, RecipientExpectation::Any, true)
        }
        // Players have no owner, so this derived reference requires an
        // object-only target even though ControllerOf can also consume a
        // player directly. Merely projecting an any-target slot is not enough:
        // the selected member could still be a player and silently produce no
        // owner at resolution.
        PlayerRefDef::OwnerOf(ObjectRefDef::Target(target)) => {
            validate_target_shape(target, targets, RecipientExpectation::Object, true)
        }
        PlayerRefDef::ControllerOf(reference) | PlayerRefDef::OwnerOf(reference) => {
            validate_object_reference_shape(reference, targets)
        }
        PlayerRefDef::EffectController | PlayerRefDef::EventPlayer => Ok(()),
    }
}

fn validate_player_set_shape(
    players: PlayerSetDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    match players {
        PlayerSetDef::One(reference) => validate_player_reference_shape(reference, targets)?,
        PlayerSetDef::LegalTargets(target) => {
            validate_target_projection(target, targets, RecipientExpectation::Player)?;
        }
        PlayerSetDef::All | PlayerSetDef::Related(_) => {}
    }
    Ok(())
}

fn validate_query_shape(
    query: ObjectQueryDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    for players in [query.controller, query.owner, query.related_player]
        .into_iter()
        .flatten()
    {
        validate_player_set_shape(players, targets)?;
    }
    validate_object_predicate_shape(query.object, targets)
}

fn validate_object_set_shape(
    objects: ObjectSetDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    match objects {
        ObjectSetDef::One(reference) | ObjectSetDef::SharingNameWith(reference) => {
            validate_object_reference_shape(reference, targets)
        }
        ObjectSetDef::Query(query) => validate_query_shape(query, targets),
        ObjectSetDef::LegalTargets(target) => {
            validate_target_projection(target, targets, RecipientExpectation::Object)
        }
        ObjectSetDef::Binding(_) => Ok(()),
    }
}

fn validate_recipient_shape(
    recipient: EffectRecipientDef,
    targets: &[AbilityTargetDef],
    expected: RecipientExpectation,
) -> Result<(), GrantedAbilityValidationError> {
    match recipient.0 {
        EffectRecipientSetDef::LegalTargets(target) => {
            validate_target_shape(target, targets, expected, false)
        }
        EffectRecipientSetDef::Objects(objects) => {
            if matches!(expected, RecipientExpectation::Player) {
                return Err(GrantedAbilityValidationError::EffectRecipientKindMismatch {
                    recipient,
                    expected: EffectSubjectKind::Player,
                });
            }
            validate_object_set_shape(objects, targets)
        }
        EffectRecipientSetDef::Players(players) => {
            if matches!(expected, RecipientExpectation::Object) {
                return Err(GrantedAbilityValidationError::EffectRecipientKindMismatch {
                    recipient,
                    expected: EffectSubjectKind::Object,
                });
            }
            validate_player_set_shape(players, targets)
        }
    }
}

fn validate_value_shape(
    value: ValueDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    match value {
        ValueDef::Negate(value) => validate_value_shape(*value, targets),
        ValueDef::Scaled(value) => validate_value_shape(value.value, targets),
        ValueDef::IfCreatureDiedThisTurn(value) => {
            validate_value_shape(value.then, targets)?;
            validate_value_shape(value.otherwise, targets)
        }
        ValueDef::IfTargetMatches(value) => {
            validate_target_shape(value.slot, targets, RecipientExpectation::Object, false)?;
            validate_value_shape(value.then, targets)?;
            validate_value_shape(value.otherwise, targets)
        }
        ValueDef::IfMatchingObjectCount(value) => {
            validate_query_shape(value.query, targets)?;
            validate_value_shape(value.then, targets)?;
            validate_value_shape(value.otherwise, targets)
        }
        ValueDef::CountMatchingObjects(query) | ValueDef::AnyMatchingObject(query) => {
            validate_query_shape(*query, targets)
        }
        ValueDef::TargetPower(target) | ValueDef::TargetManaValue(target) => {
            validate_target_shape(target, targets, RecipientExpectation::Object, true)
        }
        ValueDef::Constant(_)
        | ValueDef::ChosenX
        | ValueDef::SourcePower
        | ValueDef::SourceToughness
        | ValueDef::TriggeringObjectPower
        | ValueDef::TriggerEventAmount
        | ValueDef::CardsInHandAbove { .. }
        | ValueDef::CountersOnSource(_)
        | ValueDef::DividedAmongTargets => Ok(()),
    }
}

fn validate_object_predicate_shape(
    predicate: ObjectPredicateDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    match predicate {
        ObjectPredicateDef::All(predicates) | ObjectPredicateDef::AnyOf(predicates) => {
            for predicate in predicates {
                validate_object_predicate_shape(*predicate, targets)?;
            }
            Ok(())
        }
        ObjectPredicateDef::Not(predicate) | ObjectPredicateDef::AttachedTo(predicate) => {
            validate_object_predicate_shape(*predicate, targets)
        }
        ObjectPredicateDef::ManaValueEqualTo(value)
        | ObjectPredicateDef::ManaValueAtMostValue(value)
        | ObjectPredicateDef::ToughnessLessThan(value)
        | ObjectPredicateDef::PowerGreaterThan(value)
        | ObjectPredicateDef::ToughnessGreaterThan(value)
        | ObjectPredicateDef::PowerLessThan(value) => validate_value_shape(value, targets),
        _ => Ok(()),
    }
}

fn validate_damage_matcher_shape(
    matcher: DamageEventMatcherDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    match matcher.source {
        DamageSourceMatcherDef::Object(reference) | DamageSourceMatcherDef::Except(reference) => {
            validate_object_reference_shape(reference, targets)?;
        }
        DamageSourceMatcherDef::Matching(predicate) => {
            validate_object_predicate_shape(predicate, targets)?;
        }
        DamageSourceMatcherDef::Any
        | DamageSourceMatcherDef::Group(_)
        | DamageSourceMatcherDef::AffectedObject => {}
    }
    match matcher.recipient {
        DamageRecipientMatcherDef::Recipients(recipient) => {
            validate_recipient_shape(recipient, targets, RecipientExpectation::Any)
        }
        DamageRecipientMatcherDef::PlayerAndCreaturesControlledBy(player) => {
            validate_player_reference_shape(player, targets)
        }
        DamageRecipientMatcherDef::Any | DamageRecipientMatcherDef::AffectedObject => Ok(()),
    }
}

fn validate_condition_shape(
    condition: ConditionDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    match condition {
        ConditionDef::Exists(query) => validate_query_shape(query, targets),
    }
}

fn validate_trigger_condition_shape(
    condition: TriggerConditionDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    match condition {
        TriggerConditionDef::ObjectCount { query, .. } => validate_query_shape(query, targets),
        TriggerConditionDef::AttachedPermanentMatches { object } => {
            validate_object_predicate_shape(object, targets)
        }
        TriggerConditionDef::TargetMatches { slot, object } => {
            validate_target_shape(slot, targets, RecipientExpectation::Object, false)?;
            validate_object_predicate_shape(object, targets)
        }
        TriggerConditionDef::SourceOnBattlefield
        | TriggerConditionDef::SourceUntapped
        | TriggerConditionDef::ActivePlayer(_)
        | TriggerConditionDef::SpellsCastLastTurn { .. }
        | TriggerConditionDef::SourceLoyalty { .. }
        | TriggerConditionDef::SourceActivationsThisTurn { .. }
        | TriggerConditionDef::SourceDealtDamageToOpponentThisTurn
        | TriggerConditionDef::SourceIsTapped
        | TriggerConditionDef::ControlsGreatestPowerCreature
        | TriggerConditionDef::SourceCounters { .. } => Ok(()),
    }
}

fn validate_payment_shape(
    payment: EffectPaymentDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    validate_player_set_shape(payment.payer, targets)?;
    if let PlayerSetDef::LegalTargets(target) = payment.payer {
        validate_target_shape(target, targets, RecipientExpectation::Any, true)?;
        validate_target_projection(target, targets, RecipientExpectation::Player)?;
    }
    if let EffectPaymentCostDef::GenericMana(amount) = payment.cost {
        validate_value_shape(amount, targets)?;
    }
    Ok(())
}

fn applied_effect_adds_ability(effect: AppliedEffectDef) -> bool {
    match effect {
        AppliedEffectDef::Composite(effects) => {
            effects.iter().copied().any(applied_effect_adds_ability)
        }
        AppliedEffectDef::Characteristic(CharacteristicOperationDef::Abilities(
            AbilityOperationDef::Add(_),
        )) => true,
        AppliedEffectDef::Characteristic(_) | AppliedEffectDef::Rule(_) => false,
    }
}

fn nonbattlefield_ability_grants_are_supported(effect: AppliedEffectDef) -> bool {
    match effect {
        AppliedEffectDef::Composite(effects) => effects
            .iter()
            .copied()
            .all(nonbattlefield_ability_grants_are_supported),
        AppliedEffectDef::Characteristic(CharacteristicOperationDef::Abilities(
            AbilityOperationDef::Add(ability),
        )) => {
            ability.is_executable()
                && matches!(
                    ability.definition,
                    DeclarativeAbilityDef::AlternativeCast(definition)
                        if definition.kind == AlternativeCastKindDef::Flashback
                )
        }
        AppliedEffectDef::Characteristic(_) | AppliedEffectDef::Rule(_) => true,
    }
}

fn target_may_name_nonbattlefield_object(
    target: TargetIndex,
    targets: &[AbilityTargetDef],
) -> bool {
    targets.get(target.index()).is_none_or(|definition| {
        matches!(
            definition.predicate,
            AbilityTargetPredicate::Object { zones, .. }
                if zones.iter().any(|zone| *zone != ZoneKind::Battlefield)
        )
    })
}

fn recipient_may_name_nonbattlefield_object(
    recipient: EffectRecipientDef,
    targets: &[AbilityTargetDef],
    triggering_object_zone: Option<ZoneKind>,
) -> bool {
    match recipient.0 {
        EffectRecipientSetDef::LegalTargets(target)
        | EffectRecipientSetDef::Objects(ObjectSetDef::LegalTargets(target))
        | EffectRecipientSetDef::Objects(ObjectSetDef::One(ObjectRefDef::Target(target))) => {
            target_may_name_nonbattlefield_object(target, targets)
        }
        EffectRecipientSetDef::Objects(ObjectSetDef::Query(query)) => query
            .zones
            .iter()
            .any(|zone| *zone != ZoneKind::Battlefield),
        EffectRecipientSetDef::Objects(
            ObjectSetDef::One(ObjectRefDef::Binding(_)) | ObjectSetDef::Binding(_),
        ) => true,
        EffectRecipientSetDef::Objects(ObjectSetDef::One(ObjectRefDef::TriggeringObject)) => {
            triggering_object_zone != Some(ZoneKind::Battlefield)
        }
        EffectRecipientSetDef::Objects(
            ObjectSetDef::One(
                ObjectRefDef::Source
                | ObjectRefDef::ResolvingObject
                | ObjectRefDef::AttachedToSource,
            )
            | ObjectSetDef::SharingNameWith(_),
        )
        | EffectRecipientSetDef::Players(_) => false,
    }
}

fn recipient_nonbattlefield_zones_support_flashback(
    recipient: EffectRecipientDef,
    targets: &[AbilityTargetDef],
    triggering_object_zone: Option<ZoneKind>,
) -> bool {
    let supported_zones = |zones: &[ZoneKind]| {
        zones
            .iter()
            .all(|zone| matches!(zone, ZoneKind::Battlefield | ZoneKind::Graveyard))
    };
    match recipient.0 {
        EffectRecipientSetDef::LegalTargets(target)
        | EffectRecipientSetDef::Objects(ObjectSetDef::LegalTargets(target))
        | EffectRecipientSetDef::Objects(ObjectSetDef::One(ObjectRefDef::Target(target))) => {
            targets.get(target.index()).is_some_and(|definition| {
                matches!(
                    definition.predicate,
                    AbilityTargetPredicate::Object { zones, .. } if supported_zones(zones)
                )
            })
        }
        EffectRecipientSetDef::Objects(ObjectSetDef::Query(query)) => supported_zones(query.zones),
        EffectRecipientSetDef::Objects(ObjectSetDef::One(ObjectRefDef::TriggeringObject)) => {
            matches!(
                triggering_object_zone,
                Some(ZoneKind::Battlefield | ZoneKind::Graveyard)
            )
        }
        EffectRecipientSetDef::Objects(
            ObjectSetDef::One(ObjectRefDef::Binding(_)) | ObjectSetDef::Binding(_),
        ) => false,
        EffectRecipientSetDef::Objects(
            ObjectSetDef::One(
                ObjectRefDef::Source
                | ObjectRefDef::ResolvingObject
                | ObjectRefDef::AttachedToSource,
            )
            | ObjectSetDef::SharingNameWith(_),
        )
        | EffectRecipientSetDef::Players(_) => true,
    }
}

fn validate_applied_effect_shapes(
    recipient: EffectRecipientDef,
    effect: AppliedEffectDef,
    targets: &[AbilityTargetDef],
    static_effect: bool,
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        AppliedEffectDef::Composite(effects) => {
            for effect in effects {
                validate_applied_effect_shapes(recipient, *effect, targets, static_effect)?;
            }
            Ok(())
        }
        AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(restriction)) => {
            validate_recipient_shape(recipient, targets, RecipientExpectation::Player)?;
            validate_object_predicate_shape(restriction.object, targets)?;
            if static_effect
                && !matches!(
                    recipient.0,
                    EffectRecipientSetDef::Players(
                        PlayerSetDef::All
                            | PlayerSetDef::Related(
                                PlayerRelation::Any
                                    | PlayerRelation::You
                                    | PlayerRelation::NotYou
                                    | PlayerRelation::Opponent
                                    | PlayerRelation::ActivePlayer
                                    | PlayerRelation::NonactivePlayer
                            )
                            | PlayerSetDef::One(PlayerRefDef::EffectController)
                    )
                )
            {
                return Err(
                    GrantedAbilityValidationError::UnsupportedStaticPlayerRecipient { recipient },
                );
            }
            Ok(())
        }
        AppliedEffectDef::Rule(AppliedRuleDef::PreventDamage(matcher)) => {
            validate_recipient_shape(recipient, targets, RecipientExpectation::Object)?;
            validate_damage_matcher_shape(matcher, targets)
        }
        AppliedEffectDef::Rule(AppliedRuleDef::RedirectDamageFromTo {
            source,
            destination,
        }) => {
            validate_recipient_shape(recipient, targets, RecipientExpectation::Player)?;
            validate_object_reference_shape(source, targets)?;
            validate_object_reference_shape(destination, targets)
        }
        AppliedEffectDef::Characteristic(CharacteristicOperationDef::PowerToughness(
            PowerToughnessOperationDef::SetBase { power, toughness }
            | PowerToughnessOperationDef::Modify { power, toughness },
        )) => {
            validate_recipient_shape(recipient, targets, RecipientExpectation::Object)?;
            validate_value_shape(power, targets)?;
            validate_value_shape(toughness, targets)
        }
        AppliedEffectDef::Rule(_) | AppliedEffectDef::Characteristic(_) => {
            validate_recipient_shape(recipient, targets, RecipientExpectation::Object)
        }
    }
}

#[allow(clippy::too_many_lines)]
fn validate_effect_target_shapes(
    effect: EffectDef,
    targets: &[AbilityTargetDef],
    triggering_object_zone: Option<ZoneKind>,
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        EffectDef::Sequence(effects) => {
            for effect in effects {
                validate_effect_target_shapes(*effect, targets, triggering_object_zone)?;
            }
            Ok(())
        }
        EffectDef::Randomized {
            on_success,
            on_failure,
            ..
        } => {
            validate_effect_target_shapes(*on_success, targets, triggering_object_zone)?;
            validate_effect_target_shapes(*on_failure, targets, triggering_object_zone)
        }
        EffectDef::Choose(choice) => {
            validate_player_reference_shape(choice.chooser, targets)?;
            validate_object_set_shape(choice.candidates, targets)?;
            if let Some(excluded) = choice.exclude {
                validate_object_reference_shape(excluded, targets)?;
            }
            validate_effect_target_shapes(*choice.then, targets, triggering_object_zone)
        }
        EffectDef::PayOr(payment) => {
            validate_payment_shape(payment.payment, targets)?;
            for effect in payment.if_paid.iter().chain(payment.otherwise.iter()) {
                validate_effect_target_shapes(**effect, targets, triggering_object_zone)?;
            }
            Ok(())
        }
        EffectDef::SplitIntoPiles(partition) => {
            validate_player_set_shape(partition.divider, targets)?;
            validate_player_set_shape(partition.chooser, targets)?;
            match partition.items {
                crate::card::PartitionItemsDef::Objects(objects) => {
                    validate_object_set_shape(objects, targets)?;
                }
                crate::card::PartitionItemsDef::TopOfLibrary { player, count } => {
                    validate_player_reference_shape(player, targets)?;
                    validate_value_shape(count, targets)?;
                }
            }
            validate_effect_target_shapes(*partition.then, targets, triggering_object_zone)
        }
        EffectDef::PreventDamage { prevention, .. } => {
            validate_damage_matcher_shape(prevention.matcher, targets)?;
            if let DamagePreventionCapacityDef::Amount(amount) = prevention.capacity {
                validate_value_shape(amount, targets)?;
            }
            Ok(())
        }
        EffectDef::DealDamage { recipient, amount }
        | EffectDef::DrainLife { recipient, amount } => {
            validate_recipient_shape(recipient, targets, RecipientExpectation::Any)?;
            validate_value_shape(amount, targets)
        }
        EffectDef::GainLife { recipient, amount }
        | EffectDef::AddPoisonCounters { recipient, amount }
        | EffectDef::DrawCards { recipient, amount }
        | EffectDef::Discard {
            recipient, amount, ..
        }
        | EffectDef::LoseLife { recipient, amount } => {
            validate_recipient_shape(recipient, targets, RecipientExpectation::Player)?;
            validate_value_shape(amount, targets)
        }
        EffectDef::ShuffleLibrary { player }
        | EffectDef::EmptyManaPool { player }
        | EffectDef::LoseTheGame { player }
        | EffectDef::SearchZone { player, .. }
        | EffectDef::ChooseCards { player, .. }
        | EffectDef::TakeExtraTurn { player }
        | EffectDef::LookAtHand { player } => {
            validate_recipient_shape(player, targets, RecipientExpectation::Player)
        }
        EffectDef::SacrificeOfChoice {
            player,
            object,
            then,
            ..
        } => {
            validate_recipient_shape(player, targets, RecipientExpectation::Player)?;
            validate_object_predicate_shape(object, targets)?;
            if let Some(effect) = then {
                validate_effect_target_shapes(*effect, targets, triggering_object_zone)?;
            }
            Ok(())
        }
        EffectDef::LookAtTopAndSelect { player, selection } => {
            validate_recipient_shape(player, targets, RecipientExpectation::Player)?;
            if selection.minimum > selection.maximum
                || !matches!(
                    selection.selected_zone,
                    ZoneKind::Hand | ZoneKind::Library | ZoneKind::Graveyard | ZoneKind::Exile
                )
                || !matches!(
                    selection.rest_zone,
                    ZoneKind::Hand | ZoneKind::Library | ZoneKind::Graveyard | ZoneKind::Exile
                )
            {
                return Err(
                    GrantedAbilityValidationError::UnsupportedEffectProgramContext {
                        context: "resolving",
                        operation: "LookAtTopAndSelect with invalid bounds or unsupported destination zones",
                    },
                );
            }
            validate_value_shape(selection.count, targets)?;
            if let Some(predicate) = selection.object {
                validate_object_predicate_shape(predicate, targets)?;
            }
            if let Some(effect) = selection.then {
                validate_effect_target_shapes(*effect, targets, triggering_object_zone)?;
            }
            Ok(())
        }
        EffectDef::May { player, effect }
        | EffectDef::ReplaceNextDrawThisTurn { player, effect } => {
            validate_recipient_shape(player, targets, RecipientExpectation::Player)?;
            validate_effect_target_shapes(*effect, targets, triggering_object_zone)
        }
        EffectDef::Mill { player, amount } => {
            validate_recipient_shape(player, targets, RecipientExpectation::Player)?;
            validate_value_shape(amount, targets)
        }
        EffectDef::DiscardCards { object }
        | EffectDef::Regenerate { object }
        | EffectDef::Tap { object }
        | EffectDef::Untap { object }
        | EffectDef::Attach { object }
        | EffectDef::Destroy { object, .. }
        | EffectDef::DestroyAtEndOfCombat { object }
        | EffectDef::Detain { object }
        | EffectDef::RemoveAllCounters { object, .. }
        | EffectDef::SkipNextUntapSteps { object, .. }
        | EffectDef::Sacrifice { object }
        | EffectDef::ChangeTextBasicLandType { object }
        | EffectDef::BecomeCopyOf { object, .. }
        | EffectDef::ExileLinkedToSource { object }
        | EffectDef::GainControl { object, .. }
        | EffectDef::Transform { object }
        | EffectDef::MoveToZone { object, .. }
        | EffectDef::Counter { object, .. } => {
            validate_recipient_shape(object, targets, RecipientExpectation::Object)
        }
        EffectDef::AddCounters { object, amount, .. } => {
            validate_recipient_shape(object, targets, RecipientExpectation::Object)?;
            validate_value_shape(amount, targets)
        }
        EffectDef::CreateToken { count, .. }
        | EffectDef::ReduceGenericCostBy(count)
        | EffectDef::AddManaEqualTo { amount: count, .. } => validate_value_shape(count, targets),
        EffectDef::CreateTokenCopyOf { object } => {
            validate_recipient_shape(object, targets, RecipientExpectation::Object)
        }
        EffectDef::IfCondition { condition, then } => {
            validate_trigger_condition_shape(*condition, targets)?;
            validate_effect_target_shapes(*then, targets, triggering_object_zone)
        }
        EffectDef::IfFormat {
            then, otherwise, ..
        } => {
            validate_effect_target_shapes(*then, targets, triggering_object_zone)?;
            validate_effect_target_shapes(*otherwise, targets, triggering_object_zone)
        }
        EffectDef::InstallTrigger(trigger) => {
            if let crate::card::InstalledTriggerLifetimeDef::UntilNextTurn(player) =
                trigger.lifetime
            {
                validate_player_reference_shape(player, targets)?;
            }
            let trigger_event = match trigger.ability.definition {
                DeclarativeAbilityDef::Triggered(definition)
                | DeclarativeAbilityDef::TriggeredMana(definition) => Some(definition.event),
                DeclarativeAbilityDef::Spell(_)
                | DeclarativeAbilityDef::ActivatedMana(_)
                | DeclarativeAbilityDef::Activated(_)
                | DeclarativeAbilityDef::Static(_)
                | DeclarativeAbilityDef::Replacement(_)
                | DeclarativeAbilityDef::AlternativeCast(_)
                | DeclarativeAbilityDef::SpecialAction(_)
                | DeclarativeAbilityDef::Keyword(_)
                | DeclarativeAbilityDef::Legacy => None,
            };
            validate_program_target_shapes(
                trigger.ability.effect.definition,
                targets,
                trigger_event,
            )
        }
        EffectDef::CannotAttackUnless(query) => validate_query_shape(*query, targets),
        EffectDef::StaticApply { recipient, effect } => {
            validate_applied_effect_shapes(recipient, effect, targets, true)
        }
        EffectDef::Apply {
            recipient,
            effect,
            duration,
        } => {
            validate_applied_effect_shapes(recipient, effect, targets, false)?;
            if applied_effect_adds_ability(effect)
                && recipient_may_name_nonbattlefield_object(
                    recipient,
                    targets,
                    triggering_object_zone,
                )
            {
                if duration != ResolvedEffectDurationDef::UntilEndOfTurn
                    || !nonbattlefield_ability_grants_are_supported(effect)
                    || !recipient_nonbattlefield_zones_support_flashback(
                        recipient,
                        targets,
                        triggering_object_zone,
                    )
                {
                    return Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect);
                }
            }
            Ok(())
        }
        EffectDef::None
        | EffectDef::AddMana(_)
        | EffectDef::GrantFlashToNextSorcery
        | EffectDef::ReturnLinkedExiles { .. }
        | EffectDef::CannotBeForcedToSacrifice
        | EffectDef::LandwalkCanBeBlocked(_)
        | EffectDef::ScheduleTurnPhases(_)
        | EffectDef::CreateEmblem { .. }
        | EffectDef::Special(_) => Ok(()),
    }
}

fn validate_replacement_effect_target_shapes(
    effect: ReplacementEffectDef,
    targets: &[AbilityTargetDef],
) -> Result<(), GrantedAbilityValidationError> {
    match effect {
        ReplacementEffectDef::Sequence(effects) => {
            for effect in effects {
                validate_replacement_effect_target_shapes(*effect, targets)?;
            }
            Ok(())
        }
        ReplacementEffectDef::Conditional {
            condition,
            if_true,
            if_false,
        } => {
            validate_condition_shape(condition, targets)?;
            for effect in if_true.iter().chain(if_false.iter()) {
                validate_replacement_effect_target_shapes(*effect, targets)?;
            }
            Ok(())
        }
        ReplacementEffectDef::PayOr {
            payment,
            if_paid,
            if_declined,
        } => {
            validate_payment_shape(payment, targets)?;
            for effect in if_paid.iter().chain(if_declined.iter()) {
                validate_replacement_effect_target_shapes(*effect, targets)?;
            }
            Ok(())
        }
        ReplacementEffectDef::Perform(effect) => {
            validate_effect_target_shapes(*effect, targets, None)
        }
        ReplacementEffectDef::Choose(ReplacementChoiceDef::Scalar(choice)) => {
            if matches!(
                (choice.list, choice.destination),
                (
                    ScalarChoiceListDef::CardNames,
                    BattlefieldEntryChoiceDestinationDef::CardName
                ) | (
                    ScalarChoiceListDef::CreatureTypes,
                    BattlefieldEntryChoiceDestinationDef::CreatureType
                )
            ) {
                Ok(())
            } else {
                Err(GrantedAbilityValidationError::InvalidScalarChoice {
                    list: choice.list,
                    destination: choice.destination,
                })
            }
        }
        ReplacementEffectDef::ReplaceEventWithNothing
        | ReplacementEffectDef::MoveToZone(_)
        | ReplacementEffectDef::ModifyBattlefieldEntry(_)
        | ReplacementEffectDef::MultiplyEventAmount(_)
        | ReplacementEffectDef::Choose(ReplacementChoiceDef::Player(_))
        | ReplacementEffectDef::CopyEntering { .. } => Ok(()),
    }
}

#[cfg(test)]
mod recipient_shape_tests {
    use super::*;
    use crate::card::{
        BattlefieldEntryScalarChoiceDef, PlayActionMatcherDef, PlayRestrictionDef,
        ResolvedEffectDurationDef, TopCardSelectionDef, ZonePlacement,
    };

    const PLAYER_TARGET: AbilityTargetDef =
        AbilityTargetDef::exactly_one(AbilityTargetPredicate::Player(PlayerRelation::Any));
    const OBJECT_TARGET: AbilityTargetDef =
        AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::Any,
            zones: &[ZoneKind::Battlefield],
            controller: None,
            owner: None,
        });
    const ANY_TARGET: AbilityTargetDef =
        AbilityTargetDef::exactly_one(AbilityTargetPredicate::AnyTarget);

    fn cannot_play() -> AppliedEffectDef {
        AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(PlayRestrictionDef::new(
            PlayActionMatcherDef::CastSpell,
            ObjectPredicateDef::NoncreatureSpell,
        )))
    }

    #[test]
    fn object_and_player_effects_reject_opposite_typed_recipients() {
        assert_eq!(
            validate_ability_targets(
                &[],
                EffectDef::Tap {
                    object: EffectRecipientDef::Controller,
                },
            ),
            Err(GrantedAbilityValidationError::EffectRecipientKindMismatch {
                recipient: EffectRecipientDef::Controller,
                expected: EffectSubjectKind::Object,
            }),
        );
        assert_eq!(
            validate_ability_targets(
                &[],
                EffectDef::GainLife {
                    recipient: EffectRecipientDef::Source,
                    amount: ValueDef::Constant(1),
                },
            ),
            Err(GrantedAbilityValidationError::EffectRecipientKindMismatch {
                recipient: EffectRecipientDef::Source,
                expected: EffectSubjectKind::Player,
            }),
        );
    }

    #[test]
    fn target_slots_must_contain_the_subject_kind_an_effect_reads() {
        assert_eq!(
            validate_ability_targets(
                &[PLAYER_TARGET],
                EffectDef::Tap {
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                },
            ),
            Err(GrantedAbilityValidationError::TargetReferenceKindMismatch {
                target: TargetIndex::PRIMARY,
                predicate: PLAYER_TARGET.predicate,
                expected: EffectSubjectKind::Object,
            }),
        );
        assert_eq!(
            validate_ability_targets(
                &[OBJECT_TARGET],
                EffectDef::GainLife {
                    recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    amount: ValueDef::Constant(1),
                },
            ),
            Err(GrantedAbilityValidationError::TargetReferenceKindMismatch {
                target: TargetIndex::PRIMARY,
                predicate: OBJECT_TARGET.predicate,
                expected: EffectSubjectKind::Player,
            }),
        );
    }

    #[test]
    fn typed_projections_make_mixed_target_filtering_explicit() {
        let effects = Box::leak(Box::new([
            EffectDef::Tap {
                object: EffectRecipientDef::target_objects(TargetIndex::PRIMARY),
            },
            EffectDef::Apply {
                recipient: EffectRecipientDef::target_players(TargetIndex::PRIMARY),
                effect: cannot_play(),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        ]));
        validate_ability_targets(&[ANY_TARGET], EffectDef::Sequence(effects))
            .expect("typed projections retain both halves of an any-target slot");
    }

    #[test]
    fn raw_target_references_require_at_most_one_selected_target() {
        let targets = [AbilityTargetDef::up_to(OBJECT_TARGET.predicate, 2)];
        assert_eq!(
            validate_ability_targets(
                &targets,
                EffectDef::Tap {
                    object: EffectRecipientDef::object(ObjectRefDef::Target(TargetIndex::PRIMARY,)),
                },
            ),
            Err(
                GrantedAbilityValidationError::TargetReferenceRequiresSingular {
                    target: TargetIndex::PRIMARY,
                    maximum: 2,
                },
            ),
        );
    }

    #[test]
    fn derived_controller_accepts_mixed_targets_but_owner_requires_an_object() {
        validate_ability_targets(
            &[ANY_TARGET],
            EffectDef::GainLife {
                recipient: EffectRecipientDef::player(PlayerRefDef::ControllerOf(
                    ObjectRefDef::Target(TargetIndex::PRIMARY),
                )),
                amount: ValueDef::Constant(1),
            },
        )
        .expect("a player is its own controller, so either half is meaningful");

        validate_ability_targets(
            &[OBJECT_TARGET],
            EffectDef::GainLife {
                recipient: EffectRecipientDef::player(PlayerRefDef::OwnerOf(ObjectRefDef::Target(
                    TargetIndex::PRIMARY,
                ))),
                amount: ValueDef::Constant(1),
            },
        )
        .expect("an object target always has an owner");

        for target in [ANY_TARGET, PLAYER_TARGET] {
            assert_eq!(
                validate_ability_targets(
                    &[target],
                    EffectDef::GainLife {
                        recipient: EffectRecipientDef::player(PlayerRefDef::OwnerOf(
                            ObjectRefDef::Target(TargetIndex::PRIMARY),
                        )),
                        amount: ValueDef::Constant(1),
                    },
                ),
                Err(GrantedAbilityValidationError::TargetReferenceKindMismatch {
                    target: TargetIndex::PRIMARY,
                    predicate: target.predicate,
                    expected: EffectSubjectKind::Object,
                }),
            );
        }
    }

    #[test]
    fn scalar_entry_choices_reject_mismatched_lists_and_destinations() {
        let choice = BattlefieldEntryScalarChoiceDef {
            list: ScalarChoiceListDef::CardNames,
            destination: BattlefieldEntryChoiceDestinationDef::CreatureType,
        };
        assert_eq!(
            validate_replacement_ability_targets(
                &[],
                ReplacementEffectDef::Choose(ReplacementChoiceDef::Scalar(choice)),
            ),
            Err(GrantedAbilityValidationError::InvalidScalarChoice {
                list: choice.list,
                destination: choice.destination,
            }),
        );
    }

    #[test]
    fn top_card_selections_reject_invalid_bounds_and_unsupported_destinations() {
        static INVALID_BOUNDS: TopCardSelectionDef = TopCardSelectionDef {
            count: ValueDef::Constant(2),
            object: None,
            minimum: 2,
            maximum: 1,
            reveal_selected: false,
            selected_zone: ZoneKind::Hand,
            selected_placement: ZonePlacement::Top,
            rest_zone: ZoneKind::Library,
            rest_placement: ZonePlacement::Bottom,
            then: None,
        };
        static INVALID_ZONE: TopCardSelectionDef = TopCardSelectionDef {
            count: ValueDef::Constant(1),
            object: None,
            minimum: 0,
            maximum: 1,
            reveal_selected: false,
            selected_zone: ZoneKind::Battlefield,
            selected_placement: ZonePlacement::Top,
            rest_zone: ZoneKind::Library,
            rest_placement: ZonePlacement::Bottom,
            then: None,
        };

        for selection in [&INVALID_BOUNDS, &INVALID_ZONE] {
            assert_eq!(
                validate_ability_targets(
                    &[],
                    EffectDef::LookAtTopAndSelect {
                        player: EffectRecipientDef::Controller,
                        selection,
                    },
                ),
                Err(
                    GrantedAbilityValidationError::UnsupportedEffectProgramContext {
                        context: "resolving",
                        operation: "LookAtTopAndSelect with invalid bounds or unsupported destination zones",
                    }
                ),
            );
        }
    }

    #[test]
    fn static_player_rules_reject_event_only_selectors() {
        let recipient = EffectRecipientDef::player(PlayerRefDef::EventPlayer);
        assert_eq!(
            validate_ability_targets(
                &[],
                EffectDef::StaticApply {
                    recipient,
                    effect: cannot_play(),
                },
            ),
            Err(GrantedAbilityValidationError::UnsupportedStaticPlayerRecipient { recipient },),
        );
    }
}
