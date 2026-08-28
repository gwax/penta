//! Fallout cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, CardArt, CardRules, CardSet, CardType, CopyExceptionsDef, CounterKind, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, PlayerRelation, TriggerEventDef, ValueDef, ZoneKind,
    abilities,
};
use crate::mana_cost;

// PIP 23 — Securitron Squadron
/// A creature token you control arriving, whichever ability made it -- this
/// card's own squad copies included, if squad ever pays.
static A_CREATURE_TOKEN_YOU_CONTROL: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::Token,
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
]);

/// "Create that many tokens that are copies of it": the count is how many
/// times the squad cost was paid, which the permanent carries over from the
/// cast that made it.
static SECURITRON_SQUAD_COPIES: EffectDef =
    EffectDef::create_token_from_copy(&crate::card::TokenCopyDef {
        object: &EffectRecipientDef::Source,
        exceptions: CopyExceptionsDef::NONE,
    })
    .with_count(ValueDef::TimesAdditionalCostPaid);

static SECURITRON_SQUADRON_ABILITIES: [AbilityDef; 4] = [
    abilities::squad(mana_cost!("{3}")),
    abilities::vigilance(),
    abilities::enters_trigger(
        "When this creature enters, create that many tokens that are copies of it.",
        SECURITRON_SQUAD_COPIES,
    ),
    AbilityDef::triggered(
        "Whenever a creature token you control enters, put a +1/+1 counter on it.",
        TriggerEventDef::zone_changed(
            A_CREATURE_TOKEN_YOU_CONTROL,
            None,
            Some(ZoneKind::Battlefield),
        ),
        EffectDef::AddCounters {
            object: EffectRecipientDef::TriggeringObject,
            kind: CounterKind::PlusOnePlusOne,
            amount: ValueDef::Constant(1),
        },
    ),
];

pub(in crate::card::sets) static SECURITRON_SQUADRON: CardRecord = CardRecord::new_with_legacy_id(
    2151,
    "Securitron Squadron",
    CardArt::new("b689a206-aec3-4a31-95cf-3d4b840db04c", "Jonas De Ro"),
    CardSet::Fallout,
    CardRules::new_artifact_creature(mana_cost!("{1}{W}"), &["Robot"], 2, 2)
        .with_abilities(&SECURITRON_SQUADRON_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&SECURITRON_SQUADRON];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
