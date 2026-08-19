//! Streets of New Capenna cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{AbilityDef, CardArt, CardRules, CardSet, abilities, cards};
use crate::mana_cost;

/// A triome is a tapped land with three basic land types and cycling, and
/// nothing else. Its printed mana ability is reminder text for what the
/// subtypes already grant, so it is not restated as a clause.
const TRIOME_ABILITIES: &[AbilityDef] = &[
    abilities::enters_tapped("This land enters tapped."),
    abilities::cycling(
        "Cycling {3} ({3}, Discard this card: Draw a card.)",
        mana_cost!("{3}"),
    ),
];

const fn triome(types: &'static [&'static str]) -> CardRules {
    CardRules::new_land(types).with_abilities(TRIOME_ABILITIES)
}

// SNC 250 — Jetmir's Garden
pub(in crate::card::sets) static JETMIRS_GARDEN: CardRecord = CardRecord::new(
    cards::JETMIRS_GARDEN,
    "Jetmir's Garden",
    CardArt::new(
        "26d40e03-6de4-4373-9fbf-04c1dd79e995",
        "Kasia 'Kafis' Zielińska",
    ),
    CardSet::StreetsOfNewCapenna,
    triome(&["Mountain", "Forest", "Plains"]),
);

// SNC 254 — Raffine's Tower
pub(in crate::card::sets) static RAFFINES_TOWER: CardRecord = CardRecord::new(
    cards::RAFFINES_TOWER,
    "Raffine's Tower",
    CardArt::new("a2c56479-4bee-4edb-80d7-4af010b7c793", "Sam White"),
    CardSet::StreetsOfNewCapenna,
    triome(&["Plains", "Island", "Swamp"]),
);

// SNC 257 — Spara's Headquarters
pub(in crate::card::sets) static SPARAS_HEADQUARTERS: CardRecord = CardRecord::new(
    cards::SPARAS_HEADQUARTERS,
    "Spara's Headquarters",
    CardArt::new("7363f1fb-9af3-4212-921f-d59533faf0e5", "Kieran Yanner"),
    CardSet::StreetsOfNewCapenna,
    triome(&["Forest", "Plains", "Island"]),
);

// SNC 260 — Xander's Lounge
pub(in crate::card::sets) static XANDERS_LOUNGE: CardRecord = CardRecord::new(
    cards::XANDERS_LOUNGE,
    "Xander's Lounge",
    CardArt::new("54f449ff-4025-465e-9ec5-a5cf42c4c9d3", "James Paick"),
    CardSet::StreetsOfNewCapenna,
    triome(&["Island", "Swamp", "Mountain"]),
);

// SNC 261 — Ziatora's Proving Ground
pub(in crate::card::sets) static ZIATORAS_PROVING_GROUND: CardRecord = CardRecord::new(
    cards::ZIATORAS_PROVING_GROUND,
    "Ziatora's Proving Ground",
    CardArt::new("75fdce80-e338-4a50-bdc6-786511feaeef", "Viko Menezes"),
    CardSet::StreetsOfNewCapenna,
    triome(&["Swamp", "Mountain", "Forest"]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &JETMIRS_GARDEN,
    &RAFFINES_TOWER,
    &SPARAS_HEADQUARTERS,
    &XANDERS_LOUNGE,
    &ZIATORAS_PROVING_GROUND,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
