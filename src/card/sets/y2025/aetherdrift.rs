//! Aetherdrift cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, CardArt, CardRules, CardSet, EffectDef, EffectRecipientDef, TopCardSelectionDef,
    ValueDef, ZoneKind, ZonePlacement, cards,
};
use crate::mana_cost;

/// Impulse's shape, one card deeper and one card wider. The rest going to
/// the bottom rather than the graveyard is what keeps it from being a
/// self-mill, which matters to the decks that play it.
static STOCK_UP_SELECTION: TopCardSelectionDef = TopCardSelectionDef {
    count: ValueDef::Constant(5),
    object: None,
    minimum: 2,
    maximum: 2,
    select_all_matching: false,
    reveal_selected: false,
    selected_zone: ZoneKind::Hand,
    selected_placement: ZonePlacement::Top,
    rest_zone: ZoneKind::Library,
    rest_placement: ZonePlacement::Bottom,
    selected_order_follows_choice: false,
    then: None,
};

// DFT 67 — Stock Up
pub(in crate::card::sets) static STOCK_UP: CardRecord = CardRecord::new(
    cards::STOCK_UP,
    "Stock Up",
    CardArt::new("0a786855-6eb4-42c0-a528-4842db46809d", "Izzy"),
    CardSet::Aetherdrift,
    // Two cards for three mana at sorcery speed is unremarkable; seeing five
    // to find them is what puts it in a deck built around one or two cards.
    CardRules::new_sorcery(mana_cost!("{2}{U}")).with_ability(AbilityDef::spell(
        "Look at the top five cards of your library. Put two of them into your hand and the rest on the bottom of your library in any order.",
        EffectDef::LookAtTopAndSelect {
            player: EffectRecipientDef::Controller,
            looker: EffectRecipientDef::Controller,
            selection: &STOCK_UP_SELECTION,
        },
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&STOCK_UP];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
