//! Planar Chaos cards cataloged as cross-format rules-engine test cases.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, BasicLandType, CardArt, CardRules, CardSet, CardSupertype,
    CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef, PlayerRelation, ZoneKind, cards,
};

// PLC 165 — Urborg, Tomb of Yawgmoth
pub(in crate::card::sets) static URBORG_TOMB_OF_YAWGMOTH: CardRecord = CardRecord::new(
    cards::URBORG_TOMB_OF_YAWGMOTH,
    "Urborg, Tomb of Yawgmoth",
    CardArt::new("19e1224f-82cb-4f41-8739-f880cba61bbb", "John Avon"),
    CardSet::PlanarChaos,
    CardRules::new_land(&[])
        .with_supertype(CardSupertype::Legendary)
        .with_ability(AbilityDef::static_ability(
            "Each land is a Swamp in addition to its other land types.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::matching_objects(
                    ObjectPredicateDef::HasType(CardType::Land),
                    &[ZoneKind::Battlefield],
                    PlayerRelation::Any,
                ),
                effect: AppliedEffectDef::add_basic_land_types(&[BasicLandType::Swamp]),
            },
        )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&URBORG_TOMB_OF_YAWGMOTH];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
