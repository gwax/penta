//! War of the Spark cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef,
    AppliedRuleDef, CardArt, CardRules, CardSet, CardSupertype, ComparisonDef, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, ObjectQueryDef, PlayerRelation, TriggerConditionDef,
    ValueDef, ZoneKind, cards,
};
use crate::{TargetIndex, mana_cost};

/// Your own library, empty. Written as a count rather than a dedicated
/// question so the same shape answers "no cards in it" and any other bound.
static YOUR_LIBRARY_IS_EMPTY: TriggerConditionDef = TriggerConditionDef::ObjectCount {
    query: ObjectQueryDef::matching(
        ObjectPredicateDef::Any,
        &[ZoneKind::Library],
        PlayerRelation::You,
    ),
    comparison: ComparisonDef::LessOrEqual,
    amount: 0,
};

/// In a two-player game winning is the opponent losing, which is the shape
/// the engine has. The recorded reason is therefore an effect rather than a
/// dedicated win, which nothing in the supported pool reads.
static YOU_WIN: EffectDef = EffectDef::LoseTheGame {
    player: EffectRecipientDef::players(crate::card::PlayerSetDef::Related(
        PlayerRelation::Opponent,
    )),
};

static JACE_MILLS_AND_DRAWS: [EffectDef; 2] = [
    EffectDef::Mill {
        player: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        amount: ValueDef::Constant(2),
    },
    EffectDef::DrawCards {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::Constant(1),
    },
];

static JACE_DRAWS_SEVEN: [EffectDef; 2] = [
    EffectDef::DrawCards {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::Constant(7),
    },
    EffectDef::IfCondition {
        condition: &YOUR_LIBRARY_IS_EMPTY,
        then: &YOU_WIN,
    },
];

static JACE_ABILITIES: [AbilityDef; 3] = [
    // The static is the card: without it the seven-card draw and the mill
    // are just a slow Jace, and with it an empty library is a win rather
    // than the usual loss.
    AbilityDef::static_ability(
        "If you would draw a card while your library has no cards in it, you win the game instead.",
        EffectDef::StaticApply {
            recipient: EffectRecipientDef::players(crate::card::PlayerSetDef::Related(
                PlayerRelation::You,
            )),
            effect: AppliedEffectDef::Rule(AppliedRuleDef::WinsInsteadOfDrawingFromEmptyLibrary),
        },
    ),
    AbilityDef::activated_with_targets(
        "+1: Target player mills two cards. Draw a card.",
        &[AbilityCostDef::Loyalty(1)],
        &JACE_MILL_TARGET,
        EffectDef::Sequence(&JACE_MILLS_AND_DRAWS),
    ),
    AbilityDef::activated(
        "−8: Draw seven cards. Then if your library has no cards in it, you win the game.",
        &[AbilityCostDef::Loyalty(-8)],
        EffectDef::Sequence(&JACE_DRAWS_SEVEN),
    ),
];

static JACE_MILL_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Player(PlayerRelation::Any),
)];

// WAR 54 — Jace, Wielder of Mysteries
pub(in crate::card::sets) static JACE_WIELDER_OF_MYSTERIES: CardRecord = CardRecord::new(
    cards::JACE_WIELDER_OF_MYSTERIES,
    "Jace, Wielder of Mysteries",
    CardArt::new("6adb7d73-4482-4930-8497-cffd169b57e2", "Anna Steinbauer"),
    CardSet::WarOfTheSpark,
    CardRules::new_planeswalker(mana_cost!("{1}{U}{U}{U}"), &["Jace"], 4)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&JACE_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&JACE_WIELDER_OF_MYSTERIES];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
