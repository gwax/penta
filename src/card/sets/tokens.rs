//! Token definitions.
//!
//! A token is not a printed card, but it is a permanent with characteristics,
//! so it is cataloged like anything else. `CardSet::Token` belongs to no
//! format's allowed sets, which is what keeps a token out of every decklist
//! while still letting a client resolve one by definition.
//!
//! A token has no mana cost, so its colors come from a printed color rather
//! than from a cost, and it carries no art: a Scryfall identifier names a
//! printing, and the client already falls back to the type glyph without one.

use super::{CardRecord, PrintingRecord};
use crate::card::{CardArt, CardRules, CardSet, ManaColor, abilities, cards};

pub(in crate::card::sets) static BEAST_TOKEN_3_3_GREEN: CardRecord = CardRecord::new(
    cards::BEAST_TOKEN_3_3_GREEN,
    "Beast",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Beast"], 3, 3).printed_colors(&[ManaColor::Green]),
);

pub(in crate::card::sets) static KNIGHT_TOKEN_2_2_WHITE: CardRecord = CardRecord::new(
    cards::KNIGHT_TOKEN_2_2_WHITE,
    "Knight",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Knight"], 2, 2)
        .printed_colors(&[ManaColor::White])
        .with_abilities(&[abilities::vigilance()]),
);

pub(in crate::card::sets) static SOLDIER_TOKEN_1_1_RED_WHITE: CardRecord = CardRecord::new(
    cards::SOLDIER_TOKEN_1_1_RED_WHITE,
    "Soldier",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Soldier"], 1, 1)
        .printed_colors(&[ManaColor::Red, ManaColor::White])
        .with_abilities(&[abilities::haste()]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &BEAST_TOKEN_3_3_GREEN,
    &KNIGHT_TOKEN_2_2_WHITE,
    &SOLDIER_TOKEN_1_1_RED_WHITE,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
