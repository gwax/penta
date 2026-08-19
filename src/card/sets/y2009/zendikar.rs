//! Zendikar cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, BasicLandType, CardArt, CardRules, CardSet,
    CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef, ValueDef, ZoneKind, ZonePlacement,
    abilities, cards,
};
use crate::mana_cost;

/// The five allied fetchlands of Onslaught got an enemy-coloured cycle here,
/// with the same text. One helper states it once; only the two land types and
/// the order they are named in differ.
const fn fetch_land(text: &'static str, land_types: &'static [BasicLandType]) -> CardRules {
    CardRules::new_land(&[]).with_ability(AbilityDef::activated(
        text,
        &[
            AbilityCostDef::TapSource,
            AbilityCostDef::PayLife(1),
            AbilityCostDef::SacrificeSource,
        ],
        EffectDef::SearchZone {
            player: EffectRecipientDef::Controller,
            source: ZoneKind::Library,
            object: ObjectPredicateDef::HasAnyBasicLandType(land_types),
            minimum: 0,
            maximum: ValueDef::Constant(1),
            reveal: false,
            destination: ZoneKind::Battlefield,
            placement: ZonePlacement::Top,
            shuffle: true,
            enters_tapped: false,
            binding: None,
            then: None,
        },
    ))
}

// ZEN 67 — Spell Pierce
pub(in crate::card::sets) static SPELL_PIERCE: CardRecord = CardRecord::new(
    cards::SPELL_PIERCE,
    "Spell Pierce",
    CardArt::new("cb3d3901-e4a6-45ab-a7b5-c65d91e1875e", "Vance Kovacs"),
    CardSet::Zendikar,
    CardRules::new_instant(mana_cost!("{U}")).with_ability(AbilityDef::spell_with_targets(
        "Counter target noncreature spell unless its controller pays {2}.",
        &[AbilityTargetDef::exactly_one_spell(
            ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Creature)),
        )],
        abilities::counter_target_unless_paid(ValueDef::Constant(2)),
    )),
);

// ZEN 211 — Arid Mesa
pub(in crate::card::sets) static ARID_MESA: CardRecord = CardRecord::new(
    cards::ARID_MESA,
    "Arid Mesa",
    CardArt::new("16c8d2fa-54a7-46e8-980c-905258497c90", "Raymond Swanland"),
    CardSet::Zendikar,
    fetch_land(
        "{T}, Pay 1 life, Sacrifice this land: Search your library for a Mountain or Plains card, put it onto the battlefield, then shuffle.",
        &[BasicLandType::Mountain, BasicLandType::Plains],
    ),
);

// ZEN 219 — Marsh Flats
pub(in crate::card::sets) static MARSH_FLATS: CardRecord = CardRecord::new(
    cards::MARSH_FLATS,
    "Marsh Flats",
    CardArt::new("45026d57-0324-4312-8b86-2e7d4f581ee9", "Izzy"),
    CardSet::Zendikar,
    fetch_land(
        "{T}, Pay 1 life, Sacrifice this land: Search your library for a Plains or Swamp card, put it onto the battlefield, then shuffle.",
        &[BasicLandType::Plains, BasicLandType::Swamp],
    ),
);

// ZEN 220 — Misty Rainforest
pub(in crate::card::sets) static MISTY_RAINFOREST: CardRecord = CardRecord::new(
    cards::MISTY_RAINFOREST,
    "Misty Rainforest",
    CardArt::new("24a5cc2c-0fbf-4a5f-b175-6e0ffd0d0787", "Shelly Wan"),
    CardSet::Zendikar,
    fetch_land(
        "{T}, Pay 1 life, Sacrifice this land: Search your library for a Forest or Island card, put it onto the battlefield, then shuffle.",
        &[BasicLandType::Forest, BasicLandType::Island],
    ),
);

// ZEN 223 — Scalding Tarn
pub(in crate::card::sets) static SCALDING_TARN: CardRecord = CardRecord::new(
    cards::SCALDING_TARN,
    "Scalding Tarn",
    CardArt::new("327cf118-cc92-4073-85d0-94d2a0a6989a", "Philip Straub"),
    CardSet::Zendikar,
    fetch_land(
        "{T}, Pay 1 life, Sacrifice this land: Search your library for an Island or Mountain card, put it onto the battlefield, then shuffle.",
        &[BasicLandType::Island, BasicLandType::Mountain],
    ),
);

// ZEN 229 — Verdant Catacombs
pub(in crate::card::sets) static VERDANT_CATACOMBS: CardRecord = CardRecord::new(
    cards::VERDANT_CATACOMBS,
    "Verdant Catacombs",
    CardArt::new("7abd2723-2851-4f1a-b2d0-dfcb526472c3", "Vance Kovacs"),
    CardSet::Zendikar,
    fetch_land(
        "{T}, Pay 1 life, Sacrifice this land: Search your library for a Swamp or Forest card, put it onto the battlefield, then shuffle.",
        &[BasicLandType::Swamp, BasicLandType::Forest],
    ),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &SPELL_PIERCE,
    &ARID_MESA,
    &MARSH_FLATS,
    &MISTY_RAINFOREST,
    &SCALDING_TARN,
    &VERDANT_CATACOMBS,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
