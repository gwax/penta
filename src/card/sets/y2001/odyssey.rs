//! Odyssey cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AddManaEffectDef,
    CardArt, CardRules, CardSet, ComparisonDef, EffectDef, EffectRecipientDef, ManaColor,
    ObjectPredicateDef, ObjectQueryDef, PlayerRelation, TriggerConditionDef, ValueDef, ZoneKind,
    ZonePlacement, cards,
};
use crate::{TargetIndex, mana_cost};

// ODY 113 — Upheaval
pub(in crate::card::sets) static UPHEAVAL: CardRecord = CardRecord::new(
    cards::UPHEAVAL,
    "Upheaval",
    CardArt::new("9e201229-34a6-48c8-a07c-d8aefcf5f8a7", "Kev Walker"),
    CardSet::Odyssey,
    CardRules::new_sorcery(mana_cost!("{4}{U}{U}")).with_ability(AbilityDef::spell(
        "Return all permanents to their owners' hands.",
        EffectDef::MoveToZone {
            object: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::Any,
                &[ZoneKind::Battlefield],
                PlayerRelation::Any,
            ),
            zone: ZoneKind::Hand,
            placement: ZonePlacement::Top,
            arrival_effect: None,
            controller: None,
        },
    )),
);

/// Threshold: seven or more cards in your own graveyard. The count is of
/// cards you own, not of every graveyard on the table.
static YOUR_GRAVEYARD: ObjectQueryDef = ObjectQueryDef::owned_by(
    ObjectPredicateDef::Any,
    &[ZoneKind::Graveyard],
    crate::card::PlayerSetDef::Related(PlayerRelation::You),
);

static THRESHOLD: TriggerConditionDef = TriggerConditionDef::ObjectCount {
    query: YOUR_GRAVEYARD,
    comparison: ComparisonDef::GreaterOrEqual,
    amount: 7,
};

static BARBARIAN_RING_SHOT: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::AnyTarget,
)];

// ODY 313 — Barbarian Ring
pub(in crate::card::sets) static BARBARIAN_RING: CardRecord = CardRecord::new(
    cards::BARBARIAN_RING,
    "Barbarian Ring",
    CardArt::new("1809361e-ae1a-4c47-8464-e6496e94d962", "John Avon"),
    CardSet::Odyssey,
    // The land costs a life every time it makes mana, and pays that back once
    // the graveyard is deep enough to turn it into a burn spell.
    CardRules::new_land(&[]).with_abilities(&[
        AbilityDef::activated_mana(
            "{T}: Add {R}. This land deals 1 damage to you.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::one(ManaColor::Red).with_damage_to_controller(1)),
        ),
        AbilityDef::activated_with_targets(
            "Threshold — {R}, {T}, Sacrifice this land: It deals 2 damage to any target. Activate only if there are seven or more cards in your graveyard.",
            &[
                AbilityCostDef::Mana(mana_cost!("{R}")),
                AbilityCostDef::TapSource,
                AbilityCostDef::SacrificeSource,
            ],
            &BARBARIAN_RING_SHOT,
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(2),
            },
        )
        .with_activation_condition(&THRESHOLD),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&UPHEAVAL, &BARBARIAN_RING];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
