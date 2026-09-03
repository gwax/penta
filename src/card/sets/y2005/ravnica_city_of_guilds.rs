//! Ravnica: City of Guilds cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::AbilityCostDef;
use crate::BasicLandType;
use crate::CardType;
use crate::card::abilities;
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AggregateOperationDef, CardArt,
    CardRules, CardSet, EffectDef, EffectRecipientDef, MoveObjectsDef, ObjectPredicateDef,
    ObjectSetDef, ObjectValueAggregateDef, ObjectValueDef, PlayerRefDef, PlayerRelation,
    RevealObjectsDef, TriggerEventDef, TurnStepDef, ValueDef, ZoneKind, ZonePlacement,
};
use crate::{ParentBinding, TargetIndex, mana_cost};

// RAV 16 — Faith's Fetters
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static FAITH_S_FETTERS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("5b8ffba3-44a9-41ce-a5a1-37413346db2f"),
    "Faith's Fetters",
    crate::card::CardArt::new("fe653236-c5c1-4dcd-95cd-3c53f1e256ef", "Brian Despain"),
    crate::card::CardSet::RavnicaCityOfGuilds,
    crate::card::CardRules::unsupported(),
);

// RAV 38 — Belltower Sphinx
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static BELLTOWER_SPHINX: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("452a23a0-62de-4561-b361-9c0de9151129"),
    "Belltower Sphinx",
    crate::card::CardArt::new("d6829959-dae1-4ddf-8a75-33a77e6b4612", "Jim Nelson"),
    crate::card::CardSet::RavnicaCityOfGuilds,
    crate::card::CardRules::unsupported(),
);

// RAV 61 — Peel from Reality
pub(in crate::card::sets) static PEEL_FROM_REALITY: CardRecord = CardRecord::new_with_legacy_id(
    780,
    "Peel from Reality",
    CardArt::new("7f41285b-5961-4653-96a0-fb6d27111390", "Jason Felix"),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_instant(mana_cost!("{1}{U}")).with_ability(
        AbilityDef::spell_with_targets(
            "Return target creature you control and target creature you don't control to their owners' hands.",
            &[
                AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
                    object: ObjectPredicateDef::HasType(CardType::Creature),
                    zones: &[ZoneKind::Battlefield],
                    controller: Some(PlayerRelation::You),
                    owner: None,
                }),
                AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
                    object: ObjectPredicateDef::HasType(CardType::Creature),
                    zones: &[ZoneKind::Battlefield],
                    controller: Some(PlayerRelation::Opponent),
                    owner: None,
                }),
            ],
            EffectDef::Sequence(&[
                EffectDef::MoveToZone {
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    zone: ZoneKind::Hand,
                    placement: ZonePlacement::Top,
},
                EffectDef::MoveToZone {
                    object: EffectRecipientDef::Target(TargetIndex(1)),
                    zone: ZoneKind::Hand,
                    placement: ZonePlacement::Top,
},
            ]),
        ),
    ),
);

// RAV 63 — Remand
pub(in crate::card::sets) static REMAND: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("581f3780-c480-48c6-b15c-1618f2feccb9"),
    "Remand",
    CardArt::new("36de9999-8d0a-4174-8e38-549bacdc128b", "Mark A. Nelson"),
    CardSet::RavnicaCityOfGuilds,
    // Two mana to buy a turn and replace itself. What it answers comes back,
    // so this is tempo rather than an answer.
    CardRules::new_instant(mana_cost!("{1}{U}")).with_ability(AbilityDef::spell_with_targets(
        "Counter target spell. If that spell is countered this way, put it into its owner's hand \
         instead of into that player's graveyard.\nDraw a card.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::Spell,
                zones: &[ZoneKind::Stack],
                controller: None,
                owner: None,
            },
        )],
        // The countered card goes to its owner's hand rather than their graveyard,
        // which the counter effect's own destination says. The draw is a second
        // clause and happens whether or not the counter found anything to do.
        EffectDef::Sequence(&[
            EffectDef::Counter {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                zone: ZoneKind::Hand,
                placement: ZonePlacement::Top,
            },
            EffectDef::DrawCards {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(1),
            },
        ]),
    )),
);

