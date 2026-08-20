//! Strixhaven: School of Mages cards cataloged for the Vintage Cube.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef, CardArt,
    CardRules, CardSet, CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef,
    TriggerConditionDef, ValueDef, ZoneKind, ZonePlacement, cards,
};
use crate::{TargetIndex, mana_cost};

static MASTERY_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Creature),
            ObjectPredicateDef::HasType(CardType::Planeswalker),
        ]),
        zones: &[ZoneKind::Battlefield],
        controller: None,
        owner: None,
    },
)];

/// The discount is the whole cost of the card: two mana instead of four,
/// and the opponent gets the card back. Which cast was used is read off the
/// spell itself, so the rider is part of one resolution rather than a
/// second clause.
static MASTERY_WAS_DISCOUNTED: TriggerConditionDef =
    TriggerConditionDef::SourceCastWith(AlternativeCastKindDef::AlternativeCost);

static MASTERY_OPPONENT_DRAWS: EffectDef = EffectDef::DrawCards {
    recipient: EffectRecipientDef::Opponent,
    amount: ValueDef::Constant(1),
};

static MASTERY_EXILE: EffectDef = EffectDef::MoveToZone {
    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    zone: ZoneKind::Exile,
    placement: ZonePlacement::Top,
    controller: None,
    arrival_effect: None,
    attach_source: false,
};

/// Printed order: the draw is named before the exile, and it happens first.
static MASTERY_RESOLUTION: [EffectDef; 2] = [
    EffectDef::IfCondition {
        condition: &MASTERY_WAS_DISCOUNTED,
        then: &MASTERY_OPPONENT_DRAWS,
    },
    MASTERY_EXILE,
];

// STX 64 — Baleful Mastery
pub(in crate::card::sets) static BALEFUL_MASTERY: CardRecord = CardRecord::new(
    cards::BALEFUL_MASTERY,
    "Baleful Mastery",
    CardArt::new("35f1a6ba-e46f-44fb-93f4-fb883d677b36", "Chris Cold"),
    CardSet::StrixhavenSchoolOfMages,
    // Exile at instant speed answers anything, and the choice of price is
    // the card: four mana clean, or two and a card for them.
    CardRules::new_instant(mana_cost!("{3}{B}")).with_abilities(&[
        AbilityDef::spell_with_targets(
            "If the {1}{B} cost was paid, an opponent draws a card.\nExile target creature or planeswalker.",
            &MASTERY_TARGET,
            EffectDef::Sequence(&MASTERY_RESOLUTION),
        ),
        AbilityDef::alternative_cast(
            mana_cost!("{1}{B}"),
            AlternativeCastKindDef::AlternativeCost,
            Some("You may pay {1}{B} rather than pay this spell's mana cost."),
            EffectDef::None,
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&BALEFUL_MASTERY];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
