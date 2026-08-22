//! March of the Machine cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, CardArt, CardRules, CardSet, CardType, CounterKind,
    DrawEventMatcherDef, EffectDef, EffectRecipientDef, ObjectPredicateDef, ObjectQueryDef,
    ObjectSetDef, PlayerRelation, PlayerSetDef, TokenCountersDef, TriggerEventDef, ValueDef,
    ZoneKind, ZonePlacement, abilities, tokens,
};
use crate::ids::ObjectSetBindingIndex;
use crate::mana_cost;

/// Everyone's, which is what "all creatures" means.
static EVERY_CREATURE: ObjectQueryDef = ObjectQueryDef::new(
    ObjectPredicateDef::HasType(CardType::Creature),
    &[ZoneKind::Battlefield],
);

/// The creatures are bound before they move, because "X, where X is the
/// number of creatures exiled this way" asks about a set the board no longer
/// holds by the time the token is made.
static SUNFALL_STEPS: [EffectDef; 2] = [
    EffectDef::MoveToZone {
        counters: None,
        object: EffectRecipientDef::objects(ObjectSetDef::Binding(ObjectSetBindingIndex::PRIMARY)),
        zone: ZoneKind::Exile,
        placement: ZonePlacement::Top,
        controller: None,
        arrival_effect: None,
        attachment: None,
    },
    // Incubate X. One token however large X is, and X of zero still makes
    // one: the keyword creates the token unconditionally.
    EffectDef::create_token(tokens::incubator())
        .with_art(CardArt::new(
            "2c5ed737-657b-43bf-b222-941da7579a4a",
            "Johann Bodin",
        ))
        .with_counters(TokenCountersDef {
            kind: CounterKind::PlusOnePlusOne,
            amount: ValueDef::BoundObjectCount(ObjectSetBindingIndex::PRIMARY),
        }),
];

static SUNFALL_EXILES_THEN_INCUBATES: EffectDef = EffectDef::BindMatching {
    objects: ObjectSetDef::Query(EVERY_CREATURE),
    binding: ObjectSetBindingIndex::PRIMARY,
    then: &EffectDef::Sequence(&SUNFALL_STEPS),
};

// MOM 40 — Sunfall
pub(in crate::card::sets) static SUNFALL: CardRecord = CardRecord::new_with_legacy_id(
    2258,
    "Sunfall",
    CardArt::new(
        "32e29c7d-ed4b-4eff-b3c2-d99e5b63ef8d",
        "Kasia 'Kafis' Zielińska",
    ),
    CardSet::MarchOfTheMachine,
    // A wrath that exiles rather than destroys, and hands the caster the
    // biggest thing on the empty board it just made.
    CardRules::new_sorcery(mana_cost!("{3}{W}{W}")).with_ability(AbilityDef::spell(
        "Exile all creatures. Incubate X, where X is the number of creatures exiled this way. \
         (Create an Incubator token with X +1/+1 counters on it and \"{2}: Transform this \
         token.\" It transforms into a 0/0 Phyrexian artifact creature.)",
        SUNFALL_EXILES_THEN_INCUBATES,
    )),
);

static FAERIE_MASTERMIND_ABILITIES: [AbilityDef; 4] = [
    abilities::flash(),
    abilities::flying(),
    // The ordinal is the whole clause: their first card each turn is the one
    // the rules hand them, so this catches the extra one and nothing else.
    AbilityDef::triggered(
        "Whenever an opponent draws their second card each turn, you draw a card.",
        TriggerEventDef::DrewCard(DrawEventMatcherDef::nth_each_turn(
            PlayerRelation::Opponent,
            2,
        )),
        EffectDef::DrawCards {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
        },
    ),
    // Symmetrical on purpose: with the trigger above out, the copy they draw
    // is the one that draws you another.
    AbilityDef::activated(
        "{3}{U}: Each player draws a card.",
        &[AbilityCostDef::Mana(mana_cost!("{3}{U}"))],
        EffectDef::DrawCards {
            recipient: EffectRecipientDef::players(PlayerSetDef::All),
            amount: ValueDef::Constant(1),
        },
    ),
];

// MOM 58 — Faerie Mastermind
pub(in crate::card::sets) static FAERIE_MASTERMIND: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("52d3005f-a1c7-4ef5-911f-ccc0752f4181"),
    "Faerie Mastermind",
    CardArt::new("52d3005f-a1c7-4ef5-911f-ccc0752f4181", "Joshua Raphael"),
    CardSet::MarchOfTheMachine,
    // A two-mana flash flier that is never a dead card: it taxes every
    // cantrip the other deck was going to cast anyway, and turns into a
    // draw engine once there is nothing else to spend mana on.
    CardRules::new_creature(mana_cost!("{1}{U}"), &["Faerie", "Rogue"], 2, 1)
        .with_abilities(&FAERIE_MASTERMIND_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&SUNFALL, &FAERIE_MASTERMIND];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
