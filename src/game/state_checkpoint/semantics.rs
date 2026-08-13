use super::model::{AbilityLocator, AnimationSnapshot, KeywordSnapshot};
use crate::CardCatalog;
use crate::card::{
    AbilityDef, AnimationDef, AppliedEffectDef, DeclarativeAbilityDef, EffectDef, KeywordAbility,
    ManaColor, SpellAbilityDef,
};

pub(super) fn ability_locator(
    catalog: &CardCatalog,
    mut matches: impl FnMut(&AbilityDef) -> bool,
) -> Option<AbilityLocator> {
    for definition in catalog.definitions() {
        for part in &definition.parts {
            for attached in part.rules.indexed_abilities() {
                let mut nested = Vec::new();
                if locate_ability(&attached.definition, &mut matches, &mut nested) {
                    return Some(AbilityLocator {
                        definition: definition.id.0,
                        part_id: part.id.0,
                        ability_id: attached.id.0,
                        nested,
                    });
                }
            }
        }
    }
    None
}

pub(super) fn catalog_ability(
    catalog: &CardCatalog,
    locator: &AbilityLocator,
) -> Option<AbilityDef> {
    let mut current = *catalog
        .get(crate::CardDefinitionId(locator.definition))?
        .part(crate::CardPartId(locator.part_id))?
        .rules
        .ability(crate::AbilityId(locator.ability_id))?;
    for &index in &locator.nested {
        current = **child_abilities(&current).get(index)?;
    }
    Some(current)
}

fn locate_ability(
    ability: &AbilityDef,
    matches: &mut impl FnMut(&AbilityDef) -> bool,
    path: &mut Vec<usize>,
) -> bool {
    if matches(ability) {
        return true;
    }
    for (index, child) in child_abilities(ability).into_iter().enumerate() {
        path.push(index);
        if locate_ability(child, matches, path) {
            return true;
        }
        path.pop();
    }
    false
}

fn child_abilities(ability: &AbilityDef) -> Vec<&AbilityDef> {
    let mut children = Vec::new();
    if let DeclarativeAbilityDef::Spell(SpellAbilityDef::Modal(modal)) = ability.definition {
        children.extend(modal.modes);
    }
    collect_effect_abilities(ability.effect.definition, &mut children);
    children
}

