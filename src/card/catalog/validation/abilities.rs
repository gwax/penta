use super::program_context::validate_ability_effect_context;
use super::targeting::{validate_ability_program_targets, validate_ability_trigger_event};
use crate::card::catalog::{
    CatalogError, GrantedAbilityValidationError, MismatchedAlternativeCost,
};
use crate::card::{
    AbilityDef, AbilityOperationDef, AbilityProcedureDef, AbilityProgramDef, AppliedEffectDef,
    CardDefinition, CharacteristicOperationDef, DeclarativeAbilityDef, EffectDef,
    EffectExecutionDef, EffectRecipientDef, ImplementationStatus, ReplacementEffectDef,
    ReplacementEventDef, SpellForm, ZoneKind, ZoneMoveCauseDef,
};
use crate::{AbilityId, AlternativeCostId, CardPartId, GrantId, ModeId};

pub(super) fn validate_alternative_cast_abilities(
    definition: &CardDefinition,
) -> Result<(), CatalogError> {
    for part in &definition.parts {
        for attached in part.rules.indexed_abilities() {
            let DeclarativeAbilityDef::AlternativeCast(alternative_cast) =
                attached.definition.definition
            else {
                continue;
            };
            let cost = AlternativeCostId(attached.id.0);
            let mut owning_option_found = false;
            for option in definition.play_options.iter().filter(
                |option| matches!(option.form, SpellForm::Part(candidate) if candidate == part.id),
            ) {
                owning_option_found = true;
                let Some(expected) =
                    alternative_cast.alternative_cost(attached.id, option.mana_cost)
                else {
                    return Err(CatalogError::MissingAlternativeCostForAbility {
                        definition: definition.id,
                        part: part.id,
                        ability: attached.id,
                        cost,
                    });
                };
                let Some(actual) = option
                    .alternative_costs
                    .iter()
                    .find(|cost| cost.id == expected.id)
                else {
                    return Err(CatalogError::MissingAlternativeCostForAbility {
                        definition: definition.id,
                        part: part.id,
                        ability: attached.id,
                        cost: expected.id,
                    });
                };
                if actual != &expected {
                    return Err(CatalogError::MismatchedAlternativeCostForAbility(Box::new(
                        MismatchedAlternativeCost {
                            definition: definition.id,
                            part: part.id,
                            ability: attached.id,
                            option: option.id,
                            cost: expected.id,
                            expected_label: expected.label,
                            actual_label: actual.label.clone(),
                            expected_mana_cost: expected.mana_cost,
                            actual_mana_cost: actual.mana_cost,
                        },
                    )));
                }
            }
            if !owning_option_found {
                return Err(CatalogError::MissingAlternativeCostForAbility {
                    definition: definition.id,
                    part: part.id,
                    ability: attached.id,
                    cost,
                });
            }
        }
    }
    Ok(())
}

pub(super) fn validate_abilities(
    definition: &CardDefinition,
    part: CardPartId,
    abilities: &[AbilityDef],
) -> Result<(), CatalogError> {
    if abilities.len() > usize::from(u8::MAX) + 1 {
        return Err(CatalogError::TooManyAbilities {
            definition: definition.id,
            part,
            count: abilities.len(),
        });
    }
    let spell_count = abilities
        .iter()
        .filter(|ability| matches!(ability.definition, DeclarativeAbilityDef::Spell(_)))
        .count();
    if spell_count > 1 {
        return Err(CatalogError::MultipleSpellAbilities {
            definition: definition.id,
            part,
            count: spell_count,
        });
    }

    for (index, ability) in abilities.iter().enumerate() {
        let ability_id = AbilityId::from_index(index)
            .expect("the ability count was validated before assigning positional IDs");
        validate_attached_ability(definition, part, ability_id, ability)?;
    }
    Ok(())
}

