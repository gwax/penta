//! Oath of the Gatewatch card records.

use super::{CardRecord, PrintingAnchor, PrintingRecord};

// OGW 141 — Pulse of Murasa
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static PULSE_OF_MURASA: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("c0c8057f-b45b-4f67-90cd-c808b5e9cbfa"),
    "Pulse of Murasa",
    crate::card::CardArt::new("c591c615-69e8-4661-a089-8c4e152adac7", "Matt Stewart"),
    crate::card::CardSet::OathOfTheGatewatch,
    crate::card::CardRules::unsupported(),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&PULSE_OF_MURASA];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
