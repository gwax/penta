//! Shards of Alara cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::CostQuantityDef;
use crate::card::SpellAdditionalCostDef;
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef,
    CardArt, CardRules, CardSet, CardSupertype, CardType, EffectDef, EffectRecipientDef, ManaColor,
    ObjectPredicateDef, ObjectRefDef, PlayerRefDef, PlayerRelation, ResolvedEffectDurationDef,
    TriggerEventDef, ValueDef, ZoneKind, abilities, tokens,
};
use crate::ids::ParentBinding;
use crate::{TargetIndex, mana_cost};

// ALA 3 — Angelic Benediction
pub(in crate::card::sets) static ANGELIC_BENEDICTION: CardRecord = CardRecord::new_with_legacy_id(
    1501,
    "Angelic Benediction",
    CardArt::new("22125507-31e3-424c-9527-d994e4525d75", "Michael Komarck"),
    CardSet::ShardsOfAlara,
    CardRules::new_enchantment(mana_cost!("{3}{W}")).with_abilities(&[
        abilities::exalted(),
        AbilityDef::triggered_with_targets(
            "Whenever a creature you control attacks alone, you may tap target creature.",
            TriggerEventDef::attacks_in_declaration(
                ObjectPredicateDef::ControlledBy(PlayerRelation::You),
                1,
                Some(1),
            ),
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::HasType(CardType::Creature),
            )],
            EffectDef::May {
                player: EffectRecipientDef::Controller,
                effect: &EffectDef::Tap {
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                },
            },
        ),
    ]),
);

// ALA 9 — Elspeth, Knight-Errant
pub(in crate::card::sets) static ELSPETH_KNIGHT_ERRANT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("44c52e52-2b1c-4ca8-ab6d-20d97a342704"),
    "Elspeth, Knight-Errant",
    CardArt::new("44c52e52-2b1c-4ca8-ab6d-20d97a342704", "Volkan Ba\u{11f}a"),
    CardSet::ShardsOfAlara,
    // Four mana, two plus abilities, and neither of them is the safe one:
    // she makes a blocker or she makes an attacker, and the ultimate ends
    // the game against anything that answers permanents.
    CardRules::new_planeswalker(mana_cost!("{2}{W}{W}"), &["Elspeth"], 4)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&[
            AbilityDef::activated(
                "+1: Create a 1/1 white Soldier creature token.",
                &[AbilityCostDef::Loyalty(1)],
                EffectDef::CreateToken {
                    token: tokens::creature(&["Soldier"], &[ManaColor::White], 1, 1),
                    copy: None,
                    controller: None,
                    count: ValueDef::Constant(1),
                    tapped: false,
                    attacking: false,
                    counters: None,
                    created: None,
                },
            ),
            // The second plus is what makes her a threat rather than a hedge: any
            // creature, so the token she made last turn is a 4/4 flier this one.
            AbilityDef::activated_with_targets(
                "+1: Target creature gets +3/+3 and gains flying until end of turn.",
                &[AbilityCostDef::Loyalty(1)],
                &[AbilityTargetDef::exactly_one(
                    AbilityTargetPredicate::Object {
                        object: ObjectPredicateDef::HasType(CardType::Creature),
                        zones: &[ZoneKind::Battlefield],
                        controller: None,
                        owner: None,
                    },
                )],
                EffectDef::Apply {
                    recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    effect: AppliedEffectDef::Composite(&[
                        AppliedEffectDef::modify_power_toughness(ValueDef::Constant(3), ValueDef::Constant(3)),
                        AppliedEffectDef::add_ability(&abilities::flying()),
                    ]),
                    duration: ResolvedEffectDurationDef::UntilEndOfTurn,
                },
            ),
            AbilityDef::activated(
                "\u{2212}8: You get an emblem with \"Artifacts, creatures, enchantments, and lands you \
                 control have indestructible.\"",
                &[AbilityCostDef::Loyalty(-8)],
                EffectDef::create_emblem("Elspeth, Knight-Errant emblem", &[AbilityDef::static_ability(
                    "Artifacts, creatures, enchantments, and lands you control have indestructible.",
                    EffectDef::StaticApply {
                        recipient: EffectRecipientDef::matching_objects(
                            // The four types the emblem names, which between them are every permanent
                            // a white deck is likely to control. Written as one alternation because the
                            // emblem grants one thing to all of them.
                            ObjectPredicateDef::AnyOf(&[
                                ObjectPredicateDef::HasType(CardType::Artifact),
                                ObjectPredicateDef::HasType(CardType::Creature),
                                ObjectPredicateDef::HasType(CardType::Enchantment),
                                ObjectPredicateDef::HasType(CardType::Land),
                            ]),
                            &[ZoneKind::Battlefield],
                            PlayerRelation::You,
                        ),
                        effect: AppliedEffectDef::add_ability(&abilities::indestructible()),
                    },
                )]),
            ),
        ]),
);

