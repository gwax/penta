//! Commander 2014 cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, CardArt, CardRules, CardSet, CardType, ObjectPredicateDef, PlayerRelation,
    ReplacementEffectDef, ReplacementEventDef, ZoneKind, abilities, cards,
};
use crate::mana_cost;

/// A nontoken creature that was not cast. Tokens are exempt because the card
/// says so; everything else that arrives without going through the stack --
/// reanimation, Show and Tell, a fetched Natural Order target -- is not.
static AN_UNCAST_CREATURE: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::Not(&ObjectPredicateDef::Token),
]);

// C14 5 — Containment Priest
pub(in crate::card::sets) static CONTAINMENT_PRIEST: CardRecord = CardRecord::new(
    cards::CONTAINMENT_PRIEST,
    "Containment Priest",
    CardArt::new("c2c794b9-09da-49be-b258-b0e21f1663e3", "John Stanko"),
    CardSet::Commander2014,
    // Flash is half the card: it is held up like a counterspell and answers
    // the reanimation on the stack rather than the creature on the board.
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Human", "Cleric"], 2, 2).with_abilities(&[
        abilities::flash(),
        AbilityDef::replacement_for(
            "If a nontoken creature would enter and it wasn't cast, exile it instead.",
            ReplacementEventDef::ObjectEntersBattlefield {
                object: AN_UNCAST_CREATURE,
                controller: PlayerRelation::Any,
                cast: Some(false),
            },
            ReplacementEffectDef::MoveToZone(ZoneKind::Exile),
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&CONTAINMENT_PRIEST];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
