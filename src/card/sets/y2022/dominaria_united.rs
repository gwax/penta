//! Dominaria United cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, CardArt, CardRules, CardSet, CardSupertype, EffectDef, EffectRecipientDef,
    PlayerRelation, TriggerEventDef, ValueDef, abilities, cards,
};
use crate::mana_cost;

/// Two clauses rather than one symmetrical one, because they are not
/// symmetrical: yours gains and theirs loses, and a card that made both
/// players lose would read very differently.
static SHEOLDRED_ABILITIES: [AbilityDef; 3] = [
    abilities::deathtouch(),
    AbilityDef::triggered(
        "Whenever you draw a card, you gain 2 life.",
        TriggerEventDef::DrewCard(PlayerRelation::You),
        EffectDef::GainLife {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(2),
        },
    ),
    AbilityDef::triggered(
        "Whenever an opponent draws a card, they lose 2 life.",
        TriggerEventDef::DrewCard(PlayerRelation::Opponent),
        EffectDef::LoseLife {
            recipient: EffectRecipientDef::EventPlayer,
            amount: ValueDef::Constant(2),
        },
    ),
];

// DMU 107 — Sheoldred, the Apocalypse
pub(in crate::card::sets) static SHEOLDRED_THE_APOCALYPSE: CardRecord = CardRecord::new(
    cards::SHEOLDRED_THE_APOCALYPSE,
    "Sheoldred, the Apocalypse",
    CardArt::new("d67be074-cdd4-41d9-ac89-0a0456c4e4b2", "Chris Rahn"),
    CardSet::DominariaUnited,
    // A four-mana 4/5 deathtouch would be playable on its own. The draw
    // clauses are what make it unanswerable: the opponent's own draw step
    // pays for it, every turn it survives.
    CardRules::new_creature(mana_cost!("{2}{B}{B}"), &["Phyrexian", "Praetor"], 4, 5)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&SHEOLDRED_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&SHEOLDRED_THE_APOCALYPSE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
