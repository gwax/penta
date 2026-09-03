//! Commander Legends: Battle for Baldur's Gate cards cataloged for the
//! Vintage Cube.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::sets::y2021::adventures_in_the_forgotten_realms as catalog_afr;
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, CardArt, CardRules, CardSet,
    CardSupertype, CardType, DeckConstructionDef, EffectDef, EffectRecipientDef, ManaColor,
    ObjectPredicateDef, PlayerRelation, SacrificedAmountDef, TriggerConditionDef, TriggerEventDef,
    ValueDef, ZoneKind, abilities,
};
use crate::{TargetIndex, mana_cost};

// CLB 11 — Blessed Hippogriff
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static BLESSED_HIPPOGRIFF: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("b4590e53-ca8d-4896-a8cf-6af1e4bc456f"),
    "Blessed Hippogriff",
    crate::card::CardArt::new("b4590e53-ca8d-4896-a8cf-6af1e4bc456f", "Leanna Crossan"),
    crate::card::CardSet::CommanderLegendsBattleForBaldursGate,
    crate::card::CardRules::unsupported(),
);

// CLB 22 — Greatsword of Tyr
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static GREATSWORD_OF_TYR: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("50088a60-642b-47ed-a289-ef0b617b688f"),
    "Greatsword of Tyr",
    crate::card::CardArt::new("50088a60-642b-47ed-a289-ef0b617b688f", "Titus Lunter"),
    crate::card::CardSet::CommanderLegendsBattleForBaldursGate,
    crate::card::CardRules::unsupported(),
);

// CLB 99 — Sword Coast Serpent
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SWORD_COAST_SERPENT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("0bbfb7ae-9a32-428d-903c-99d0d8669b8d"),
    "Sword Coast Serpent",
    crate::card::CardArt::new("0bbfb7ae-9a32-428d-903c-99d0d8669b8d", "Caio Monteiro"),
    crate::card::CardSet::CommanderLegendsBattleForBaldursGate,
    crate::card::CardRules::unsupported(),
);

// CLB 106 — Young Blue Dragon
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static YOUNG_BLUE_DRAGON: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("56b0f66b-dca9-4a01-9394-20a513c2b225"),
    "Young Blue Dragon",
    crate::card::CardArt::new("56b0f66b-dca9-4a01-9394-20a513c2b225", "Tuan Duong Chu"),
    crate::card::CardSet::CommanderLegendsBattleForBaldursGate,
    crate::card::CardRules::unsupported(),
);

// CLB 113 — Arms of Hadar
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static ARMS_OF_HADAR: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("db1fd431-8f6d-4ca5-bc0c-53881c500da1"),
    "Arms of Hadar",
    crate::card::CardArt::new("db1fd431-8f6d-4ca5-bc0c-53881c500da1", "Mirko Failoni"),
    crate::card::CardSet::CommanderLegendsBattleForBaldursGate,
    crate::card::CardRules::unsupported(),
);

// CLB 119 — Cast Down (reprint)
const CAST_DOWN_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y2018::dominaria::CAST_DOWN)
        .with_art("aba79021-39af-4e74-beb5-f2f508c865b2", "Tyler Walpole");

// CLB 130 — Guildsworn Prowler
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static GUILDSWORN_PROWLER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("d7efb10f-c760-431c-8ac6-904965d850dc"),
    "Guildsworn Prowler",
    crate::card::CardArt::new("d7efb10f-c760-431c-8ac6-904965d850dc", "Fariba Khamseh"),
    crate::card::CardSet::CommanderLegendsBattleForBaldursGate,
    crate::card::CardRules::unsupported(),
);

