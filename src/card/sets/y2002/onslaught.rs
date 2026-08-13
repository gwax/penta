//! Onslaught cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, BasicLandType, CardArt, CardRules, CardSet,
    CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef, ZoneKind, cards,
};
use crate::mana_cost;

// ONS 275 — Naturalize
pub(in crate::card::sets) static NATURALIZE: CardRecord = CardRecord::new(
    cards::NATURALIZE,
    "Naturalize",
    CardArt::new("c0acc41f-b55b-47cb-8803-d39d72788799", "Ron Spears"),
    CardSet::Onslaught,
    CardRules::new_instant(mana_cost!("{1}{G}")).with_ability(AbilityDef::destroy_target(
        "Destroy target artifact or enchantment.",
        &AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Artifact),
            ObjectPredicateDef::HasType(CardType::Enchantment),
        ])),
        true,
    )),
);

// ONS 316 — Flooded Strand
pub(in crate::card::sets) static FLOODED_STRAND: CardRecord = CardRecord::new(
    cards::FLOODED_STRAND,
    "Flooded Strand",
    CardArt::new("b4e3d844-d3b4-41d8-921d-c1cb3af343f8", "Rob Alexander"),
    CardSet::Onslaught,
    fetch_land(
        "{T}, Pay 1 life, Sacrifice this land: Search your library for a Plains or Island card, put it onto the battlefield, then shuffle.",
        &[BasicLandType::Plains, BasicLandType::Island],
    ),
);

// ONS 330 — Wooded Foothills
pub(in crate::card::sets) static WOODED_FOOTHILLS: CardRecord = CardRecord::new(
    cards::WOODED_FOOTHILLS,
    "Wooded Foothills",
    CardArt::new("cdad38f7-9dfa-4f1b-9fac-41ab2b253f53", "Rob Alexander"),
    CardSet::Onslaught,
    fetch_land(
        "{T}, Pay 1 life, Sacrifice this land: Search your library for a Mountain or Forest card, put it onto the battlefield, then shuffle.",
        &[BasicLandType::Mountain, BasicLandType::Forest],
    ),
);

const fn fetch_land(text: &'static str, land_types: &'static [BasicLandType]) -> CardRules {
    CardRules::new_land(&[]).with_ability(AbilityDef::activated(
        text,
        &[
            AbilityCostDef::TapSource,
            AbilityCostDef::PayLife(1),
            AbilityCostDef::SacrificeSource,
        ],
        EffectDef::SearchLibrary {
            player: EffectRecipientDef::Controller,
            object: ObjectPredicateDef::HasAnyBasicLandType(land_types),
            destination: ZoneKind::Battlefield,
        },
    ))
}

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&NATURALIZE, &FLOODED_STRAND, &WOODED_FOOTHILLS];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