fn collect_effect_abilities(effect: EffectDef, abilities: &mut Vec<&'static AbilityDef>) {
    match effect {
        EffectDef::Sequence(effects) => {
            for effect in effects {
                collect_effect_abilities(*effect, abilities);
            }
        }
        EffectDef::Randomized {
            on_success,
            on_failure,
            ..
        } => {
            collect_effect_abilities(*on_success, abilities);
            collect_effect_abilities(*on_failure, abilities);
        }
        EffectDef::OptionalPayment {
            if_paid: effect, ..
        }
        | EffectDef::UnlessPaid {
            otherwise: effect, ..
        }
        | EffectDef::May(effect)
        | EffectDef::IfCondition { then: effect, .. }
        | EffectDef::AtNextStep { effect, .. }
        | EffectDef::ChoosePermanent { then: effect, .. }
        | EffectDef::SacrificeOfChoice {
            then: Some(effect), ..
        } => collect_effect_abilities(*effect, abilities),
        EffectDef::LookAtTopAndSelect { selection, .. } => {
            if let Some(effect) = selection.then {
                collect_effect_abilities(*effect, abilities);
            }
        }
        EffectDef::Apply { effect, .. } => collect_applied_abilities(effect, abilities),
        EffectDef::TriggerUntilYourNextTurn { ability } => abilities.push(ability),
        EffectDef::None
        | EffectDef::AddMana(_)
        | EffectDef::AddManaEqualTo { .. }
        | EffectDef::DealDamage { .. }
        | EffectDef::DrainLife { .. }
        | EffectDef::GainLife { .. }
        | EffectDef::DrawCards { .. }
        | EffectDef::Discard { .. }
        | EffectDef::ShuffleLibrary { .. }
        | EffectDef::EmptyManaPool { .. }
        | EffectDef::LoseLife { .. }
        | EffectDef::LoseTheGame { .. }
        | EffectDef::Tap { .. }
        | EffectDef::Untap { .. }
        | EffectDef::PreventCombatDamageThisTurn { .. }
        | EffectDef::PreventCombatDamageDealtByThisTurn { .. }
        | EffectDef::Attach { .. }
        | EffectDef::CreateToken { .. }
        | EffectDef::Destroy { .. }
        | EffectDef::Sacrifice { .. }
        | EffectDef::SacrificeOfChoice { then: None, .. }
        | EffectDef::DestroyOfChoice { .. }
        | EffectDef::SplitPermanentsAndSacrificeAPile { .. }
        | EffectDef::RevealAndSplitIntoPiles { .. }
        | EffectDef::Mill { .. }
        | EffectDef::LookAtTopAndMayTake { .. }
        | EffectDef::LookAtHand { .. }
        | EffectDef::SearchLibrary { .. }
        | EffectDef::Counter { .. }
        | EffectDef::CounterUnlessPaid { .. }
        | EffectDef::AddCounters { .. }
        | EffectDef::ChangeTextBasicLandType { .. }
        | EffectDef::BecomeCopyOf { .. }
        | EffectDef::CannotBeForcedToSacrifice
        | EffectDef::CreateEmblem { .. }
        | EffectDef::Transform { .. }
        | EffectDef::AdditionalCombatPhase
        | EffectDef::CannotCastNoncreatureSpellsThisTurn { .. }
        | EffectDef::GrantFlashToNextSorcery
        | EffectDef::ExileLinkedToSource { .. }
        | EffectDef::ReturnLinkedExiles { .. }
        | EffectDef::MakeUnblockableThisTurn { .. }
        | EffectDef::GainControlThisTurn { .. }
        | EffectDef::ReduceGenericCostBy(_)
        | EffectDef::PlayersCantPlay(_)
        | EffectDef::MultiplyEventAmount(_)
        | EffectDef::Replacement(_)
        | EffectDef::MoveToZone { .. }
        | EffectDef::ChooseCardName { .. }
        | EffectDef::ChoosePlayer { .. }
        | EffectDef::CopyPermanentAsItEnters { .. }
        | EffectDef::ChooseCreatureType { .. }
        | EffectDef::Special(_) => {}
    }
}

fn collect_applied_abilities(effect: AppliedEffectDef, abilities: &mut Vec<&'static AbilityDef>) {
    match effect {
        AppliedEffectDef::Composite(effects) => {
            for effect in effects {
                collect_applied_abilities(*effect, abilities);
            }
        }
        AppliedEffectDef::GrantAbility(ability) => abilities.push(ability),
        AppliedEffectDef::CannotBeCountered
        | AppliedEffectDef::DoesNotUntapDuringUntapStep
        | AppliedEffectDef::CannotBeEnchanted
        | AppliedEffectDef::CannotBecomeEnchanted
        | AppliedEffectDef::CannotChangeController
        | AppliedEffectDef::CannotBeBlockedBy(_)
        | AppliedEffectDef::PreventDamageFrom(_)
        | AppliedEffectDef::AddLandTypes(_)
        | AppliedEffectDef::SetLandTypes(_)
        | AppliedEffectDef::RemoveAbilities(_)
        | AppliedEffectDef::Animate(_)
        | AppliedEffectDef::ModifyPowerToughness { .. }
        | AppliedEffectDef::Special(_) => {}
    }
}

pub(super) fn animation_snapshot(animation: &AnimationDef) -> AnimationSnapshot {
    AnimationSnapshot {
        power: animation.power,
        toughness: animation.toughness,
        types: animation.types.type_name().clone(),
        subtypes: animation
            .subtypes
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        all_creature_types: animation.all_creature_types,
        replaces_subtypes: animation.replaces_subtypes,
        loses_abilities: animation.loses_abilities,
        colors: animation.colors.map(crate::card::ColorSet::to_flags),
    }
}

