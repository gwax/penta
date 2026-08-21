//! Commander Legends: Battle for Baldur's Gate cards cataloged for the
//! Vintage Cube.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCoverageDef, AbilityDef, CardArt, CardRules, CardSet, CardSupertype, CardType,
    EffectDef, EffectRecipientDef, ObjectPredicateDef, PlayerRelation, SacrificedAmountDef,
    TriggerEventDef, ValueDef, cards,
};
use crate::mana_cost;

/// "Another creature or an artifact." Gut is neither an artifact nor another
/// creature, so the exclusion covers both halves without saying so twice.
static ANOTHER_CREATURE_OR_AN_ARTIFACT: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::AnyOf(&[
        ObjectPredicateDef::HasType(CardType::Creature),
        ObjectPredicateDef::HasType(CardType::Artifact),
    ]),
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
    ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
]);

/// The token arrives already attacking, which is the whole point: it was
/// never declared, so nothing that watches a declaration sees it, and it
/// still connects this combat.
static GUT_MAKES_A_SKELETON: EffectDef = EffectDef::CreateToken {
    token: cards::SKELETON_TOKEN_4_1_BLACK,
    count: ValueDef::Constant(1),
    tapped: true,
    attacking: true,
    counters: None,
    created: None,
};

/// "Whenever you attack" is one or more creatures you control attacking,
/// counted once for the declaration rather than once per attacker.
static WHENEVER_YOU_ATTACK: TriggerEventDef = TriggerEventDef::attack_declared(
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
    1,
    None,
);

// CLB 180 — Gut, True Soul Zealot
pub(in crate::card::sets) static GUT_TRUE_SOUL_ZEALOT: CardRecord = CardRecord::new(
    cards::GUT_TRUE_SOUL_ZEALOT,
    "Gut, True Soul Zealot",
    CardArt::new("3d8ca18d-9099-4f1e-95c1-f04da58a26bd", "Wayne Reynolds"),
    CardSet::CommanderLegendsBattleForBaldursGate,
    // Every spent artifact and every creature that has done its work turns
    // into four attacking power that two blockers cannot answer alone.
    CardRules::new_creature(mana_cost!("{2}{R}"), &["Goblin", "Shaman"], 2, 2)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&[
            AbilityDef::triggered(
                "Whenever you attack, you may sacrifice another creature or an artifact. If you do, create a 4/1 black Skeleton creature token with menace that's tapped and attacking.",
                WHENEVER_YOU_ATTACK,
                EffectDef::SacrificeOfChoice {
                    player: EffectRecipientDef::Controller,
                    object: ANOTHER_CREATURE_OR_AN_ARTIFACT,
                    then: Some(&GUT_MAKES_A_SKELETON),
                    amount: SacrificedAmountDef::Power,
                    otherwise: None,
                    optional: true,
                },
            ),
            AbilityDef::static_ability(
                "Choose a Background (You can have a Background as a second commander.)",
                EffectDef::Special("Choose a Background"),
            )
            .with_coverage(AbilityCoverageDef::metadata_only(
                "Backgrounds are a Commander deck-construction rule. This engine plays no \
                 format that has a command zone, so the clause names nothing a game can do.",
            )),
        ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&GUT_TRUE_SOUL_ZEALOT];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
