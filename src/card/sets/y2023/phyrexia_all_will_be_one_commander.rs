//! Phyrexia: All Will Be One Commander cards cataloged for the Vintage Cube
//! pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, CardArt, CardRules, CardSet, CardType, CounterKind, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, PlayerRelation, TriggerEventDef, ValueDef,
    abilities,
};
use crate::mana_cost;

// ONC 6 — Glimmer Lens
pub(in crate::card::sets) static GLIMMER_LENS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("c9262000-e6f3-4da1-ad1c-038f65d3bef6"),
    "Glimmer Lens",
    CardArt::new(
        "c9262000-e6f3-4da1-ad1c-038f65d3bef6",
        "Sidharth Chaturvedi",
    ),
    CardSet::PhyrexiaAllWillBeOneCommander,
    CardRules::new_artifact(mana_cost!("{1}{W}"))
        .with_subtypes(&["Equipment"])
        .with_abilities(&[
            abilities::for_mirrodin(),
            // "And at least one other creature" is the whole declaration
            // being two or more: the Rebel it brought is one attacker, so
            // the card asks for a second body before it draws.
            AbilityDef::triggered(
                "Whenever equipped creature and at least one other creature attack, draw a card.",
                TriggerEventDef::attacks_in_declaration(
                    ObjectPredicateDef::AttachedToSource,
                    2,
                    None,
                ),
                EffectDef::DrawCards {
                    recipient: EffectRecipientDef::Controller,
                    amount: ValueDef::Constant(1),
                },
            ),
            abilities::equip(
                &[AbilityCostDef::Mana(mana_cost!("{1}{W}"))],
                "Equip {1}{W}",
            ),
        ]),
);

// ONC 39 — Otharri, Suns' Glory
// Audit: metadata-only — Card rules have not been implemented.
pub(in crate::card::sets) static OTHARRI_SUNS_GLORY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("348e0927-1d8f-4723-879d-f7e95ac60c27"),
    "Otharri, Suns' Glory",
    crate::card::CardArt::new("348e0927-1d8f-4723-879d-f7e95ac60c27", "Marta Nael"),
    crate::card::CardSet::PhyrexiaAllWillBeOneCommander,
    crate::card::CardRules::unsupported(),
);

// ONC 48 — Staff of the Storyteller
static STAFF_SPIRIT: [AbilityDef; 1] = [abilities::flying()];

/// The Staff pays for itself the moment it lands: the Spirit it makes is a
/// creature token you created, so its own trigger sees it.
static STAFF_MAKES_A_SPIRIT: EffectDef =
    EffectDef::create_creature_token(&["Spirit"], &[ManaColor::White], 1, 1)
        .with_abilities(&STAFF_SPIRIT);

/// One instruction, one counter, however many tokens it made -- which is
/// what "one or more" says and what makes a wide token maker no better here
/// than a narrow one.
static STAFF_COUNTS_THE_STORY: TriggerEventDef = TriggerEventDef::TokensCreated {
    player: PlayerRelation::You,
    token: ObjectPredicateDef::HasType(CardType::Creature),
};

static STAFF_DRAW_COST: [AbilityCostDef; 3] = [
    AbilityCostDef::Mana(mana_cost!("{W}")),
    AbilityCostDef::TapSource,
    AbilityCostDef::RemoveCountersFromSource {
        kind: CounterKind::Story,
        amount: 1,
    },
];

static STAFF_ABILITIES: [AbilityDef; 3] = [
    abilities::enters_trigger(
        "When this artifact enters, create a 1/1 white Spirit creature token with flying.",
        STAFF_MAKES_A_SPIRIT,
    ),
    AbilityDef::triggered(
        "Whenever you create one or more creature tokens, put a story counter on this artifact.",
        STAFF_COUNTS_THE_STORY,
        EffectDef::AddCounters {
            object: EffectRecipientDef::Source,
            kind: CounterKind::Story,
            amount: ValueDef::Constant(1),
        },
    ),
    AbilityDef::activated(
        "{W}, {T}, Remove a story counter from this artifact: Draw a card.",
        &STAFF_DRAW_COST,
        EffectDef::DrawCards {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
        },
    ),
];

pub(in crate::card::sets) static STAFF_OF_THE_STORYTELLER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("17be11f2-f2db-40c4-8fc1-2ed7173f9a1a"),
    "Staff of the Storyteller",
    CardArt::new("17be11f2-f2db-40c4-8fc1-2ed7173f9a1a", "Dan Murayama Scott"),
    CardSet::PhyrexiaAllWillBeOneCommander,
    // Two mana for a flier, and a card for every turn the deck keeps making
    // tokens afterwards.
    CardRules::new_artifact(mana_cost!("{1}{W}")).with_abilities(&STAFF_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &GLIMMER_LENS,
    &OTHARRI_SUNS_GLORY,
    &STAFF_OF_THE_STORYTELLER,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