pub(super) const fn keyword_snapshot(keyword: KeywordAbility) -> KeywordSnapshot {
    match keyword {
        KeywordAbility::Flying => KeywordSnapshot::Flying,
        KeywordAbility::Trample => KeywordSnapshot::Trample,
        KeywordAbility::Haste => KeywordSnapshot::Haste,
        KeywordAbility::FirstStrike => KeywordSnapshot::FirstStrike,
        KeywordAbility::DoubleStrike => KeywordSnapshot::DoubleStrike,
        KeywordAbility::Banding => KeywordSnapshot::Banding,
        KeywordAbility::Vigilance => KeywordSnapshot::Vigilance,
        KeywordAbility::Defender => KeywordSnapshot::Defender,
        KeywordAbility::Deathtouch => KeywordSnapshot::Deathtouch,
        KeywordAbility::Lifelink => KeywordSnapshot::Lifelink,
        KeywordAbility::Reach => KeywordSnapshot::Reach,
        KeywordAbility::Flash => KeywordSnapshot::Flash,
        KeywordAbility::Hexproof => KeywordSnapshot::Hexproof,
        KeywordAbility::Shroud => KeywordSnapshot::Shroud,
        KeywordAbility::Intimidate => KeywordSnapshot::Intimidate,
        KeywordAbility::Undying => KeywordSnapshot::Undying,
        KeywordAbility::Indestructible => KeywordSnapshot::Indestructible,
        KeywordAbility::AttacksEachCombatIfAble => KeywordSnapshot::AttacksEachCombatIfAble,
        KeywordAbility::Mountainwalk => KeywordSnapshot::Mountainwalk,
        KeywordAbility::Forestwalk => KeywordSnapshot::Forestwalk,
        KeywordAbility::ProtectionFrom(ManaColor::White) => KeywordSnapshot::ProtectionFromWhite,
        KeywordAbility::ProtectionFrom(ManaColor::Blue) => KeywordSnapshot::ProtectionFromBlue,
        KeywordAbility::ProtectionFrom(ManaColor::Black) => KeywordSnapshot::ProtectionFromBlack,
        KeywordAbility::ProtectionFrom(ManaColor::Red) => KeywordSnapshot::ProtectionFromRed,
        KeywordAbility::ProtectionFrom(ManaColor::Green) => KeywordSnapshot::ProtectionFromGreen,
        KeywordAbility::ProtectionFrom(ManaColor::Colorless) => {
            KeywordSnapshot::ProtectionFromColorless
        }
    }
}