// CLB 180 — Gut, True Soul Zealot
pub(in crate::card::sets) static GUT_TRUE_SOUL_ZEALOT: CardRecord = CardRecord::new_with_legacy_id(
    2211,
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
                // "Whenever you attack" is one or more creatures you control attacking,
                // counted once for the declaration rather than once per attacker.
                TriggerEventDef::attack_declared(
                    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
                    1,
                    None,
                ),
                EffectDef::SacrificeOfChoice {
                    count: ValueDef::Constant(1),
                    player: EffectRecipientDef::Controller,
                    // "Another creature or an artifact." Gut is neither an artifact nor another
                    // creature, so the exclusion covers both halves without saying so twice.
                    object: ObjectPredicateDef::All(&[
                        ObjectPredicateDef::AnyOf(&[
                            ObjectPredicateDef::HasType(CardType::Creature),
                            ObjectPredicateDef::HasType(CardType::Artifact),
                        ]),
                        ObjectPredicateDef::ControlledBy(PlayerRelation::You),
                        ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
                    ]),
                    // The token arrives already attacking, which is the whole point: it was
                    // never declared, so nothing that watches a declaration sees it, and it
                    // still connects this combat.
                    then: Some(&EffectDef::create_creature_token(&["Skeleton"], &[ManaColor::Black], 4, 1)
                            .with_abilities(&[abilities::menace()])
                            .with_art(CardArt::new(
                                "cf4c245f-af2f-46a7-81f3-670a04940901",
                                "David Astruga",
                            ))
                            .entering_tapped()
                            .entering_attacking()),
                    amount: SacrificedAmountDef::Power,
                    otherwise: None,
                    optional: true,
                },
            ),
            AbilityDef::deck_construction(
                "Choose a Background (You can have a Background as a second commander.)",
                DeckConstructionDef::ChooseABackground,
                "The parenthesis is the whole sentence: it is a deck-construction \
                 permission, checked where a Commander list is assembled and silent \
                 once the game starts.",
            ),
        ]),
);

// CLB 263 — You Meet in a Tavern (reprint)
const YOU_MEET_IN_A_TAVERN_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&catalog_afr::YOU_MEET_IN_A_TAVERN)
        .with_art("9fddbd7a-799c-4432-810c-d839c5c354b9", "Zoltan Boros");

// CLB 285 — Minsc & Boo, Timeless Heroes
// Audit: unsupported — Needs a sacrifice-scoped reflexive trigger that chooses its damage target after the sacrifice.
pub(in crate::card::sets) static MINSC_BOO_TIMELESS_HEROES: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("928036c9-11b8-493e-b9f2-8fbd3487cd19"),
    "Minsc & Boo, Timeless Heroes",
    CardArt::new("928036c9-11b8-493e-b9f2-8fbd3487cd19", "Andreas Zafiratos"),
    CardSet::CommanderLegendsBattleForBaldursGate,
    CardRules::unsupported(),
);

// CLB 346 — Basilisk Gate
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static BASILISK_GATE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("4a306025-d429-4006-b7ed-bdb287e83f57"),
    "Basilisk Gate",
    crate::card::CardArt::new("935f3dfa-7d8d-459a-8ac2-37892cb9545f", "Jorge Jacinto"),
    crate::card::CardSet::CommanderLegendsBattleForBaldursGate,
    crate::card::CardRules::unsupported(),
);