fn validate_attached_ability(
    definition: &CardDefinition,
    part: CardPartId,
    ability_id: AbilityId,
    ability: &AbilityDef,
) -> Result<(), CatalogError> {
    if let Err(problem) = validate_ability_definition(ability) {
        return Err(top_level_ability_error(
            definition, part, ability_id, &problem,
        ));
    }
    if let DeclarativeAbilityDef::Spell(spell) = ability.definition
        && let Some(modal) = spell.modal()
    {
        if ability.coverage.status != ImplementationStatus::Complete
            || ability.effect.execution != EffectExecutionDef::Declarative
            || ability.effect.definition != AbilityProgramDef::Effects(EffectDef::None)
        {
            return Err(CatalogError::InvalidModalSpellParent {
                definition: definition.id,
                part,
                ability: ability_id,
            });
        }
        if modal.modes.len() > usize::from(u8::MAX) + 1 {
            return Err(CatalogError::TooManySpellModes {
                definition: definition.id,
                part,
                ability: ability_id,
                count: modal.modes.len(),
            });
        }
        // A conditional "you may choose two instead" has to be a real
        // increase and has to stay within the modes the card prints, or the
        // condition would offer a selection that cannot be made.
        let conditional_maximum = modal
            .conditional_maximum
            .map_or(modal.maximum, |conditional| conditional.maximum);
        if modal.modes.is_empty()
            || modal.minimum > modal.maximum
            || modal.maximum == 0
            || conditional_maximum < modal.maximum
            || (!modal.may_repeat && usize::from(conditional_maximum) > modal.modes.len())
        {
            return Err(CatalogError::InvalidModalSpellSelection {
                definition: definition.id,
                part,
                ability: ability_id,
                minimum: modal.minimum,
                maximum: conditional_maximum,
                may_repeat: modal.may_repeat,
                available: modal.modes.len(),
            });
        }
        for (index, mode) in modal.modes.iter().enumerate() {
            let mode_id = ModeId::from_index(index)
                .expect("the spell mode count was validated before assigning positional IDs");
            let DeclarativeAbilityDef::Spell(mode_spell) = mode.definition else {
                return Err(CatalogError::NonSpellMode {
                    definition: definition.id,
                    part,
                    ability: ability_id,
                    mode: mode_id,
                });
            };
            if mode_spell.modal().is_some() {
                return Err(CatalogError::NestedModalSpellMode {
                    definition: definition.id,
                    part,
                    ability: ability_id,
                    mode: mode_id,
                });
            }
            if mode.is_executable() && mode.declarative_effect().is_none() {
                return Err(CatalogError::CustomSpellModeImplementation {
                    definition: definition.id,
                    part,
                    ability: ability_id,
                    mode: mode_id,
                });
            }
            if let Err(problem) = validate_ability_definition(mode) {
                return Err(CatalogError::InvalidSpellMode {
                    definition: definition.id,
                    part,
                    ability: ability_id,
                    mode: mode_id,
                    problem,
                });
            }
        }
    }
    validate_granted_abilities(definition, part, ability_id, ability, &mut Vec::new())
}

fn validate_granted_abilities(
    definition: &CardDefinition,
    part: CardPartId,
    outer_ability: AbilityId,
    ability: &AbilityDef,
    path: &mut Vec<GrantId>,
) -> Result<(), CatalogError> {
    let mut grants = Vec::new();
    collect_direct_ability_grants(ability, &mut grants);
    for (index, granted) in grants.into_iter().enumerate() {
        let grant = GrantId::from_index(index)
            .expect("the containing ability's grant-site capacity was validated");
        path.push(grant);
        if granted.is_executable() && matches!(granted.definition, DeclarativeAbilityDef::Static(_))
        {
            return Err(CatalogError::InvalidGrantedAbility {
                definition: definition.id,
                part,
                ability: outer_ability,
                grant_path: path.clone(),
                problem: GrantedAbilityValidationError::ExecutableStaticAbility,
            });
        }
        if let Err(problem) = validate_ability_definition(granted) {
            return Err(CatalogError::InvalidGrantedAbility {
                definition: definition.id,
                part,
                ability: outer_ability,
                grant_path: path.clone(),
                problem,
            });
        }
        validate_granted_abilities(definition, part, outer_ability, granted, path)?;
        path.pop();
    }
    Ok(())
}