pub(super) const fn parse_keyword(value: KeywordSnapshot) -> KeywordAbility {
    match value {
        KeywordSnapshot::Flying => KeywordAbility::Flying,
        KeywordSnapshot::Trample => KeywordAbility::Trample,
        KeywordSnapshot::Haste => KeywordAbility::Haste,
        KeywordSnapshot::FirstStrike => KeywordAbility::FirstStrike,
        KeywordSnapshot::DoubleStrike => KeywordAbility::DoubleStrike,
        KeywordSnapshot::Banding => KeywordAbility::Banding,
        KeywordSnapshot::Vigilance => KeywordAbility::Vigilance,
        KeywordSnapshot::Defender => KeywordAbility::Defender,
        KeywordSnapshot::Deathtouch => KeywordAbility::Deathtouch,
        KeywordSnapshot::Lifelink => KeywordAbility::Lifelink,
        KeywordSnapshot::Reach => KeywordAbility::Reach,
        KeywordSnapshot::Flash => KeywordAbility::Flash,
        KeywordSnapshot::Hexproof => KeywordAbility::Hexproof,
        KeywordSnapshot::Shroud => KeywordAbility::Shroud,
        KeywordSnapshot::Intimidate => KeywordAbility::Intimidate,
        KeywordSnapshot::Undying => KeywordAbility::Undying,
        KeywordSnapshot::Indestructible => KeywordAbility::Indestructible,
        KeywordSnapshot::AttacksEachCombatIfAble => KeywordAbility::AttacksEachCombatIfAble,
        KeywordSnapshot::Mountainwalk => KeywordAbility::Mountainwalk,
        KeywordSnapshot::Forestwalk => KeywordAbility::Forestwalk,
        KeywordSnapshot::ProtectionFromWhite => KeywordAbility::ProtectionFrom(ManaColor::White),
        KeywordSnapshot::ProtectionFromBlue => KeywordAbility::ProtectionFrom(ManaColor::Blue),
        KeywordSnapshot::ProtectionFromBlack => KeywordAbility::ProtectionFrom(ManaColor::Black),
        KeywordSnapshot::ProtectionFromRed => KeywordAbility::ProtectionFrom(ManaColor::Red),
        KeywordSnapshot::ProtectionFromGreen => KeywordAbility::ProtectionFrom(ManaColor::Green),
        KeywordSnapshot::ProtectionFromColorless => {
            KeywordAbility::ProtectionFrom(ManaColor::Colorless)
        }
    }
}

pub(super) fn catalog_animation(
    catalog: &CardCatalog,
    key: &AnimationSnapshot,
) -> Option<&'static AnimationDef> {
    catalog
        .definitions()
        .into_iter()
        .flat_map(|definition| &definition.parts)
        .flat_map(|part| part.rules.indexed_abilities())
        .find_map(|attached| animation_in_ability(&attached.definition, key))
}

fn animation_in_ability(
    ability: &AbilityDef,
    key: &AnimationSnapshot,
) -> Option<&'static AnimationDef> {
    if let DeclarativeAbilityDef::Spell(SpellAbilityDef::Modal(modal)) = ability.definition
        && let Some(animation) = modal
            .modes
            .iter()
            .find_map(|mode| animation_in_ability(mode, key))
    {
        return Some(animation);
    }
    animation_in_effect(ability.effect.definition, key)
}

fn animation_in_effect(
    effect: EffectDef,
    key: &AnimationSnapshot,
) -> Option<&'static AnimationDef> {
    match effect {
        EffectDef::Sequence(effects) => effects
            .iter()
            .find_map(|effect| animation_in_effect(*effect, key)),
        EffectDef::SacrificeOfChoice { then, .. } => {
            then.and_then(|effect| animation_in_effect(*effect, key))
        }
        EffectDef::LookAtTopAndSelect { selection, .. } => selection
            .then
            .and_then(|effect| animation_in_effect(*effect, key)),
        EffectDef::OptionalPayment { if_paid, .. } => animation_in_effect(*if_paid, key),
        EffectDef::UnlessPaid { otherwise, .. }
        | EffectDef::May(otherwise)
        | EffectDef::IfCondition {
            then: otherwise, ..
        }
        | EffectDef::AtNextStep {
            effect: otherwise, ..
        } => animation_in_effect(*otherwise, key),
        EffectDef::TriggerUntilYourNextTurn { ability } => animation_in_ability(ability, key),
        EffectDef::Apply { effect, .. } => animation_in_applied(effect, key),
        _ => None,
    }
}

fn animation_in_applied(
    effect: AppliedEffectDef,
    key: &AnimationSnapshot,
) -> Option<&'static AnimationDef> {
    match effect {
        AppliedEffectDef::Animate(animation) if animation_snapshot(animation) == *key => {
            Some(animation)
        }
        AppliedEffectDef::Composite(effects) => effects
            .iter()
            .find_map(|effect| animation_in_applied(*effect, key)),
        AppliedEffectDef::GrantAbility(ability) => animation_in_ability(ability, key),
        _ => None,
    }
}
