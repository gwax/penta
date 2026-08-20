//! Worldwake cards cataloged for the Vintage Cube.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, CardArt, CardChoiceSourceDef, CardRules, CardSet, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, TriggerEventDef, ValueDef, ZoneKind, ZonePlacement,
    cards,
};
use crate::mana_cost;

static AN_EQUIPMENT_IN_HAND: [CardChoiceSourceDef; 1] = [CardChoiceSourceDef::Zone(ZoneKind::Hand)];

/// The second half of the card, and the reason the first half is worth
/// finding: a minimum of zero is the printed "you may", and with no
/// Equipment in hand the choice is never offered at all.
static MYSTIC_PUT_EQUIPMENT_DOWN: EffectDef = EffectDef::ChooseCards {
    player: EffectRecipientDef::Controller,
    sources: &AN_EQUIPMENT_IN_HAND,
    object: ObjectPredicateDef::Subtype("Equipment"),
    minimum: 0,
    maximum: 1,
    reveal: false,
    destination: ZoneKind::Battlefield,
    placement: ZonePlacement::Top,
    // It arrives as itself: nothing about the Equipment changes on the way
    // down, and it is not attached to anything.
    arrival_effect: None,
};

// WWK 20 — Stoneforge Mystic
pub(in crate::card::sets) static STONEFORGE_MYSTIC: CardRecord = CardRecord::new(
    cards::STONEFORGE_MYSTIC,
    "Stoneforge Mystic",
    CardArt::new("19557351-b65f-4b04-b971-66abdc07000a", "Mike Bierek"),
    CardSet::Worldwake,
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Kor", "Artificer"], 1, 2)
        .with_abilities(&[
            AbilityDef::triggered(
                "When this creature enters, you may search your library for an Equipment card, reveal it, put it into your hand, then shuffle.",
                TriggerEventDef::zone_changed(
                    ObjectPredicateDef::Source,
                    None,
                    Some(ZoneKind::Battlefield),
                ),
                EffectDef::May {
                    player: EffectRecipientDef::Controller,
                    effect: &EffectDef::SearchZone {
                        player: EffectRecipientDef::Controller,
                        source: ZoneKind::Library,
                        object: ObjectPredicateDef::Subtype("Equipment"),
                        minimum: 0,
                        maximum: ValueDef::Constant(1),
                        reveal: true,
                        destination: ZoneKind::Hand,
                        placement: ZonePlacement::Top,
                        shuffle: true,
                        enters_tapped: false,
                        binding: None,
                        then: None,
                    },
                },
            ),
            AbilityDef::activated(
                "{1}{W}, {T}: You may put an Equipment card from your hand onto the battlefield.",
                &[
                    AbilityCostDef::Mana(mana_cost!("{1}{W}")),
                    AbilityCostDef::TapSource,
                ],
                MYSTIC_PUT_EQUIPMENT_DOWN,
            ),
        ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&STONEFORGE_MYSTIC];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