/// Collects the grant sites owned directly by one ability clause. Modal spell
/// branches are part of their parent clause's effect tree, so their sites
/// continue the same [`GrantId`] sequence in printed mode order.
fn collect_direct_ability_grants<'a>(ability: &'a AbilityDef, grants: &mut Vec<&'a AbilityDef>) {
    collect_program_ability_grants(ability.effect.definition, grants);
    if let DeclarativeAbilityDef::Spell(spell) = ability.definition
        && let Some(modal) = spell.modal()
    {
        for mode in modal.modes {
            collect_program_ability_grants(mode.effect.definition, grants);
        }
    }
}

#[allow(clippy::too_many_lines)]
fn validate_ability_definition(ability: &AbilityDef) -> Result<(), GrantedAbilityValidationError> {
    let mut grant_sites = program_ability_grant_sites(ability.effect.definition);
    if let DeclarativeAbilityDef::Spell(spell) = ability.definition
        && let Some(modal) = spell.modal()
    {
        grant_sites = modal
            .modes
            .iter()
            .map(|mode| program_ability_grant_sites(mode.effect.definition))
            .fold(grant_sites, usize::saturating_add);
    }
    if grant_sites > usize::from(u8::MAX) + 1 {
        return Err(GrantedAbilityValidationError::TooManyGrantSites { count: grant_sites });
    }
    if ability.text.trim().is_empty() {
        return Err(GrantedAbilityValidationError::EmptyText);
    }
    let uses_legacy_procedure = match ability.definition {
        DeclarativeAbilityDef::ActivatedMana(definition)
        | DeclarativeAbilityDef::Activated(definition) => {
            definition.procedure == AbilityProcedureDef::Legacy
        }
        DeclarativeAbilityDef::TriggeredMana(definition)
        | DeclarativeAbilityDef::Triggered(definition) => {
            definition.procedure == AbilityProcedureDef::Legacy
        }
        DeclarativeAbilityDef::Spell(_)
        | DeclarativeAbilityDef::Static(_)
        | DeclarativeAbilityDef::Replacement(_)
        | DeclarativeAbilityDef::AlternativeCast(_)
        | DeclarativeAbilityDef::SpecialAction(_)
        | DeclarativeAbilityDef::Keyword(_)
        | DeclarativeAbilityDef::Legacy => false,
    };
    if ability.is_executable()
        && uses_legacy_procedure
        && !matches!(ability.effect.execution, EffectExecutionDef::Custom(_))
    {
        return Err(GrantedAbilityValidationError::LegacyProcedureRequiresCustomExecution);
    }
    let explanation = ability.coverage.explanation;
    let explanation_required = ability.coverage.status != ImplementationStatus::Complete
        || ability.effect.execution != EffectExecutionDef::Declarative
        || uses_legacy_procedure;
    if explanation.is_some_and(|explanation| explanation.trim().is_empty())
        || (explanation_required && explanation.is_none())
    {
        return Err(GrantedAbilityValidationError::MissingImplementationExplanation);
    }
    validate_ability_program(ability)?;
    let (source_zones, targets, is_mana_ability) = match &ability.definition {
        DeclarativeAbilityDef::Spell(spell) => (None, spell.targets(), false),
        DeclarativeAbilityDef::ActivatedMana(activated) => {
            (Some(activated.source_zones), activated.targets, true)
        }
        DeclarativeAbilityDef::TriggeredMana(triggered) => {
            (Some(triggered.source_zones), triggered.targets, true)
        }
        DeclarativeAbilityDef::Activated(activated) => {
            (Some(activated.source_zones), activated.targets, false)
        }
        DeclarativeAbilityDef::Triggered(triggered) => {
            (Some(triggered.source_zones), triggered.targets, false)
        }
        DeclarativeAbilityDef::Static(static_ability) => {
            (Some(static_ability.source_zones), &[][..], false)
        }
        DeclarativeAbilityDef::Replacement(replacement) => {
            (Some(replacement.source_zones), &[][..], false)
        }
        DeclarativeAbilityDef::SpecialAction(special_action) => {
            (Some(special_action.source_zones), &[][..], false)
        }
        DeclarativeAbilityDef::AlternativeCast(alternative) => (None, alternative.targets, false),
        DeclarativeAbilityDef::Keyword(_) | DeclarativeAbilityDef::Legacy => (None, &[][..], false),
    };

    if source_zones.is_some_and(<[ZoneKind]>::is_empty) {
        return Err(GrantedAbilityValidationError::HasNoSourceZone);
    }
    if is_mana_ability && !targets.is_empty() {
        return Err(GrantedAbilityValidationError::ManaAbilityHasTargets);
    }
    if ability.is_executable() {
        match ability.definition {
            DeclarativeAbilityDef::TriggeredMana(triggered)
            | DeclarativeAbilityDef::Triggered(triggered) => {
                if triggered.procedure == AbilityProcedureDef::Shared
                    && (!matches!(
                        triggered.source_zones,
                        [ZoneKind::Battlefield | ZoneKind::Graveyard]
                    ) || (triggered.event == crate::card::TriggerEventDef::StateCondition
                        && triggered.condition.is_none())
                        || (matches!(ability.definition, DeclarativeAbilityDef::TriggeredMana(_))
                            && (triggered.condition.is_some()
                                || !matches!(
                                    triggered.event,
                                    crate::card::TriggerEventDef::Tapped(matcher)
                                        if matcher.purpose == crate::card::TapPurposeDef::Mana
                                ))))
                {
                    return Err(GrantedAbilityValidationError::UnsupportedTriggerEvent {
                        event: triggered.event,
                    });
                }
                validate_ability_trigger_event(triggered.event, targets.len())?;
                if triggered.procedure == AbilityProcedureDef::Shared
                    && matches!(ability.definition, DeclarativeAbilityDef::TriggeredMana(_))
                    && !matches!(
                        ability.effect.definition,
                        AbilityProgramDef::Effects(effect)
                            if triggered_mana_program_is_immediate(effect)
                    )
                {
                    return Err(GrantedAbilityValidationError::UnsupportedTriggeredManaProgram);
                }
            }
            _ => {}
        }
    }
    if let Err(problem) = validate_ability_effect_context(ability) {
        return Err(
            GrantedAbilityValidationError::UnsupportedEffectProgramContext {
                context: problem.context,
                operation: problem.operation,
            },
        );
    }
    let trigger_event = match ability.definition {
        DeclarativeAbilityDef::TriggeredMana(definition)
        | DeclarativeAbilityDef::Triggered(definition) => Some(definition.event),
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
    validate_ability_program_targets(targets, ability.effect.definition, trigger_event)?;
    Ok(())
}

fn triggered_mana_program_is_immediate(effect: EffectDef) -> bool {
    match effect {
        EffectDef::Sequence(effects) => {
            !effects.is_empty()
                && effects
                    .iter()
                    .copied()
                    .all(triggered_mana_program_is_immediate)
        }
        EffectDef::AddMana(mana) => {
            matches!(mana.mana, crate::card::ManaSelectionDef::One(_)) && mana.amount > 0
        }
        _ => false,
    }
}

fn validate_ability_program(ability: &AbilityDef) -> Result<(), GrantedAbilityValidationError> {
    match (ability.definition, ability.effect.definition) {
        (
            DeclarativeAbilityDef::Replacement(definition),
            AbilityProgramDef::Replacement(effect),
        ) => {
            if ability.is_executable()
                && ability.effect.execution == EffectExecutionDef::Declarative
                && let Err(operation) =
                    validate_replacement_program_for_event(definition.event, effect)
            {
                return Err(
                    GrantedAbilityValidationError::UnsupportedReplacementProgram {
                        event: definition.event,
                        operation,
                    },
                );
            }
        }
        (DeclarativeAbilityDef::Replacement(_), AbilityProgramDef::Effects(_)) => {
            return Err(
                GrantedAbilityValidationError::ReplacementAbilityRequiresReplacementProgram,
            );
        }
        (_, AbilityProgramDef::Replacement(_)) => {
            return Err(
                GrantedAbilityValidationError::ReplacementProgramRequiresReplacementAbility,
            );
        }
        (_, AbilityProgramDef::Effects(_)) => {}
    }
    Ok(())
}

fn validate_replacement_program_for_event(
    event: ReplacementEventDef,
    effect: ReplacementEffectDef,
) -> Result<(), &'static str> {
    match event {
        ReplacementEventDef::SourceEntersBattlefield
        | ReplacementEventDef::ObjectEntersBattlefield { .. } => {
            validate_entry_replacement_program(effect)
        }
        ReplacementEventDef::WouldMove {
            from: Some(ZoneKind::Hand),
            to: ZoneKind::Graveyard,
            ..
        } if effect == ReplacementEffectDef::MoveToZone(ZoneKind::Battlefield) => Ok(()),
        // "From anywhere" replaces the same move wherever it starts, so it
        // is held to the same program as the battlefield exit it includes.
        ReplacementEventDef::WouldMove {
            from: None | Some(ZoneKind::Battlefield),
            to: ZoneKind::Graveyard,
            cause: ZoneMoveCauseDef::Any,
        } => validate_battlefield_exit_replacement_program(effect),
        ReplacementEventDef::WouldGainLife(_)
            if matches!(effect, ReplacementEffectDef::MultiplyEventAmount(_)) =>
        {
            Ok(())
        }
        ReplacementEventDef::WouldBeginTurn { .. } => {
            validate_begin_turn_replacement_program(effect)
        }
        ReplacementEventDef::AnyObjectWouldMove {
            to: ZoneKind::Graveyard,
        } if effect == ReplacementEffectDef::MoveToZone(ZoneKind::Exile) => Ok(()),
        ReplacementEventDef::WouldMove { .. }
        | ReplacementEventDef::WouldGainLife(_)
        | ReplacementEventDef::AnyObjectWouldMove { .. }
        | ReplacementEventDef::Special(_) => Err(replacement_operation_name(effect)),
    }
}

