//! Stronghold cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, CardArt, CardRules, CardSet, ObjectPredicateDef, ValueDef,
    abilities, cards,
};
use crate::mana_cost;

// STH 36 — Mana Leak
pub(in crate::card::sets) static MANA_LEAK: CardRecord = CardRecord::new(
    cards::MANA_LEAK,
    "Mana Leak",
    CardArt::new("abcaf16d-aa02-43e2-aa38-bb1835d47a05", "Christopher Rush"),
    CardSet::Stronghold,
    CardRules::new_instant(mana_cost!("{1}{U}")).with_ability(AbilityDef::spell_with_targets(
        "Counter target spell unless its controller pays {3}.",
        &[AbilityTargetDef::exactly_one_spell(ObjectPredicateDef::Any)],
        abilities::counter_target_unless_paid(ValueDef::Constant(3)),
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&MANA_LEAK];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
