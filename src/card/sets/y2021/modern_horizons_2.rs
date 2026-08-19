//! Modern Horizons 2 cards cataloged as cross-format rules-engine test cases.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, BasicLandType, CardArt, CardRules, CardSet, CardSupertype,
    CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef, ObjectQueryDef, PlayerRelation,
    ValueDef, ZoneKind, abilities, cards,
};
use crate::mana_cost;

/// "Artifact and/or enchantment" is one query rather than two sums: a
/// permanent that is both is counted once, and Nettlecyst counts itself.
static ARTIFACTS_AND_ENCHANTMENTS_YOU_CONTROL: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::AnyOf(&[
        ObjectPredicateDef::HasType(CardType::Artifact),
        ObjectPredicateDef::HasType(CardType::Enchantment),
    ]),
    &[ZoneKind::Battlefield],
    PlayerRelation::You,
);

// MH2 231 — Nettlecyst
pub(in crate::card::sets) static NETTLECYST: CardRecord = CardRecord::new(
    cards::NETTLECYST,
    "Nettlecyst",
    CardArt::new("4a0bb5dc-75a6-4bd6-81f8-611197fb0fba", "Vincent Proce"),
    CardSet::ModernHorizons2,
    CardRules::new_artifact(mana_cost!("{3}"))
        .with_subtypes(&["Equipment"])
        .with_abilities(&[
            abilities::living_weapon(cards::GERM_TOKEN_0_0_BLACK),
            AbilityDef::static_ability(
                "Equipped creature gets +1/+1 for each artifact and/or enchantment you control.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::modify_power_toughness(
                        ValueDef::CountMatchingObjects(&ARTIFACTS_AND_ENCHANTMENTS_YOU_CONTROL),
                        ValueDef::CountMatchingObjects(&ARTIFACTS_AND_ENCHANTMENTS_YOU_CONTROL),
                    ),
                },
            ),
            abilities::equip(mana_cost!("{2}"), "Equip {2}"),
        ]),
);

// MH2 261 — Yavimaya, Cradle of Growth
pub(in crate::card::sets) static YAVIMAYA_CRADLE_OF_GROWTH: CardRecord = CardRecord::new(
    cards::YAVIMAYA_CRADLE_OF_GROWTH,
    "Yavimaya, Cradle of Growth",
    CardArt::new("4e4b6e22-93b2-4896-bba5-0ceaa5d8ea3c", "Sarah Finnigan"),
    CardSet::ModernHorizons2,
    CardRules::new_land(&[])
        .with_supertype(CardSupertype::Legendary)
        .with_ability(AbilityDef::static_ability(
            "Each land is a Forest in addition to its other land types.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::matching_objects(
                    ObjectPredicateDef::HasType(CardType::Land),
                    &[ZoneKind::Battlefield],
                    PlayerRelation::Any,
                ),
                effect: AppliedEffectDef::add_basic_land_types(&[BasicLandType::Forest]),
            },
        )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&NETTLECYST, &YAVIMAYA_CRADLE_OF_GROWTH];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