fn validate_entry_replacement_program(effect: ReplacementEffectDef) -> Result<(), &'static str> {
    match effect {
        ReplacementEffectDef::ModifyBattlefieldEntry(_)
        | ReplacementEffectDef::Choose(_)
        // Sending the entering card somewhere else instead, which is how an
        // unpaid Mox Diamond reaches its owner's graveyard.
        | ReplacementEffectDef::MoveToZone(_)
        | ReplacementEffectDef::CopyEntering { .. } => Ok(()),
        ReplacementEffectDef::Sequence(effects) => {
            if effects.is_empty() {
                return Err("empty Sequence");
            }
            for effect in effects {
                validate_entry_replacement_program(*effect)?;
            }
            Ok(())
        }
        ReplacementEffectDef::Conditional {
            if_true, if_false, ..
        } => {
            for effect in if_true.iter().chain(if_false.iter()) {
                validate_entry_replacement_program(*effect)?;
            }
            Ok(())
        }
        ReplacementEffectDef::PayOr {
            if_paid,
            if_declined,
            ..
        } => {
            for effect in if_paid.iter().chain(if_declined.iter()) {
                validate_entry_replacement_program(*effect)?;
            }
            Ok(())
        }
        ReplacementEffectDef::ReplaceEventWithNothing
        | ReplacementEffectDef::Perform(_)
        | ReplacementEffectDef::MultiplyEventAmount(_) => Err(replacement_operation_name(effect)),
    }
}

