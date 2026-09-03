//! Starter 1999 card records.

use super::{CardRecord, PrintingAnchor, PrintingRecord};

// S99 15 — Eager Cadet
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static EAGER_CADET: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("d1e1ce2f-d8af-4fd0-975e-9d910d12b883"),
    "Eager Cadet",
    crate::card::CardArt::new(
        "46b89ce6-8a73-4e27-8696-e65ea0c16925",
        "Greg Hildebrandt & Tim Hildebrandt",
    ),
    crate::card::CardSet::Starter1999,
    crate::card::CardRules::unsupported(),
);

// S99 59 — Vizzerdrix
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static VIZZERDRIX: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("25711022-7270-4335-a48b-9f2b8275ceeb"),
    "Vizzerdrix",
    crate::card::CardArt::new("249ecab6-e145-4dfd-9e9e-56492db30b4c", "Dave Dorman"),
    crate::card::CardSet::Starter1999,
    crate::card::CardRules::unsupported(),
);

// S99 71 — Dakmor Lancer
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static DAKMOR_LANCER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("9d012ddf-abe1-4de9-89cb-78d82afb9e7b"),
    "Dakmor Lancer",
    crate::card::CardArt::new("660cc594-63f5-4819-a556-7a9484145f72", "Luca Zontini"),
    crate::card::CardSet::Starter1999,
    crate::card::CardRules::unsupported(),
);

// S99 99 — Goblin Chariot
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static GOBLIN_CHARIOT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("9ca11a7e-17f8-419f-9ba8-1bcaa3860f8b"),
    "Goblin Chariot",
    crate::card::CardArt::new("1db520e2-9926-45d2-a140-37b119b88106", "John Howe"),
    crate::card::CardSet::Starter1999,
    crate::card::CardRules::unsupported(),
);

// S99 120 — Trained Orgg
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static TRAINED_ORGG: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("425540b0-c826-4814-b0df-032264b1c237"),
    "Trained Orgg",
    crate::card::CardArt::new(
        "14a83031-8b57-41d2-b586-bb4dcf16136a",
        "Alex Horley-Orlandelli",
    ),
    crate::card::CardSet::Starter1999,
    crate::card::CardRules::unsupported(),
);

// S99 139 — Pride of Lions
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static PRIDE_OF_LIONS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f5006984-8e3d-4f13-b12e-1fbecd134bb3"),
    "Pride of Lions",
    crate::card::CardArt::new("1673b038-97b6-4139-8468-9cbbd01dd239", "Gary Ruddell"),
    crate::card::CardSet::Starter1999,
    crate::card::CardRules::unsupported(),
);

// S99 143 — Squall
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SQUALL: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("63c1b2f6-e47f-4f18-a94a-1d08eb009ef3"),
    "Squall",
    crate::card::CardArt::new("e5409b54-66ed-4add-bf43-cfeb074b1c50", "Val Mayerik"),
    crate::card::CardSet::Starter1999,
    crate::card::CardRules::unsupported(),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &EAGER_CADET,
    &VIZZERDRIX,
    &DAKMOR_LANCER,
    &GOBLIN_CHARIOT,
    &TRAINED_ORGG,
    &PRIDE_OF_LIONS,
    &SQUALL,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
