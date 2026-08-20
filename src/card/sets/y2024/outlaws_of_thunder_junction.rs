//! Outlaws of Thunder Junction cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, CardArt, CardRules, CardSet, CardSupertype,
    CardType, CounterKind, EffectDef, EffectRecipientDef, ObjectPredicateDef, PlayerRelation,
    TriggerEventDef, ValueDef, ZoneKind, cards,
};
use crate::{TargetIndex, mana_cost};

static BILL_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one_permanent(
    ObjectPredicateDef::HasType(CardType::Creature),
)];

static BILL_DOUBLE_COST: [AbilityCostDef; 1] = [AbilityCostDef::Mana(mana_cost!("{3}{G}{G}"))];

static BILL_ABILITIES: [AbilityDef; 2] = [
    AbilityDef::triggered_with_targets(
        "Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature.",
        TriggerEventDef::zone_changed(
            ObjectPredicateDef::All(&[
                ObjectPredicateDef::HasType(CardType::Land),
                ObjectPredicateDef::ControlledBy(PlayerRelation::You),
            ]),
            None,
            Some(ZoneKind::Battlefield),
        ),
        &BILL_TARGET,
        EffectDef::AddCounters {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            kind: CounterKind::PlusOnePlusOne,
            amount: ValueDef::Constant(1),
        },
    ),
    // Each creature doubles its own, so a board of one-counter creatures
    // gains one apiece and a single large one gains everything it has.
    AbilityDef::activated(
        "{3}{G}{G}: Double the number of +1/+1 counters on each creature you control.",
        &BILL_DOUBLE_COST,
        EffectDef::DoubleCounters {
            object: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::HasType(CardType::Creature),
                &[ZoneKind::Battlefield],
                PlayerRelation::You,
            ),
            kind: CounterKind::PlusOnePlusOne,
        },
    ),
];

// OTJ 157 — Bristly Bill, Spine Sower
pub(in crate::card::sets) static BRISTLY_BILL_SPINE_SOWER: CardRecord = CardRecord::new(
    cards::BRISTLY_BILL_SPINE_SOWER,
    "Bristly Bill, Spine Sower",
    CardArt::new("52eef0d6-24b7-40b7-8403-e8e863d0cd55", "Daniel Zrom"),
    CardSet::OutlawsOfThunderJunction,
    // The counters accumulate for free off lands, and then the activation
    // turns a slow board into a lethal one in a single turn.
    CardRules::new_creature(mana_cost!("{1}{G}"), &["Plant", "Druid"], 2, 2)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&BILL_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&BRISTLY_BILL_SPINE_SOWER];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