// ALA 12 — Guardians of Akrasa
pub(in crate::card::sets) static GUARDIANS_OF_AKRASA: CardRecord = CardRecord::new_with_legacy_id(
    1503,
    "Guardians of Akrasa",
    CardArt::new("383c9aa5-30ad-4a2a-8b64-65d4b333c613", "Alan Pollack"),
    CardSet::ShardsOfAlara,
    CardRules::new_creature(mana_cost!("{2}{W}"), &["Human", "Soldier"], 0, 4)
        .with_abilities(&[abilities::defender(), abilities::exalted()]),
);

// ALA 67 — Bone Splinters
pub(in crate::card::sets) static BONE_SPLINTERS: CardRecord = CardRecord::new_with_legacy_id(
    1962,
    "Bone Splinters",
    CardArt::new("387eda28-f35b-48b0-ba59-773d82902327", "Nils Hamm"),
    CardSet::ShardsOfAlara,
    // The sacrifice is paid on the way to the stack, so the creature it eats
    // is gone before the target is destroyed -- and the spell can eat the
    // very creature it is aimed at only if something else is left to target.
    CardRules::new_sorcery(mana_cost!("{B}")).with_ability(AbilityDef::spell_with_additional_cost(
        "As an additional cost to cast this spell, sacrifice a creature.\nDestroy target \
             creature.",
        &[AbilityTargetDef::exactly_one_permanent(
            ObjectPredicateDef::HasType(CardType::Creature),
        )],
        SpellAdditionalCostDef::sacrifice(
            ObjectPredicateDef::HasType(CardType::Creature),
            CostQuantityDef::Fixed(1),
        ),
        EffectDef::Destroy {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            can_regenerate: true,
            then: None,
        },
    )),
);

// ALA 100 — Flameblast Dragon
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static FLAMEBLAST_DRAGON: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("5544b26b-0bc4-4c1b-9616-613e9bf08557"),
    "Flameblast Dragon",
    crate::card::CardArt::new("c01ab5c8-f9b7-482c-a900-1388b727b89f", "Jaime Jones"),
    crate::card::CardSet::ShardsOfAlara,
    crate::card::CardRules::unsupported(),
);

// ALA 104 — Hissing Iguanar
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static HISSING_IGUANAR: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("4b8b8b90-cb6e-4910-bc40-d96b78b0d70c"),
    "Hissing Iguanar",
    crate::card::CardArt::new("4b8b8b90-cb6e-4910-bc40-d96b78b0d70c", "Brandon Kitkouski"),
    crate::card::CardSet::ShardsOfAlara,
    crate::card::CardRules::unsupported(),
);

// ALA 107 — Lightning Talons
pub(in crate::card::sets) static LIGHTNING_TALONS: CardRecord = CardRecord::new_with_legacy_id(
    1204,
    "Lightning Talons",
    CardArt::new("87186a8a-45da-4cde-a167-c16a6abc4d24", "Johann Bodin"),
    CardSet::ShardsOfAlara,
    CardRules::new_enchantment(mana_cost!("{2}{R}"))
        .with_subtypes(&["Aura"])
        .with_abilities(&[
            AbilityDef::spell_with_targets(
                "Enchant creature",
                &[AbilityTargetDef::exactly_one_permanent(
                    ObjectPredicateDef::HasType(CardType::Creature),
                )],
                EffectDef::Attach {
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                },
            ),
            AbilityDef::static_ability(
                "Enchanted creature gets +3/+0 and has first strike.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::Composite(&[
                        AppliedEffectDef::modify_power_toughness(
                            ValueDef::Constant(3),
                            ValueDef::Constant(0),
                        ),
                        AppliedEffectDef::add_ability(&abilities::first_strike()),
                    ]),
                },
            ),
        ]),
);

