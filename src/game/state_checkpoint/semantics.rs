use serde_json::{Value, json};

use crate::CardCatalog;
use crate::card::{
    AbilityDef, AnimationDef, AppliedEffectDef, DeclarativeAbilityDef, EffectDef, KeywordAbility,
    ManaColor, SpellAbilityDef,
};

pub(super) fn animation_json(animation: &AnimationDef) -> Value {
    json!({
        "power": animation.power,
        "toughness": animation.toughness,
        "types": animation.types.type_name(),
        "subtypes": animation.subtypes,
        "allCreatureTypes": animation.all_creature_types,
        "replacesSubtypes": animation.replaces_subtypes,
        "losesAbilities": animation.loses_abilities,
        "colors": animation.colors.map(crate::card::ColorSet::to_flags),
    })
}

pub(super) fn keyword_json(keyword: KeywordAbility) -> Value {
    Value::from(match keyword {
        KeywordAbility::Flying => "flying",
        KeywordAbility::Trample => "trample",
        KeywordAbility::Haste => "haste",
        KeywordAbility::FirstStrike => "firstStrike",
        KeywordAbility::DoubleStrike => "doubleStrike",
        KeywordAbility::Banding => "banding",
        KeywordAbility::Vigilance => "vigilance",
        KeywordAbility::Defender => "defender",
        KeywordAbility::Deathtouch => "deathtouch",
        KeywordAbility::Lifelink => "lifelink",
        KeywordAbility::Reach => "reach",
        KeywordAbility::Flash => "flash",
        KeywordAbility::Hexproof => "hexproof",
        KeywordAbility::Shroud => "shroud",
        KeywordAbility::Intimidate => "intimidate",
        KeywordAbility::Undying => "undying",
        KeywordAbility::Indestructible => "indestructible",
        KeywordAbility::AttacksEachCombatIfAble => "attacksEachCombatIfAble",
        KeywordAbility::Mountainwalk => "mountainwalk",
        KeywordAbility::Forestwalk => "forestwalk",
        KeywordAbility::ProtectionFrom(ManaColor::White) => "protectionFromWhite",
        KeywordAbility::ProtectionFrom(ManaColor::Blue) => "protectionFromBlue",
        KeywordAbility::ProtectionFrom(ManaColor::Black) => "protectionFromBlack",
        KeywordAbility::ProtectionFrom(ManaColor::Red) => "protectionFromRed",
        KeywordAbility::ProtectionFrom(ManaColor::Green) => "protectionFromGreen",
        KeywordAbility::ProtectionFrom(ManaColor::Colorless) => "protectionFromColorless",
    })
}

pub(super) fn parse_keyword(value: &Value) -> Result<KeywordAbility, String> {
    match value.as_str() {
        Some("flying") => Ok(KeywordAbility::Flying),
        Some("trample") => Ok(KeywordAbility::Trample),
        Some("haste") => Ok(KeywordAbility::Haste),
        Some("firstStrike") => Ok(KeywordAbility::FirstStrike),
        Some("doubleStrike") => Ok(KeywordAbility::DoubleStrike),
        Some("banding") => Ok(KeywordAbility::Banding),
        Some("vigilance") => Ok(KeywordAbility::Vigilance),
        Some("defender") => Ok(KeywordAbility::Defender),
        Some("deathtouch") => Ok(KeywordAbility::Deathtouch),
        Some("lifelink") => Ok(KeywordAbility::Lifelink),
        Some("reach") => Ok(KeywordAbility::Reach),
        Some("flash") => Ok(KeywordAbility::Flash),
        Some("hexproof") => Ok(KeywordAbility::Hexproof),
        Some("shroud") => Ok(KeywordAbility::Shroud),
        Some("intimidate") => Ok(KeywordAbility::Intimidate),
        Some("undying") => Ok(KeywordAbility::Undying),
        Some("indestructible") => Ok(KeywordAbility::Indestructible),
        Some("attacksEachCombatIfAble") => Ok(KeywordAbility::AttacksEachCombatIfAble),
        Some("mountainwalk") => Ok(KeywordAbility::Mountainwalk),
        Some("forestwalk") => Ok(KeywordAbility::Forestwalk),
        Some("protectionFromWhite") => Ok(KeywordAbility::ProtectionFrom(ManaColor::White)),
        Some("protectionFromBlue") => Ok(KeywordAbility::ProtectionFrom(ManaColor::Blue)),
        Some("protectionFromBlack") => Ok(KeywordAbility::ProtectionFrom(ManaColor::Black)),
        Some("protectionFromRed") => Ok(KeywordAbility::ProtectionFrom(ManaColor::Red)),
        Some("protectionFromGreen") => Ok(KeywordAbility::ProtectionFrom(ManaColor::Green)),
        Some("protectionFromColorless") => Ok(KeywordAbility::ProtectionFrom(ManaColor::Colorless)),
        Some(other) => Err(format!("unknown keyword {other}")),
        None => Err("keyword must be a string".into()),
    }
}

pub(super) fn catalog_animation(
    catalog: &CardCatalog,
    key: &Value,
) -> Option<&'static AnimationDef> {
    catalog
        .definitions()
        .into_iter()
        .flat_map(|definition| &definition.parts)
        .flat_map(|part| part.rules.indexed_abilities())
        .find_map(|attached| animation_in_ability(&attached.definition, key))
}

fn animation_in_ability(ability: &AbilityDef, key: &Value) -> Option<&'static AnimationDef> {
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

fn animation_in_effect(effect: EffectDef, key: &Value) -> Option<&'static AnimationDef> {
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

fn animation_in_applied(effect: AppliedEffectDef, key: &Value) -> Option<&'static AnimationDef> {
    match effect {
        AppliedEffectDef::Animate(animation) if animation_json(animation) == *key => {
            Some(animation)
        }
        AppliedEffectDef::Composite(effects) => effects
            .iter()
            .find_map(|effect| animation_in_applied(*effect, key)),
        AppliedEffectDef::GrantAbility(ability) => animation_in_ability(ability, key),
        _ => None,
    }
}
