//! Dominaria United Commander card records required by supported formats.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityDef, CardArt, CardRules, CardSet, CardSupertype, CardType, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, TopCardSelectionDef, ValueDef, ZoneKind,
    ZonePlacement, abilities,
};
use crate::mana_cost;

// DMC 47 — Torsten, Founder of Benalia
/// "Any number", so the choice is real: a land you would rather not draw
/// later can be left to the bottom, which is the only reason the clause is
/// bounded rather than mandatory.
static TORSTEN_DIG: TopCardSelectionDef = TopCardSelectionDef {
    count: ValueDef::Constant(7),
    object: Some(ObjectPredicateDef::AnyOf(&[
        ObjectPredicateDef::HasType(CardType::Creature),
        ObjectPredicateDef::HasType(CardType::Land),
    ])),
    minimum: 0,
    maximum: 7,
    select_all_matching: false,
    select_one_of_each_type: false,
    // All seven are revealed, which is what the other player learns whether
    // or not any of them are taken.
    reveal_inspected: true,
    reveal_selected: false,
    counted: None,
    selected_zone: ZoneKind::Hand,
    selected_placement: ZonePlacement::Top,
    selected_hidden: false,
    selected_linked_to_source: false,
    selected_face_down: None,
    rest_zone: ZoneKind::Library,
    rest_placement: ZonePlacement::Bottom,
    // Random rather than an order you choose: what is left is not a plan for
    // later, which is part of why leaving anything is a real decision.
    rest_random_order: true,
    rest_counters: None,
    selected_order_follows_choice: false,
    then: None,
};

static TORSTEN_ABILITIES: [AbilityDef; 2] = [
    abilities::enters_trigger(
        "When Torsten enters, reveal the top seven cards of your library. Put any number of \
         creature and/or land cards from among them into your hand and the rest on the bottom of \
         your library in a random order.",
        EffectDef::LookAtTopAndSelect {
            player: EffectRecipientDef::Controller,
            looker: EffectRecipientDef::Controller,
            selection: &TORSTEN_DIG,
        },
    ),
    abilities::dies_trigger(
        "When Torsten dies, create seven 1/1 white Soldier creature tokens.",
        EffectDef::create_creature_token(&["Soldier"], &[ManaColor::White], 1, 1)
            .with_count(ValueDef::Constant(7))
            .with_art(CardArt::new(
                "8c4b0257-2ca5-4015-9d63-d7cf6e87ab9d",
                "Justine Cruz",
            )),
    ),
];

pub(in crate::card::sets) static TORSTEN_FOUNDER_OF_BENALIA: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("0783b426-a527-42c1-9271-be28b229e1c6"),
    "Torsten, Founder of Benalia",
    CardArt::new("0783b426-a527-42c1-9271-be28b229e1c6", "Volkan Baǵa"),
    CardSet::DominariaUnitedCommander,
    // Seven mana, and the two halves answer the two ways it goes wrong: it
    // refills your hand the turn it lands, and leaves seven bodies behind if
    // somebody kills it.
    CardRules::new_creature(mana_cost!("{5}{G}{W}"), &["Human", "Soldier"], 7, 7)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&TORSTEN_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&TORSTEN_FOUNDER_OF_BENALIA];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
