//! Reusable constructors for common ability clauses.
//!
//! The functions here return identity-free [`AbilityDef`] values. A card part,
//! intrinsic rule, or grant site assigns identity when it attaches the clause.

use super::model::{
    AbilityCostDef, AbilityDef, AbilityImplementationDef, AddManaEffectDef, AlternativeCastKindDef,
    AppliedEffectDef, EffectDef, EffectDurationDef, EffectRecipientDef, KeywordAbility, ManaColor,
    ZoneKind,
};
use crate::ids::AlternativeCostId;

const fn keyword(text: &'static str, keyword: KeywordAbility) -> AbilityDef {
    AbilityDef::keyword(text, keyword)
}

const fn unsupported_keyword(
    text: &'static str,
    ability: KeywordAbility,
    explanation: &'static str,
) -> AbilityDef {
    keyword(text, ability)
        .with_implementation(AbilityImplementationDef::NotImplemented { explanation })
}

#[must_use]
pub const fn flying() -> AbilityDef {
    keyword("Flying", KeywordAbility::Flying)
}

#[must_use]
pub const fn trample() -> AbilityDef {
    keyword("Trample", KeywordAbility::Trample)
}

#[must_use]
pub const fn haste() -> AbilityDef {
    keyword("Haste", KeywordAbility::Haste)
}

#[must_use]
pub const fn first_strike() -> AbilityDef {
    keyword("First strike", KeywordAbility::FirstStrike)
}

#[must_use]
pub const fn defender() -> AbilityDef {
    keyword("Defender", KeywordAbility::Defender)
}

#[must_use]
pub const fn double_strike() -> AbilityDef {
    keyword("Double strike", KeywordAbility::DoubleStrike)
}

#[must_use]
pub const fn banding() -> AbilityDef {
    unsupported_keyword(
        "Banding",
        KeywordAbility::Banding,
        "Band formation and combat damage assignment are not implemented.",
    )
}

#[must_use]
pub const fn vigilance() -> AbilityDef {
    keyword("Vigilance", KeywordAbility::Vigilance)
}

#[must_use]
pub const fn deathtouch() -> AbilityDef {
    keyword("Deathtouch", KeywordAbility::Deathtouch)
}

#[must_use]
pub const fn lifelink() -> AbilityDef {
    keyword("Lifelink", KeywordAbility::Lifelink)
}

#[must_use]
pub const fn reach() -> AbilityDef {
    keyword("Reach", KeywordAbility::Reach)
}

#[must_use]
pub const fn flash() -> AbilityDef {
    keyword("Flash", KeywordAbility::Flash)
}

#[must_use]
pub const fn hexproof() -> AbilityDef {
    keyword("Hexproof", KeywordAbility::Hexproof)
}

#[must_use]
pub const fn intimidate() -> AbilityDef {
    keyword(
        "Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)",
        KeywordAbility::Intimidate,
    )
}

#[must_use]
pub const fn undying() -> AbilityDef {
    keyword("Undying", KeywordAbility::Undying)
}

#[must_use]
pub const fn mountainwalk() -> AbilityDef {
    keyword("Mountainwalk", KeywordAbility::Mountainwalk)
}

#[must_use]
pub const fn protection_from(color: ManaColor) -> AbilityDef {
    let text = match color {
        ManaColor::White => "Protection from white",
        ManaColor::Blue => "Protection from blue",
        ManaColor::Black => "Protection from black",
        ManaColor::Red => "Protection from red",
        ManaColor::Green => "Protection from green",
        ManaColor::Colorless => "Protection from colorless",
    };
    keyword(text, KeywordAbility::ProtectionFrom(color))
}

/// A printed flashback clause linked to one alternative cost on the card's
/// spell play option.
#[must_use]
pub const fn flashback(text: &'static str, alternative: AlternativeCostId) -> AbilityDef {
    AbilityDef::alternative_cast(
        text,
        alternative,
        AlternativeCastKindDef::Flashback,
        None,
        EffectDef::None,
    )
}

