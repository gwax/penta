//! Zendikar Rising cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, abilities, cards,
};
use crate::{TargetIndex, mana_cost};

static CREATURE_OR_PLANESWALKER: ObjectPredicateDef = ObjectPredicateDef::AnyOf(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::HasType(CardType::Planeswalker),
]);

/// The mana-value bound is part of what may be targeted rather than something
/// checked on resolution, so an unkicked Thirst never points at anything
/// bigger in the first place.
static THIRST_SMALL_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one_permanent(
    ObjectPredicateDef::All(&[
        CREATURE_OR_PLANESWALKER,
        ObjectPredicateDef::ManaValueAtMost(2),
    ]),
)];

static THIRST_ANY_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one_permanent(
    CREATURE_OR_PLANESWALKER,
)];

static THIRST_DESTROY: EffectDef = EffectDef::Destroy {
    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    can_regenerate: true,
};

// ZNR 85 — Thieving Skydiver
// Audit: blocked — Kicker here is a spell cast for more mana with different instructions, and the kicked clause has to carry those instructions. This card's kicker changes nothing about how the spell resolves; it changes whether a triggered ability fires afterwards and what that ability may target, which the kicked alternative has no way to say. It also needs a minimum on X, since casts are enumerated from zero and "X can't be 0" would otherwise let an unkicked-sized cast steal a nothing-cost artifact.

// ZNR 94 — Bloodchief's Thirst
pub(in crate::card::sets) static BLOODCHIEFS_THIRST: CardRecord = CardRecord::new(
    cards::BLOODCHIEFS_THIRST,
    "Bloodchief's Thirst",
    CardArt::new("059e8447-6b1c-4651-a734-a8fea2cbf7b2", "Jason Rainville"),
    CardSet::ZendikarRising,
    // One black kills most of what an aggressive deck leads with; four kills
    // whatever is left, which is why the card is played over a cheaper
    // removal spell that can only do the first job.
    CardRules::new_sorcery(mana_cost!("{B}")).with_abilities(&[
        AbilityDef::spell_with_targets(
            "Kicker {2}{B} (You may pay an additional {2}{B} as you cast this spell.)\nDestroy target creature or planeswalker with mana value 2 or less.",
            &THIRST_SMALL_TARGET,
            THIRST_DESTROY,
        ),
        abilities::kicker(
            mana_cost!("{3}{B}"),
            "Destroy target creature or planeswalker.",
            &THIRST_ANY_TARGET,
            THIRST_DESTROY,
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&BLOODCHIEFS_THIRST];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