// ALA 130 — Elvish Visionary
pub(in crate::card::sets) static ELVISH_VISIONARY: CardRecord = CardRecord::new_with_legacy_id(
    1034,
    "Elvish Visionary",
    CardArt::new(
        "65ea2998-ed91-43b8-bd81-b01a6c24a5b0",
        "D. Alexander Gregory",
    ),
    CardSet::ShardsOfAlara,
    CardRules::new_creature(mana_cost!("{1}{G}"), &["Elf", "Shaman"], 1, 1).with_ability(
        abilities::enters_trigger(
            "When this creature enters, draw a card.",
            EffectDef::DrawCards {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(1),
            },
        ),
    ),
);

// ALA 156 — Blightning
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static BLIGHTNING: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("3c05e8a2-b7d0-4f24-b2ae-8e4db30e5842"),
    "Blightning",
    crate::card::CardArt::new("3c05e8a2-b7d0-4f24-b2ae-8e4db30e5842", "Thomas M. Baxa"),
    crate::card::CardSet::ShardsOfAlara,
    crate::card::CardRules::unsupported(),
);

// ALA 158 — Branching Bolt
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static BRANCHING_BOLT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("e7468876-f401-4a75-81c0-bed09cdda3e1"),
    "Branching Bolt",
    crate::card::CardArt::new("e7468876-f401-4a75-81c0-bed09cdda3e1", "Vance Kovacs"),
    crate::card::CardSet::ShardsOfAlara,
    crate::card::CardRules::unsupported(),
);

// ALA 202 — Tidehollow Sculler
pub(in crate::card::sets) static TIDEHOLLOW_SCULLER: CardRecord = CardRecord::new_with_legacy_id(
    2145,
    "Tidehollow Sculler",
    CardArt::new("1abecc77-07f2-43e4-8585-0a8199cdcf01", "rk post"),
    CardSet::ShardsOfAlara,
    CardRules::new_artifact_creature(mana_cost!("{W}{B}"), &["Zombie"], 2, 2)
        .with_abilities(&[
            abilities::enters_trigger_with_targets(
                "When this creature enters, target opponent reveals their hand and you choose a nonland card from it. Exile that card.",
                &[AbilityTargetDef::exactly_one(
                    AbilityTargetPredicate::Player(PlayerRelation::Opponent),
                )],
                EffectDef::Sequence(&abilities::reveal_hand_and_choose_card(
                    PlayerRefDef::Target(TargetIndex::PRIMARY),
                    ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Land)),
                    // Linked to the Sculler rather than exiled outright, which is the whole
                    // bargain: the card is gone only for as long as the body survives.
                    &EffectDef::ExileLinkedToSource {
                        until_source_leaves: false,
                        object: EffectRecipientDef::object(ObjectRefDef::Binding(ParentBinding)),
                        face_down: false,
                        then: None,
                    },
                )),
            ),
            // Leaves, not dies: bouncing or exiling the Sculler gives the card back
            // just as killing it does.
            AbilityDef::triggered(
                "When this creature leaves the battlefield, return the exiled card to its owner's hand.",
                TriggerEventDef::zone_changed(
                    ObjectPredicateDef::Source,
                    Some(ZoneKind::Battlefield),
                    None,
                ),
                EffectDef::ReturnLinkedExiles {
                    object: ObjectPredicateDef::Any,
                    counters: None,
                    zone: ZoneKind::Hand,
                    grant: None,
                    controller: None,
                    transformed: false,
                },
            ),
        ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &ANGELIC_BENEDICTION,
    &ELSPETH_KNIGHT_ERRANT,
    &GUARDIANS_OF_AKRASA,
    &BONE_SPLINTERS,
    &FLAMEBLAST_DRAGON,
    &HISSING_IGUANAR,
    &LIGHTNING_TALONS,
    &ELVISH_VISIONARY,
    &BLIGHTNING,
    &BRANCHING_BOLT,
    &TIDEHOLLOW_SCULLER,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
