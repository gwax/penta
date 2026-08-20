//! Conspiracy cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, CardArt, CardRules, CardSet, CardType, EffectDef, ObjectPredicateDef, cards,
};
use crate::mana_cost;

/// "A nonland permanent you don't control" is read against the spell's
/// controller for every voter, so both players choose from the same ballot.
/// The vote machinery supplies the "you don't control" half.
static JUDGMENT_BALLOT: ObjectPredicateDef =
    ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Land));

// CNS 16 — Council's Judgment
pub(in crate::card::sets) static COUNCILS_JUDGMENT: CardRecord = CardRecord::new(
    cards::COUNCILS_JUDGMENT,
    "Council's Judgment",
    CardArt::new("17f28b16-da65-41a8-ba4f-f1c5e104aad6", "Kev Walker"),
    CardSet::Conspiracy,
    // Exiling without targeting is what it is played for: shroud, hexproof,
    // and protection are all no answer at all. Two players usually means two
    // permanents, since a disagreement ties.
    CardRules::new_sorcery(mana_cost!("{1}{W}{W}")).with_ability(AbilityDef::spell(
        "Will of the council — Starting with you, each player votes for a nonland permanent you don't control. Exile each permanent with the most votes or tied for most votes.",
        EffectDef::VoteForPermanentToExile {
            object: JUDGMENT_BALLOT,
        },
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&COUNCILS_JUDGMENT];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
