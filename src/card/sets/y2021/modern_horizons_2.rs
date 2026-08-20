//! Modern Horizons 2 cards cataloged as cross-format rules-engine test cases.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef, AppliedEffectDef,
    BasicLandType, CardArt, CardRules, CardSet, CardSupertype, CardType, DividedTotal, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, ObjectQueryDef, PlayerRelation,
    SpellAdditionalCostDef, SpendModeDef, TriggerEventDef, ValueDef, ZoneKind, abilities, cards,
};
use crate::{TargetIndex, mana_cost};

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

/// Four damage split however the caster likes, over creatures and
/// planeswalkers alike. Every target must be assigned at least one, so four
/// is the most it can ever cover.
static FURY_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef {
    predicate: AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Creature),
            ObjectPredicateDef::HasType(CardType::Planeswalker),
        ]),
        zones: &[ZoneKind::Battlefield],
        controller: None,
        owner: None,
    },
    minimum: 1,
    maximum: AbilityTargetDef::UNLIMITED,
    divided_total: Some(DividedTotal::Fixed(4)),
}];

static EXILE_A_RED_CARD: SpellAdditionalCostDef =
    SpellAdditionalCostDef::new(ObjectPredicateDef::Color(ManaColor::Red), ZoneKind::Hand, 1)
        .spent(SpendModeDef::Exile);

static FURY_ABILITIES: [AbilityDef; 4] = [
    abilities::double_strike(),
    AbilityDef::triggered_with_targets(
        "When this creature enters, it deals 4 damage divided as you choose among any number of target creatures and/or planeswalkers.",
        TriggerEventDef::zone_changed(
            ObjectPredicateDef::Source,
            None,
            Some(ZoneKind::Battlefield),
        ),
        &FURY_TARGETS,
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::DividedAmongTargets,
        },
    ),
    AbilityDef::alternative_cast(
        mana_cost!("{0}"),
        AlternativeCastKindDef::AlternativeCost,
        Some("Evoke—Exile a red card from your hand."),
        EffectDef::None,
    )
    .with_alternative_additional_cost(&EXILE_A_RED_CARD),
    // Evoke's own sacrifice. It is a separate trigger because it happens
    // after the Elemental has arrived, alongside the damage trigger rather
    // than instead of it -- which is why an evoked Fury still burns.
    abilities::evoke_sacrifice("When this creature enters, if it was evoked, sacrifice it."),
];

// MH2 126 — Fury
pub(in crate::card::sets) static FURY: CardRecord = CardRecord::new(
    cards::FURY,
    "Fury",
    CardArt::new("bd281158-8180-40b9-a5b7-03cfc712d81a", "Raoul Vitale"),
    CardSet::ModernHorizons2,
    CardRules::new_creature(mana_cost!("{3}{R}{R}"), &["Elemental", "Incarnation"], 3, 3)
        .with_abilities(&FURY_ABILITIES),
);

// MH2 202 — Grist, the Hunger Tide
// Audit: blocked — Needs three capabilities at once: a resolution loop that repeats a step while reading what the previous iteration milled, a reflexive triggered ability that chooses its target when the optional sacrifice is actually made rather than on activation, and characteristics that apply in every zone except the battlefield.

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

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&FURY, &NETTLECYST, &YAVIMAYA_CRADLE_OF_GROWTH];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
