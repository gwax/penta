//! Rivals of Ixalan card records.

use super::{CardRecord, PrintingAnchor, PrintingRecord};

// RIX 101 — Fanatical Firebrand
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static FANATICAL_FIREBRAND: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("5e5565de-028c-4799-a9f6-4dcd685639eb"),
    "Fanatical Firebrand",
    crate::card::CardArt::new("d1296316-7781-4e98-95e6-7020648be6a5", "Wayne Reynolds"),
    crate::card::CardSet::RivalsOfIxalan,
    crate::card::CardRules::unsupported(),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&FANATICAL_FIREBRAND];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
