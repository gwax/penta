//! Planeshift cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, AppliedRuleDef, BattlefieldEntryChoiceDestinationDef,
    BattlefieldEntryScalarChoiceDef, CardArt, CardRules, CardSet, CounterKind, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, PlayActionMatcherDef, PlayRestrictionDef,
    PlayerRelation, ReplacementChoiceDef, ReplacementEffectDef, TriggerEventDef, ValueDef, cards,
};
use crate::mana_cost;

// PLS 89 — Quirion Dryad
pub(in crate::card::sets) static QUIRION_DRYAD: CardRecord = CardRecord::new(
    cards::QUIRION_DRYAD,
    "Quirion Dryad",
    CardArt::new("f6841ae6-b15f-488e-9cae-2cc5ec668278", "Don Hazeltine"),
    CardSet::Planeshift,
    CardRules::new_creature(mana_cost!("{1}{G}"), &["Dryad"], 1, 1).with_ability(
        AbilityDef::triggered(
            "Whenever you cast a spell that's white, blue, black, or red, put a +1/+1 counter on this creature.",
            TriggerEventDef::SpellCast(ObjectPredicateDef::All(&[
                ObjectPredicateDef::AnyOf(&[
                    ObjectPredicateDef::Color(ManaColor::White),
                    ObjectPredicateDef::Color(ManaColor::Blue),
                    ObjectPredicateDef::Color(ManaColor::Black),
                    ObjectPredicateDef::Color(ManaColor::Red),
                ]),
                ObjectPredicateDef::ControlledBy(PlayerRelation::You),
            ])),
            EffectDef::AddCounters {
                object: EffectRecipientDef::Source,
                kind: CounterKind::PlusOnePlusOne,
                amount: ValueDef::Constant(1),
            },
        ),
    ),
);

/// The lock is a player-facing rule rather than an object one: it names the
/// action, and the predicate reads the name the Mage chose on the way in.
static SPELLS_WITH_THE_CHOSEN_NAME: PlayRestrictionDef = PlayRestrictionDef::new(
    PlayActionMatcherDef::CastSpell,
    ObjectPredicateDef::HasSourcesChosenScalar(BattlefieldEntryChoiceDestinationDef::CardName),
);

// PLS 116 — Meddling Mage
pub(in crate::card::sets) static MEDDLING_MAGE: CardRecord = CardRecord::new(
    cards::MEDDLING_MAGE,
    "Meddling Mage",
    CardArt::new(
        "176f84c6-aa5e-449c-bd2b-cc91a898f0c7",
        "Christopher Moeller",
    ),
    CardSet::Planeshift,
    // Both players, which is why the mirror is miserable: the Mage does not
    // care who was going to cast the card it named.
    CardRules::new_creature(mana_cost!("{W}{U}"), &["Human", "Wizard"], 2, 2).with_abilities(&[
        AbilityDef::replacement(
            "As this creature enters, choose a nonland card name.",
            ReplacementEffectDef::Choose(ReplacementChoiceDef::Scalar(
                BattlefieldEntryScalarChoiceDef::NONLAND_CARD_NAME,
            )),
        ),
        AbilityDef::static_ability(
            "Spells with the chosen name can't be cast.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::EachPlayer,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(
                    SPELLS_WITH_THE_CHOSEN_NAME,
                )),
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&QUIRION_DRYAD, &MEDDLING_MAGE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
