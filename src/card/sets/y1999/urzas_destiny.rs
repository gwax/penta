//! Urza's Destiny cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, CardArt, CardRules, CardSet, CardType, CounterKind, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, PlayerRelation, TriggerEventDef, TurnStepDef, ValueDef,
    ZoneKind, cards,
};
use crate::mana_cost;

/// Everything the fuse counters name. A Keg with no counters on it destroys
/// every nothing-cost permanent, which is the mode that answers a board of
/// tokens.
static MATCHING_ARTIFACTS_AND_CREATURES: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::AnyOf(&[
        ObjectPredicateDef::HasType(CardType::Artifact),
        ObjectPredicateDef::HasType(CardType::Creature),
    ]),
    ObjectPredicateDef::ManaValueEqualTo(ValueDef::CountersOnSource(CounterKind::Fuse)),
]);

static KEG_DETONATION: EffectDef = EffectDef::Destroy {
    object: EffectRecipientDef::matching_objects(
        MATCHING_ARTIFACTS_AND_CREATURES,
        &[ZoneKind::Battlefield],
        PlayerRelation::Any,
    ),
    can_regenerate: true,
};

/// The counter is optional, so the Keg can be held at whatever size the board
/// calls for rather than ticking past it.
static KEG_FUSE: EffectDef = EffectDef::May {
    player: EffectRecipientDef::Controller,
    effect: &EffectDef::AddCounters {
        object: EffectRecipientDef::Source,
        kind: CounterKind::Fuse,
        amount: ValueDef::Constant(1),
    },
};

// UDS 136 — Powder Keg
pub(in crate::card::sets) static POWDER_KEG: CardRecord = CardRecord::new(
    cards::POWDER_KEG,
    "Powder Keg",
    CardArt::new("4d9715c2-9036-4ae2-a5b4-1b190d50c963", "Dan Frazier"),
    CardSet::UrzasDestiny,
    CardRules::new_artifact(mana_cost!("{2}")).with_abilities(&[
        AbilityDef::triggered(
            "At the beginning of your upkeep, you may put a fuse counter on this artifact.",
            TriggerEventDef::StepBegins {
                step: TurnStepDef::Upkeep,
                player: PlayerRelation::You,
            },
            KEG_FUSE,
        ),
        AbilityDef::activated(
            "{T}, Sacrifice this artifact: Destroy each artifact and creature with mana value equal to the number of fuse counters on this artifact.",
            &[AbilityCostDef::TapSource, AbilityCostDef::SacrificeSource],
            KEG_DETONATION,
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&POWDER_KEG];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
