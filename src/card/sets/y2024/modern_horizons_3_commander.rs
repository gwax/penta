//! Modern Horizons 3 Commander cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCoverageDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef,
    CardArt, CardRules, CardSet, CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef,
    PlayerRelation, TriggerEventDef, ValueDef, ZoneKind, cards,
};
use crate::{TargetIndex, mana_cost};

/// A Lhurgoyf you control -- this one included, which is what "this creature
/// or another" comes to.
static A_LHURGOYF_YOU_CONTROL: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::Subtype("Lhurgoyf"),
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
]);

static PYROGOYF_DAMAGE_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::AnyTarget,
)];

static PYROGOYF_ABILITIES: [AbilityDef; 2] = [
    AbilityDef::static_ability(
        "Pyrogoyf's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1.",
        EffectDef::StaticApply {
            recipient: EffectRecipientDef::Source,
            // The printed toughness carries the "plus 1", so the counted part
            // is the same number on both sides.
            effect: AppliedEffectDef::modify_power_toughness(
                ValueDef::CardTypesAmongGraveyards,
                ValueDef::CardTypesAmongGraveyards,
            ),
        },
    )
    .with_coverage(AbilityCoverageDef::partial(
        "A characteristic-defining ability sets power and toughness in every zone. This is a \
         battlefield-only continuous effect, so the value is right wherever the card is played \
         and absent for anything reading it in another zone.",
    )),
    AbilityDef::triggered_with_targets(
        "Whenever this creature or another Lhurgoyf creature you control enters, that creature deals damage equal to its power to any target.",
        TriggerEventDef::zone_changed(A_LHURGOYF_YOU_CONTROL, None, Some(ZoneKind::Battlefield)),
        &PYROGOYF_DAMAGE_TARGET,
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::TriggeringObjectPower,
        },
    )
    .with_coverage(AbilityCoverageDef::partial(
        "The damage is dealt by the Lhurgoyf that entered. Its amount is read from that \
         creature, but the source recorded for the damage is Pyrogoyf, so protection from red \
         and redirection answer the wrong object when some other Lhurgoyf is the one entering. \
         No other Lhurgoyf is cataloged yet.",
    )),
];

// M3C 59 — Pyrogoyf
pub(in crate::card::sets) static PYROGOYF: CardRecord = CardRecord::new(
    cards::PYROGOYF,
    "Pyrogoyf",
    CardArt::new("f60be310-4461-4b84-95f0-b2095108bd79", "Xabi Gaztelua"),
    CardSet::ModernHorizons3Commander,
    // The printed body is 0/1: the counted part supplies the rest, and the
    // "plus 1" is the toughness this starts from.
    CardRules::new_creature(mana_cost!("{3}{R}"), &["Lhurgoyf"], 0, 1)
        .with_abilities(&PYROGOYF_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&PYROGOYF];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
