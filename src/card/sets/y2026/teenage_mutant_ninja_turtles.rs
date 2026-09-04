//! TLE card records required by supported formats.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{CardArt, CardRules, CardSet, CardType, abilities};
use crate::mana_cost;

// TLE 276 — Wolf Cove Villager
pub(in crate::card::sets) static WOLF_COVE_VILLAGER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("7dbeced9-d27d-476c-92d4-3c14d8a40458"),
    "Wolf Cove Villager",
    CardArt::new("993652d5-b44b-4142-a081-427edb480dcf", "Gemi"),
    CardSet::TeenageMutantNinjaTurtles,
    // A 2/2 for one, paid for entirely by arriving tapped: it blocks the
    // turn after it lands and never the turn it does.
    CardRules::new_creature(mana_cost!("{W}"), &["Human", "Peasant"], 2, 2)
        .with_ability(abilities::enters_tapped(CardType::Creature)),
);

// TLE 285 — Warship Scout
pub(in crate::card::sets) static WARSHIP_SCOUT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("b1a95982-be16-465a-9c1b-1f4d875c0c40"),
    "Warship Scout",
    CardArt::new("f47fc407-5b7d-4c9d-90b4-3eb234f9f18b", "Brandon L. Hunt"),
    CardSet::TeenageMutantNinjaTurtles,
    // A vanilla 2/1 for one: nothing is missing from the definition, the
    // card simply prints no rules text.
    CardRules::new_creature(mana_cost!("{R}"), &["Human", "Scout"], 2, 1),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&WOLF_COVE_VILLAGER, &WARSHIP_SCOUT];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