// RAV 74 — Vedalken Entrancer
pub(in crate::card::sets) static VEDALKEN_ENTRANCER: CardRecord = CardRecord::new_with_legacy_id(
    994,
    "Vedalken Entrancer",
    CardArt::new("dc4bbd25-5ddd-4502-b582-b7d89c9f97a5", "Dan Murayama Scott"),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_creature(mana_cost!("{3}{U}"), &["Vedalken", "Wizard"], 1, 4).with_ability(
        AbilityDef::activated_with_targets(
            "{U}, {T}: Target player mills two cards.",
            &[
                AbilityCostDef::Mana(mana_cost!("{U}")),
                AbilityCostDef::TapSource,
            ],
            &[AbilityTargetDef::exactly_one(
                AbilityTargetPredicate::Player(PlayerRelation::Any),
            )],
            EffectDef::Mill {
                player: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(2),
            },
        ),
    ),
);

// RAV 81 — Dark Confidant
/// One card off the top, shown to everybody, into your hand. Nothing is
/// chosen and nothing may be declined: the minimum and the maximum are both
/// the one card the trigger names.
pub(in crate::card::sets) static DARK_CONFIDANT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("94f7a441-bf2d-46fb-a7b6-9bd6137f86d9"),
    "Dark Confidant",
    CardArt::new("94f7a441-bf2d-46fb-a7b6-9bd6137f86d9", "Ron Spears"),
    CardSet::RavnicaCityOfGuilds,
    // Two mana for an extra card every turn, at whatever the top of your
    // deck happens to cost -- which is why the decks that play him keep
    // their curve low enough to survive him.
    CardRules::new_creature(mana_cost!("{1}{B}"), &["Human", "Wizard"], 2, 1).with_ability(
        AbilityDef::triggered(
            "At the beginning of your upkeep, reveal the top card of your library and put that \
             card into your hand. You lose life equal to its mana value.",
            TriggerEventDef::StepBegins {
                step: TurnStepDef::Upkeep,
                player: PlayerRelation::You,
            },
            abilities::bind_top_cards_then(
                PlayerRefDef::EffectController,
                ValueDef::Constant(1),
                &const {
                    EffectDef::Sequence(&[
                        EffectDef::RevealObjects(RevealObjectsDef {
                            input: ObjectSetDef::Binding(ParentBinding),
                            then: &EffectDef::None,
                        }),
                        EffectDef::MoveObjects(MoveObjectsDef {
                            input: ObjectSetDef::Binding(ParentBinding),
                            from: Some(ZoneKind::Library),
                            zone: ZoneKind::Hand,
                            placement: ZonePlacement::Top,
                            moved: Some(ParentBinding),
                            // "You lose life equal to its mana value." The card is in your hand by the
                            // time this is asked, so what the reveal hands on is the number rather than
                            // the card.
                            then: &EffectDef::LoseLife {
                                recipient: EffectRecipientDef::Controller,
                                amount: ValueDef::AggregateObjectValues(&ObjectValueAggregateDef {
                                    objects: ObjectSetDef::Binding(ParentBinding),
                                    select: ObjectValueDef::ManaValue,
                                    operation: AggregateOperationDef::Maximum,
                                }),
                            },
                        }),
                    ])
                },
            ),
        ),
    ),
);

// RAV 125 — Frenzied Goblin
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static FRENZIED_GOBLIN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("d307d8c7-b9b5-4f8f-933d-f1c64cbbf92f"),
    "Frenzied Goblin",
    crate::card::CardArt::new("7ddfe382-3a80-45f3-a022-54739c4b69a6", "Carl Critchlow"),
    crate::card::CardSet::RavnicaCityOfGuilds,
    crate::card::CardRules::unsupported(),
);

