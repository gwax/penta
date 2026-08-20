//! Aether Revolt cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, TriggerConditionDef, cards,
};
use crate::ids::TargetIndex;
use crate::mana_cost;

static A_CREATURE: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one_permanent(
    ObjectPredicateDef::HasType(CardType::Creature),
)];

/// The mana value is read as the spell resolves rather than as it is cast,
/// so anything targetable is a legal target and a creature grown too
/// expensive in between simply survives.
static A_SMALL_CREATURE: TriggerConditionDef = TriggerConditionDef::TargetMatches {
    slot: TargetIndex::PRIMARY,
    object: ObjectPredicateDef::ManaValueAtMost(2),
};

static A_BIGGER_CREATURE: TriggerConditionDef = TriggerConditionDef::TargetMatches {
    slot: TargetIndex::PRIMARY,
    object: ObjectPredicateDef::ManaValueAtMost(4),
};

static REVOLT: TriggerConditionDef = TriggerConditionDef::ControllerHadPermanentLeaveThisTurn;

static WITHOUT_REVOLT: [TriggerConditionDef; 2] =
    [TriggerConditionDef::Not(&REVOLT), A_SMALL_CREATURE];

static WITH_REVOLT: [TriggerConditionDef; 2] = [REVOLT, A_BIGGER_CREATURE];

static PUSH_IT: EffectDef = EffectDef::Destroy {
    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    can_regenerate: true,
};

/// The revolt clause replaces the threshold rather than adding to it, so the
/// two branches are written as the exclusive pair the card prints and only
/// one of them can ever destroy anything.
static FATAL_PUSH_EFFECT: [EffectDef; 2] = [
    EffectDef::IfCondition {
        condition: &TriggerConditionDef::All(&WITHOUT_REVOLT),
        then: &PUSH_IT,
    },
    EffectDef::IfCondition {
        condition: &TriggerConditionDef::All(&WITH_REVOLT),
        then: &PUSH_IT,
    },
];

// AER 57 — Fatal Push
pub(in crate::card::sets) static FATAL_PUSH: CardRecord = CardRecord::new(
    cards::FATAL_PUSH,
    "Fatal Push",
    CardArt::new("b5e81649-9954-424c-89d1-f87d73b66047", "Eric Deschamps"),
    CardSet::AetherRevolt,
    // One black mana answers most of what a fast deck plays, and a fetchland
    // cracked on the way in stretches it over almost everything else.
    CardRules::new_instant(mana_cost!("{B}")).with_ability(AbilityDef::spell_with_targets(
        "Destroy target creature if it has mana value 2 or less.\nRevolt — Destroy that creature \
         if it has mana value 4 or less instead if a permanent left the battlefield under your \
         control this turn.",
        &A_CREATURE,
        EffectDef::Sequence(&FATAL_PUSH_EFFECT),
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&FATAL_PUSH];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
