//! Magic 2011 cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, CardArt, CardRules, CardSet, CardType, EffectDef, EffectRecipientDef,
    ObjectPredicateDef, TriggerEventDef, ValueDef, ZoneKind, ZonePlacement, abilities, cards,
};
use crate::mana_cost;

/// One printed ability with two ways in, not two abilities: the card says
/// "enters or attacks", and a Titan that does both in a turn triggers twice
/// for the same reason it would have anyway.
static ENTERS_OR_ATTACKS: [TriggerEventDef; 2] = [
    TriggerEventDef::zone_changed(
        ObjectPredicateDef::Source,
        None,
        Some(ZoneKind::Battlefield),
    ),
    TriggerEventDef::attacks(ObjectPredicateDef::Source),
];

/// Any land card, not just a basic: the two it finds are usually the two the
/// deck was built around.
static FETCH_TWO_LANDS: EffectDef = EffectDef::SearchZone {
    player: EffectRecipientDef::Controller,
    source: ZoneKind::Library,
    object: ObjectPredicateDef::HasType(CardType::Land),
    minimum: 0,
    maximum: ValueDef::Constant(2),
    reveal: false,
    destination: ZoneKind::Battlefield,
    placement: ZonePlacement::Top,
    shuffle: true,
    enters_tapped: true,
    binding: None,
    then: None,
};

// M11 192 — Primeval Titan
pub(in crate::card::sets) static PRIMEVAL_TITAN: CardRecord = CardRecord::new(
    cards::PRIMEVAL_TITAN,
    "Primeval Titan",
    CardArt::new("feee9327-b937-46ba-a2aa-6c015ab6cdd5", "Aleksi Briclot"),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{4}{G}{G}"), &["Giant"], 6, 6).with_abilities(&[
        abilities::trample(),
        AbilityDef::triggered(
            "Whenever this creature enters or attacks, you may search your library for up to two land cards, put them onto the battlefield tapped, then shuffle.",
            TriggerEventDef::AnyOf(&ENTERS_OR_ATTACKS),
            EffectDef::May {
                player: EffectRecipientDef::Controller,
                effect: &FETCH_TWO_LANDS,
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&PRIMEVAL_TITAN];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
