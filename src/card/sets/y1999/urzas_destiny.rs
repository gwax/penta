//! Urza's Destiny cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, CardArt, CardRules, CardSet, CardType, CounterKind, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, PlayerRelation, TriggerEventDef, TurnStepDef, ValueDef,
    ZoneKind, ZonePlacement, cards,
};
use crate::mana_cost;

/// Every enchantment card the graveyard holds, all at once. The printed
/// reminder about Auras is the ordinary rule for an Aura arriving with
/// nothing to enchant, not a clause of its own.
static ENCHANTMENTS_IN_YOUR_GRAVEYARD: EffectRecipientDef = EffectRecipientDef::matching_objects(
    ObjectPredicateDef::HasType(CardType::Enchantment),
    &[ZoneKind::Graveyard],
    PlayerRelation::You,
);

// UDS 15 — Replenish
pub(in crate::card::sets) static REPLENISH: CardRecord = CardRecord::new(
    cards::REPLENISH,
    "Replenish",
    CardArt::new("c922d401-7916-42d3-9185-9de6219f9c38", "Jim Nelson"),
    CardSet::UrzasDestiny,
    // The deck is built to fill its own graveyard first, so this is not
    // recursion so much as the whole board arriving on one turn.
    CardRules::new_sorcery(mana_cost!("{3}{W}")).with_ability(AbilityDef::spell(
        "Return all enchantment cards from your graveyard to the battlefield.",
        EffectDef::MoveToZone {
            object: ENCHANTMENTS_IN_YOUR_GRAVEYARD,
            zone: ZoneKind::Battlefield,
            placement: ZonePlacement::Top,
            arrival_effect: None,
            controller: None,
        },
    )),
);

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

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&REPLENISH, &POWDER_KEG];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