fn validate_begin_turn_replacement_program(
    effect: ReplacementEffectDef,
) -> Result<(), &'static str> {
    match effect {
        ReplacementEffectDef::ReplaceEventWithNothing => Ok(()),
        ReplacementEffectDef::Perform(effect)
            if matches!(
                *effect,
                EffectDef::Untap {
                    object: EffectRecipientDef::Source,
                }
            ) =>
        {
            Ok(())
        }
        ReplacementEffectDef::Sequence(effects) => {
            if effects.is_empty() {
                return Err("empty Sequence");
            }
            for effect in effects {
                validate_begin_turn_replacement_program(*effect)?;
            }
            if !effects
                .iter()
                .any(|effect| matches!(effect, ReplacementEffectDef::ReplaceEventWithNothing))
            {
                return Err("Sequence without ReplaceEventWithNothing");
            }
            Ok(())
        }
        ReplacementEffectDef::MoveToZone(_)
        | ReplacementEffectDef::Perform(_)
        | ReplacementEffectDef::ModifyBattlefieldEntry(_)
        | ReplacementEffectDef::MultiplyEventAmount(_)
        | ReplacementEffectDef::Choose(_)
        | ReplacementEffectDef::CopyEntering { .. }
        | ReplacementEffectDef::Conditional { .. }
        | ReplacementEffectDef::PayOr { .. } => Err(replacement_operation_name(effect)),
    }
}