/// A printed overload clause linked to one alternative cost. `effect` is the
/// spell after every instance of "target" has been changed to "each."
#[must_use]
pub const fn overload(
    text: &'static str,
    alternative: AlternativeCostId,
    stack_text: &'static str,
    effect: EffectDef,
) -> AbilityDef {
    AbilityDef::alternative_cast(
        text,
        alternative,
        AlternativeCastKindDef::Overload,
        Some(stack_text),
        effect,
    )
}

/// The static ability carried by a spell that says it can't be countered.
#[must_use]
pub const fn cannot_be_countered() -> AbilityDef {
    AbilityDef::static_ability(
        "This spell can't be countered.",
        EffectDef::Apply {
            recipient: EffectRecipientDef::Source,
            effect: AppliedEffectDef::CannotBeCountered,
            duration: EffectDurationDef::WhileSourceRemainsInZone,
        },
    )
    .with_source_zones(&[ZoneKind::Stack])
}

/// A common mana ability that taps its source to add one fixed kind of mana.
#[must_use]
pub const fn tap_for(mana: ManaColor) -> AbilityDef {
    let text = match mana {
        ManaColor::White => "{T}: Add {W}.",
        ManaColor::Blue => "{T}: Add {U}.",
        ManaColor::Black => "{T}: Add {B}.",
        ManaColor::Red => "{T}: Add {R}.",
        ManaColor::Green => "{T}: Add {G}.",
        ManaColor::Colorless => "{T}: Add {C}.",
    };
    AbilityDef::activated_mana(
        text,
        &[AbilityCostDef::TapSource],
        EffectDef::AddMana(AddManaEffectDef::one(mana)),
    )
}

#[cfg(test)]
mod tests {
    use super::{banding, double_strike, first_strike, flying, intimidate, tap_for};
    use crate::card::{
        AbilityCostDef, AbilityDef, AbilityImplementationDef, AddManaEffectDef, CardRules,
        DeclarativeAbilityDef, EffectDef, KeywordAbility, ManaColor, ManaCost,
    };

    #[test]
    fn tap_for_builds_a_complete_executable_mana_ability() {
        let cases = [
            (ManaColor::White, "{T}: Add {W}."),
            (ManaColor::Blue, "{T}: Add {U}."),
            (ManaColor::Black, "{T}: Add {B}."),
            (ManaColor::Red, "{T}: Add {R}."),
            (ManaColor::Green, "{T}: Add {G}."),
            (ManaColor::Colorless, "{T}: Add {C}."),
        ];

        for (mana, text) in cases {
            let ability = tap_for(mana);
            assert_eq!(ability.text, text);
            assert_eq!(ability.implementation, AbilityImplementationDef::Definition);
            assert!(ability.implementation.is_executable());
            assert!(matches!(
                ability.definition,
                DeclarativeAbilityDef::ActivatedMana(definition)
                    if definition.costs == [AbilityCostDef::TapSource]
            ));
            assert_eq!(
                ability.effect,
                EffectDef::AddMana(AddManaEffectDef::one(mana))
            );
        }
    }

    #[test]
    fn keyword_presence_is_distinct_from_executable_keyword_support() {
        static KEYWORDS: [AbilityDef; 2] = [flying(), banding()];
        let rules =
            CardRules::new_creature(ManaCost::default(), &[], 1, 1).with_abilities(&KEYWORDS);

        assert!(rules.has_keyword(KeywordAbility::Flying));
        assert!(rules.has_executable_keyword(KeywordAbility::Flying));
        assert!(rules.has_keyword(KeywordAbility::Banding));
        assert!(!rules.has_executable_keyword(KeywordAbility::Banding));
    }

    #[test]
    fn common_combat_keywords_are_complete_definitions() {
        let cases = [
            (first_strike(), KeywordAbility::FirstStrike),
            (double_strike(), KeywordAbility::DoubleStrike),
            (intimidate(), KeywordAbility::Intimidate),
        ];

        for (ability, expected) in cases {
            assert_eq!(ability.implementation, AbilityImplementationDef::Definition);
            assert!(ability.implementation.is_executable());
            assert_eq!(ability.definition, DeclarativeAbilityDef::Keyword(expected));
        }
        assert_eq!(
            intimidate().text,
            "Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)"
        );
    }
}