// RAV 139 — Reroute
pub(in crate::card::sets) static REROUTE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("42794e10-ddcd-4d2d-ab0c-a6b99b6d4662"),
    "Reroute",
    CardArt::new(
        "42794e10-ddcd-4d2d-ab0c-a6b99b6d4662",
        "Christopher Rush",
    ),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_instant(mana_cost!("{1}{R}")).with_ability(
        AbilityDef::spell_with_targets(
            "Change the target of target activated ability with a single target. (Mana abilities can't be targeted.)\nDraw a card.",
            &[AbilityTargetDef::exactly_one(
                AbilityTargetPredicate::Object {
                    object: ObjectPredicateDef::All(&[
                        ObjectPredicateDef::ActivatedAbility,
                        ObjectPredicateDef::DeclaredTargetCount {
                            minimum: 1,
                            maximum: 1,
                        },
                    ]),
                    zones: &[ZoneKind::Stack],
                    controller: None,
                    owner: None,
                },
            )],
            EffectDef::Sequence(&[
                EffectDef::ChangeStackTargets(&crate::card::ChangeStackTargetsDef {
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    chooser: PlayerRefDef::EffectController,
                    change: crate::card::StackTargetChangeDef::ChooseNew {
                        optional: false,
                        restriction: None,
                    },
                }),
                EffectDef::DrawCards {
                    recipient: EffectRecipientDef::Controller,
                    amount: ValueDef::Constant(1),
                },
            ]),
        ),
    ),
);

// RAV 163 — Farseek
pub(in crate::card::sets) static FARSEEK: CardRecord = CardRecord::new_with_legacy_id(
    1697,
    "Farseek",
    CardArt::new("f9b69d33-96dd-4844-aefa-27a885cb2ffc", "Martina Pilcerova"),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_sorcery(mana_cost!("{1}{G}")).with_ability(AbilityDef::spell(
        "Search your library for a Plains, Island, Swamp, or Mountain card, put it onto the battlefield tapped, then shuffle.",
        EffectDef::SearchZone {
            player: EffectRecipientDef::Controller,
            source: ZoneKind::Library,
            object: ObjectPredicateDef::HasAnyBasicLandType(&[
                BasicLandType::Plains,
                BasicLandType::Island,
                BasicLandType::Swamp,
                BasicLandType::Mountain,
            ]),
            minimum: 0,
            maximum: ValueDef::Constant(1),
            reveal: false,
            destination: ZoneKind::Battlefield,
            placement: ZonePlacement::Top,
            shuffle: true,
            enters_tapped: true,
            attachment: None,
            binding: None,
            then: None,
        },
    )),
);

// RAV 221 — Putrefy
pub(in crate::card::sets) static PUTREFY: CardRecord = CardRecord::new_with_legacy_id(
    198,
    "Putrefy",
    CardArt::new("0d43a0b6-2a5c-4959-96ee-6e570949dfed", "Igor Kieryluk"),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_instant(mana_cost!("{1}{B}{G}")).with_ability(AbilityDef::destroy_target(
        "Destroy target artifact or creature. It can't be regenerated.",
        &AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Artifact),
            ObjectPredicateDef::HasType(CardType::Creature),
        ])),
        false,
    )),
);

// RAV 232 — Skyknight Legionnaire
pub(in crate::card::sets) static SKYKNIGHT_LEGIONNAIRE: CardRecord = CardRecord::new_with_legacy_id(
    1120,
    "Skyknight Legionnaire",
    CardArt::new("ae8c9948-b52e-4d07-a72a-99ab6be05cc6", "Anthony Palumbo"),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_creature(mana_cost!("{1}{R}{W}"), &["Human", "Knight"], 2, 2)
        .with_abilities(&[abilities::flying(), abilities::haste()]),
);

// RAV 245 — Dimir Guildmage
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static DIMIR_GUILDMAGE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("b9ab53af-749e-4559-85fa-f8d4181cf7da"),
    "Dimir Guildmage",
    crate::card::CardArt::new("0b963389-6231-4095-a1f4-33457ce51ff2", "Adam Rex"),
    crate::card::CardSet::RavnicaCityOfGuilds,
    crate::card::CardRules::unsupported(),
);