// CLB 560 — Displacer Kitten
pub(in crate::card::sets) static DISPLACER_KITTEN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("9a53e8fc-bfd2-4866-a61c-f3204b0a98bf"),
    "Displacer Kitten",
    CardArt::new("9a53e8fc-bfd2-4866-a61c-f3204b0a98bf", "Campbell White"),
    CardSet::CommanderLegendsBattleForBaldursGate,
    // Four mana for a 2/2 that does nothing on its own and everything in a
    // deck built to cast noncreature spells: every one of them is another
    // enter trigger off whatever is already on the battlefield.
    CardRules::new_creature(mana_cost!("{3}{U}"), &["Cat", "Beast"], 2, 2).with_ability(
        AbilityDef::triggered_with_targets(
            "Avoidance — Whenever you cast a noncreature spell, exile up to one target nonland \
             permanent you control, then return that card to the battlefield under its owner's \
             control.",
            // A noncreature spell you cast. What it does is no part of the condition:
            // the Kitten reads the type line and nothing else.
            TriggerEventDef::spell_cast(ObjectPredicateDef::All(&[
                ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Creature)),
                ObjectPredicateDef::ControlledBy(PlayerRelation::You),
            ])),
            // "Up to one target nonland permanent you control": the trigger goes on the
            // stack whether or not there is anything worth blinking.
            &[AbilityTargetDef::up_to(
                AbilityTargetPredicate::Object {
                    object: ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Land)),
                    zones: &[ZoneKind::Battlefield],
                    controller: Some(PlayerRelation::You),
                    owner: None,
                },
                1,
            )],
            // Exiling links the permanent to the Kitten, which is what lets the return
            // name the card the exile just made.
            EffectDef::Sequence(&[
                EffectDef::ExileLinkedToSource {
                    until_source_leaves: false,
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    face_down: false,
                    then: None,
                },
                EffectDef::ReturnLinkedExiles {
                    object: ObjectPredicateDef::Any,
                    counters: None,
                    zone: ZoneKind::Battlefield,
                    grant: None,
                    controller: None,
                    transformed: false,
                },
            ]),
        ),
    ),
);

// CLB 630 — Delayed Blast Fireball
pub(in crate::card::sets) static DELAYED_BLAST_FIREBALL: CardRecord =
    CardRecord::new_with_legacy_id(
        2299,
        "Delayed Blast Fireball",
        CardArt::new("400c76c6-f677-4e7e-87ad-2e526d4b498a", "Andreas Zafiratos"),
        CardSet::CommanderLegendsBattleForBaldursGate,
        // A one-sided sweeper that costs a turn of setup, which is the trade the
        // cube's aggressive decks are least able to make and the slow ones most.
        CardRules::new_instant(mana_cost!("{1}{R}{R}")).with_abilities(&[
            AbilityDef::spell(
                "Delayed Blast Fireball deals 2 damage to each opponent and each creature they \
             control. If this spell was cast from exile, it deals 5 damage to each opponent and \
             each creature they control instead.",
                EffectDef::IfElseCondition {
                    condition: &TriggerConditionDef::SourceCastFrom(ZoneKind::Exile),
                    then: &EffectDef::DealDamage {
                        recipient: EffectRecipientDef::EachOpponentAndTheirCreatures,
                        amount: ValueDef::Constant(5),
                    },
                    // Two damage as the baseline and five when it was foretold, which is the
                    // whole of the card: the two mana spent a turn earlier buy three damage and
                    // one mana off the price.
                    otherwise: &EffectDef::DealDamage {
                        recipient: EffectRecipientDef::EachOpponentAndTheirCreatures,
                        amount: ValueDef::Constant(2),
                    },
                },
            ),
            abilities::foretell(mana_cost!("{4}{R}{R}")),
        ]),
    );

// CLB 748 — Dauthi Horror (reprint)
const DAUTHI_HORROR_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y1997::tempest::DAUTHI_HORROR)
        .with_art("7c41afe6-7eed-4cf5-9bbb-ccc9f82cb4fa", "Jeff Laubenstein");

// CLB 897 — Izzet Boilerworks (reprint)
const IZZET_BOILERWORKS_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y2006::guildpact::IZZET_BOILERWORKS)
        .with_art("c86e42c6-342b-443f-9b99-a68cf536ff45", "John Avon");

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &BLESSED_HIPPOGRIFF,
    &GREATSWORD_OF_TYR,
    &SWORD_COAST_SERPENT,
    &YOUNG_BLUE_DRAGON,
    &ARMS_OF_HADAR,
    &GUILDSWORN_PROWLER,
    &GUT_TRUE_SOUL_ZEALOT,
    &MINSC_BOO_TIMELESS_HEROES,
    &BASILISK_GATE,
    &DISPLACER_KITTEN,
    &DELAYED_BLAST_FIREBALL,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[
    CAST_DOWN_REPRINT,
    YOU_MEET_IN_A_TAVERN_REPRINT,
    DAUTHI_HORROR_REPRINT,
    IZZET_BOILERWORKS_REPRINT,
];
