//! Core Set 2021 card records.

use super::{CardRecord, PrintingAnchor, PrintingRecord};

// M21 71 — Shipwreck Dowser
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SHIPWRECK_DOWSER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("59d38ef7-5017-4ea3-b97f-a8fe12d03e98"),
    "Shipwreck Dowser",
    crate::card::CardArt::new("1f20fe3d-792a-4030-a25c-e81b48b2bcb4", "Caroline Gariba"),
    crate::card::CardSet::CoreSet2021,
    crate::card::CardRules::unsupported(),
);

// M21 126 — Village Rites
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static VILLAGE_RITES: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("9c0f60a6-b5c8-4704-8b61-94e8fc463e5d"),
    "Village Rites",
    crate::card::CardArt::new("0fab9ee8-776a-48e5-b309-bcd381e67bf7", "Igor Kieryluk"),
    crate::card::CardSet::CoreSet2021,
    crate::card::CardRules::unsupported(),
);

// M21 193 — Llanowar Visionary
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static LLANOWAR_VISIONARY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("880c9523-717e-4903-a09e-d6c47614383d"),
    "Llanowar Visionary",
    crate::card::CardArt::new("c2635b0c-c990-4cce-9ac4-97602a757cf0", "Cristi Balanescu"),
    crate::card::CardSet::CoreSet2021,
    crate::card::CardRules::unsupported(),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&SHIPWRECK_DOWSER, &VILLAGE_RITES, &LLANOWAR_VISIONARY];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