fn validate_battlefield_exit_replacement_program(
    effect: ReplacementEffectDef,
) -> Result<(), &'static str> {
    match effect {
        // Exile and library are the two destinations that answer "instead":
        // one takes the card out of the game, the other puts it back where
        // it came from.
        ReplacementEffectDef::MoveToZone(ZoneKind::Exile | ZoneKind::Library) => Ok(()),
        ReplacementEffectDef::Perform(effect)
            if matches!(
                *effect,
                EffectDef::TakeExtraTurn {
                    player: EffectRecipientDef::Controller,
                } | EffectDef::ShuffleLibrary {
                    player: EffectRecipientDef::Controller,
                }
            ) =>
        {
            Ok(())
        }
        ReplacementEffectDef::Sequence(effects) => {
            if effects.is_empty() {
                return Err("empty Sequence");
            }
            for effect in effects {
                validate_battlefield_exit_replacement_program(*effect)?;
            }
            if !effects
                .iter()
                .any(|effect| matches!(effect, ReplacementEffectDef::MoveToZone(_)))
            {
                return Err("Sequence without MoveToZone");
            }
            Ok(())
        }
        ReplacementEffectDef::ReplaceEventWithNothing
        | ReplacementEffectDef::MoveToZone(_)
        | ReplacementEffectDef::Perform(_)
        | ReplacementEffectDef::ModifyBattlefieldEntry(_)
        | ReplacementEffectDef::MultiplyEventAmount(_)
        | ReplacementEffectDef::Choose(_)
        | ReplacementEffectDef::CopyEntering { .. }
        | ReplacementEffectDef::Conditional { .. }
        | ReplacementEffectDef::PayOr { .. } => Err(replacement_operation_name(effect)),
    }
}

const fn replacement_operation_name(effect: ReplacementEffectDef) -> &'static str {
    match effect {
        ReplacementEffectDef::Sequence(_) => "Sequence",
        ReplacementEffectDef::ReplaceEventWithNothing => "ReplaceEventWithNothing",
        ReplacementEffectDef::MoveToZone(_) => "MoveToZone",
        ReplacementEffectDef::Perform(_) => "Perform",
        ReplacementEffectDef::ModifyBattlefieldEntry(_) => "ModifyBattlefieldEntry",
        ReplacementEffectDef::MultiplyEventAmount(_) => "MultiplyEventAmount",
        ReplacementEffectDef::Choose(_) => "Choose",
        ReplacementEffectDef::CopyEntering { .. } => "CopyEntering",
        ReplacementEffectDef::Conditional { .. } => "Conditional",
        ReplacementEffectDef::PayOr { .. } => "PayOr",
    }
}

// Mapping internal validation failures onto card/catalog identity is kept
// separate from the recursive ability-program walk below.
include!("abilities/top_level_errors.rs");

// Walking a definition for the abilities it grants, and counting where it
// grants them. Kept beside the validation above rather than in it: the
// walk is one arm per effect variant and says nothing about validity.
include!("abilities/ability_grants.rs");
