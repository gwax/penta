//! Duskmourn: House of Horror cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, CardArt, CardRules, CardSet, CardType, CardTypeSet, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, PlayerRelation, TriggerConditionDef, TriggerEventDef,
    ValueDef, ZoneKind, ZonePlacement, abilities, cards,
};
use crate::mana_cost;

/// "Other creatures you control with power 2 or less", read as each one
/// enters. The cap below is what makes a batch of them draw one card.
static A_SMALL_CREATURE_YOU_CONTROL: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
    ObjectPredicateDef::PowerLessThan(ValueDef::Constant(3)),
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
]);

/// What it comes back as. Setting the type line rather than adding to it is
/// what takes the creature away, and the effect lasts as long as the
/// permanent does -- so the next time it dies the clause below finds an
/// enchantment and leaves it in the graveyard.
static ENDURES_AS_AN_ENCHANTMENT: AppliedEffectDef =
    AppliedEffectDef::set_card_types(CardTypeSet::single(CardType::Enchantment));

static IT_WAS_A_CREATURE: TriggerConditionDef = TriggerConditionDef::SourceMatches {
    object: ObjectPredicateDef::HasType(CardType::Creature),
};

static INNOCENCE_RETURNS: EffectDef = EffectDef::MoveToZone {
    object: EffectRecipientDef::Source,
    zone: ZoneKind::Battlefield,
    placement: ZonePlacement::Top,
    controller: None,
    arrival_effect: Some(&ENDURES_AS_AN_ENCHANTMENT),
    attachment: None,
};

static ENDURING_INNOCENCE_ABILITIES: [AbilityDef; 3] = [
    abilities::lifelink(),
    AbilityDef::triggered(
        "Whenever one or more other creatures you control with power 2 or less enter, draw a \
         card. This ability triggers only once each turn.",
        TriggerEventDef::zone_changed(
            A_SMALL_CREATURE_YOU_CONTROL,
            None,
            Some(ZoneKind::Battlefield),
        ),
        EffectDef::DrawCards {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
        },
    )
    .triggering_at_most(1),
    AbilityDef::triggered_if(
        "When this creature dies, if it was a creature, return it to the battlefield under its \
         owner's control. It's an enchantment. (It's not a creature.)",
        TriggerEventDef::zone_changed(
            ObjectPredicateDef::Source,
            Some(ZoneKind::Battlefield),
            Some(ZoneKind::Graveyard),
        ),
        &IT_WAS_A_CREATURE,
        INNOCENCE_RETURNS,
    ),
];

// DSK 6 — Enduring Innocence
pub(in crate::card::sets) static ENDURING_INNOCENCE: CardRecord = CardRecord::new(
    cards::ENDURING_INNOCENCE,
    "Enduring Innocence",
    CardArt::new("6d908299-aac0-46a6-8fa5-780d5b3e0386", "Liiga Smilshkalne"),
    CardSet::DuskmournHouseOfHorror,
    // Answering it costs two cards: one to kill the creature and one for the
    // enchantment that gets up afterwards and keeps drawing.
    CardRules::new_enchantment_creature(mana_cost!("{1}{W}{W}"), &["Sheep", "Glimmer"], 2, 1)
        .with_abilities(&ENDURING_INNOCENCE_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&ENDURING_INNOCENCE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
