//! Murders at Karlov Manor cards cataloged for the Vintage Cube.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, CardArt, CardRules, CardSet, CardType, EffectDef, ObjectPredicateDef,
    PlayerRelation, TriggerEventDef, ValueDef, cards,
};
use crate::mana_cost;

/// An artifact spell you cast, which is the whole of the trigger: what it
/// does is not part of the condition.
static AN_ARTIFACT_SPELL_YOU_CAST: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Artifact),
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
]);

static ARTIFACTS_YOU_CONTROL: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Artifact),
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
]);

// MKM 57 — Forensic Gadgeteer
pub(in crate::card::sets) static FORENSIC_GADGETEER: CardRecord = CardRecord::new(
    cards::FORENSIC_GADGETEER,
    "Forensic Gadgeteer",
    CardArt::new("97d08a15-e61c-4421-a541-c68a4f87cb74", "Volkan Baǵa"),
    CardSet::MurdersAtKarlovManor,
    // Every artifact you cast is a card later, and every artifact you
    // already have is cheaper to use -- including the Clues it just made.
    CardRules::new_creature(mana_cost!("{2}{U}"), &["Vedalken", "Artificer", "Detective"], 2, 3)
        .with_abilities(&[
            AbilityDef::triggered(
                "Whenever you cast an artifact spell, investigate. (Create a Clue token. It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")",
                TriggerEventDef::SpellCast(AN_ARTIFACT_SPELL_YOU_CAST),
                EffectDef::CreateToken {
                    token: cards::CLUE_TOKEN,
                    count: ValueDef::Constant(1),
                    tapped: false,
                },
            ),
            AbilityDef::static_ability(
                "Activated abilities of artifacts you control cost {1} less to activate. This effect can't reduce the mana in that cost to less than one mana.",
                EffectDef::ReduceMatchingAbilityCostBy {
                    permanent: ARTIFACTS_YOU_CONTROL,
                    amount: ValueDef::Constant(1),
                    minimum: 1,
                },
            ),
        ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&FORENSIC_GADGETEER];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
