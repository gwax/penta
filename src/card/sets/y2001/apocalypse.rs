//! Apocalypse cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, CardArt, CardRules, CardSet, ObjectPredicateDef, cards,
};
use crate::mana_cost;

// APC 126 — Vindicate
pub(in crate::card::sets) static VINDICATE: CardRecord = CardRecord::new(
    cards::VINDICATE,
    "Vindicate",
    CardArt::new("2a1bfefd-dae8-49e9-9d56-cc852e3dc93b", "Brian Snõddy"),
    CardSet::Apocalypse,
    CardRules::new_sorcery(mana_cost!("{1}{W}{B}")).with_ability(AbilityDef::destroy_target(
        "Destroy target permanent.",
        &AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::Any),
        true,
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&VINDICATE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