// RAV 275 — Boros Garrison
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static BOROS_GARRISON: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("7dfe3f03-078f-44fb-89cd-efa3ebfaf637"),
    "Boros Garrison",
    crate::card::CardArt::new("c468dd1c-6f0a-4679-9d33-17e17db8841d", "John Avon"),
    crate::card::CardSet::RavnicaCityOfGuilds,
    crate::card::CardRules::unsupported(),
);

// RAV 276 — Dimir Aqueduct
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static DIMIR_AQUEDUCT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("df3c3d56-8291-407e-87a1-94b7d12811fd"),
    "Dimir Aqueduct",
    crate::card::CardArt::new("84bf9d60-64b8-4209-acfe-e07eefc6bf1f", "John Avon"),
    crate::card::CardSet::RavnicaCityOfGuilds,
    crate::card::CardRules::unsupported(),
);

// RAV 278 — Golgari Rot Farm
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static GOLGARI_ROT_FARM: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("104364d5-ede8-4ac5-900f-19947f51bbc1"),
    "Golgari Rot Farm",
    crate::card::CardArt::new("725fab98-558b-4b0c-a0a4-ef0eec92eebb", "John Avon"),
    crate::card::CardSet::RavnicaCityOfGuilds,
    crate::card::CardRules::unsupported(),
);

// RAV 279 — Overgrown Tomb
pub(in crate::card::sets) static OVERGROWN_TOMB: CardRecord = CardRecord::new_with_legacy_id(
    194,
    "Overgrown Tomb",
    CardArt::new("1c7d50d6-b63a-4d8c-88fa-1d78ae693a45", "Steven Belledin"),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_land(&["Swamp", "Forest"]).with_ability(abilities::shock_land_enters()),
);

// RAV 280 — Sacred Foundry
pub(in crate::card::sets) static SACRED_FOUNDRY: CardRecord = CardRecord::new_with_legacy_id(
    207,
    "Sacred Foundry",
    CardArt::new("0a26d900-c652-4f9c-8681-a35c5f8b1937", "Sam Burley"),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_land(&["Mountain", "Plains"]).with_ability(abilities::shock_land_enters()),
);

// RAV 281 — Selesnya Sanctuary
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SELESNYA_SANCTUARY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("c5e51787-f9c9-4926-9df1-a384a3092676"),
    "Selesnya Sanctuary",
    crate::card::CardArt::new("fdc53c6a-8e28-4314-9bcf-b31b6c6f56d7", "John Avon"),
    crate::card::CardSet::RavnicaCityOfGuilds,
    crate::card::CardRules::unsupported(),
);

// RAV 284 — Temple Garden
pub(in crate::card::sets) static TEMPLE_GARDEN: CardRecord = CardRecord::new_with_legacy_id(
    224,
    "Temple Garden",
    CardArt::new("b821e604-f9fd-47a4-b5ff-bfb5022834c2", "Volkan Baǵa"),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_land(&["Forest", "Plains"]).with_ability(abilities::shock_land_enters()),
);

// RAV 286 — Watery Grave
pub(in crate::card::sets) static WATERY_GRAVE: CardRecord = CardRecord::new_with_legacy_id(
    1142,
    "Watery Grave",
    CardArt::new("47fde349-010e-4a2e-838e-e924dbeec355", "Raymond Swanland"),
    CardSet::RavnicaCityOfGuilds,
    CardRules::new_land(&["Island", "Swamp"]).with_ability(abilities::shock_land_enters()),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &FAITH_S_FETTERS,
    &BELLTOWER_SPHINX,
    &PEEL_FROM_REALITY,
    &REMAND,
    &VEDALKEN_ENTRANCER,
    &DARK_CONFIDANT,
    &FRENZIED_GOBLIN,
    &REROUTE,
    &FARSEEK,
    &PUTREFY,
    &SKYKNIGHT_LEGIONNAIRE,
    &DIMIR_GUILDMAGE,
    &BOROS_GARRISON,
    &DIMIR_AQUEDUCT,
    &GOLGARI_ROT_FARM,
    &OVERGROWN_TOMB,
    &SACRED_FOUNDRY,
    &SELESNYA_SANCTUARY,
    &TEMPLE_GARDEN,
    &WATERY_GRAVE,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
