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
use crate::card::{CardArt, CardRules, CardSet, ManaColor, cards};

pub(in crate::card::sets) static BEAST_TOKEN_3_3_GREEN: CardRecord = CardRecord::new(
    cards::BEAST_TOKEN_3_3_GREEN,
    "Beast",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Beast"], 3, 3).printed_colors(&[ManaColor::Green]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&BEAST_TOKEN_3_3_GREEN];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
