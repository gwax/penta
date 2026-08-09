//! Magic 2014 card records used by the built-in ISD–RTR Standard decks.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityImplementationDef, AbilityTargetDef, AbilityTargetPredicate,
    CardArt, CardBehavior, CardRules, CardSet, CardSupertype, CardType, CounterKind, EffectDef,
    EffectRecipientDef, LandEntry, ManaColor, ObjectPredicateDef, PlayerRelation, TriggerEventDef,
    ValueDef, ZoneKind, abilities, cards,
};
use crate::ids::TargetSlotId;
use crate::mana_cost;

pub(in crate::card::sets) static ARCHANGEL_OF_THUNE: CardRecord = CardRecord::new(
    cards::ARCHANGEL_OF_THUNE,
    "Archangel of Thune",
    CardArt::new("531cba81-afd7-4be4-adec-87edb77ba2a9", "James Ryman"),
    CardSet::Magic2014,
    CardRules::new_creature(mana_cost!("{3}{W}{W}"), &["Angel"], 3, 4).with_abilities(&[
        abilities::flying(),
        abilities::lifelink().with_text(
            "Lifelink (Damage dealt by this creature also causes you to gain that much life.)",
        ),
        AbilityDef::triggered(
            "Whenever you gain life, put a +1/+1 counter on each creature you control.",
            TriggerEventDef::LifeGained(PlayerRelation::You),
            EffectDef::AddCounters {
                kind: CounterKind::PlusOnePlusOne,
                object: EffectRecipientDef::MatchingObjects {
                    object: ObjectPredicateDef::All(&[
                        ObjectPredicateDef::HasType(CardType::Creature),
                        ObjectPredicateDef::ControlledBy(PlayerRelation::You),
                    ]),
                    zones: &[ZoneKind::Battlefield],
                    controller: PlayerRelation::You,
                },
                // One counter however much life arrived, and one trigger for
                // each separate gain.
                amount: ValueDef::Constant(1),
            },
        ),
    ]),
);

pub(in crate::card::sets) static BURNING_EARTH: CardRecord = CardRecord::new(
    cards::BURNING_EARTH,
    "Burning Earth",
    CardArt::new("1df3a7c9-5c8d-438c-a5ad-3c9754c6ea5d", "rk post"),
    CardSet::Magic2014,
    CardRules::new_enchantment(mana_cost!("{3}{R}")).with_ability(
        AbilityDef::triggered(
            "Whenever a player taps a nonbasic land for mana, this enchantment deals 1 damage to that player.",
            TriggerEventDef::TappedForMana(ObjectPredicateDef::All(&[
                ObjectPredicateDef::HasType(CardType::Land),
                ObjectPredicateDef::Not(&ObjectPredicateDef::Supertype(CardSupertype::Basic)),
            ])),
            EffectDef::DealDamage {
                // Whoever tapped it, which includes this enchantment's own
                // controller.
                recipient: EffectRecipientDef::ControllerOfTriggeringObject,
                amount: ValueDef::Constant(1),
            },
        ),
    ),
);

pub(in crate::card::sets) static CELESTIAL_FLARE: CardRecord = CardRecord::new(
    cards::CELESTIAL_FLARE,
    "Celestial Flare",
    CardArt::new("6c8d1320-0f1a-4c66-86c9-9f8da0f1d9ef", "Clint Cearley"),
    CardSet::Magic2014,
    CardRules::new_instant(mana_cost!("{W}{W}")).with_ability(
        AbilityDef::spell(
            "Target player sacrifices an attacking or blocking creature of their choice.",
            EffectDef::SacrificeOfChoice {
                player: EffectRecipientDef::Target(TargetSlotId(0)),
                object: ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Creature),
                    ObjectPredicateDef::AttackingOrBlocking,
                ]),
            },
        )
        .with_targets(&[AbilityTargetDef::exactly_one(
            TargetSlotId(0),
            "player",
            AbilityTargetPredicate::Player(PlayerRelation::Any),
        )]),
    ),
);

pub(in crate::card::sets) static DOOM_BLADE: CardRecord = CardRecord::new(
    cards::DOOM_BLADE,
    "Doom Blade",
    CardArt::new("75d96a37-bdbe-46ae-926f-8742699a0b20", "Chippy"),
    CardSet::Magic2014,
    CardRules::new_instant(mana_cost!("{1}{B}")).with_ability(AbilityDef::custom_full(
        "Destroy target nonblack creature.",
        CardBehavior::DoomBlade,
        "Implemented by the named card-local special behavior.",
    )),
);

pub(in crate::card::sets) static ELVISH_MYSTIC: CardRecord = CardRecord::new(
    cards::ELVISH_MYSTIC,
    "Elvish Mystic",
    CardArt::new("60d0e6a6-629a-45a7-bfcb-25ba7156788b", "Wesley Burt"),
    CardSet::Magic2014,
    CardRules::new_creature(mana_cost!("{G}"), &["Elf", "Druid"], 1, 1)
        .with_abilities(&[abilities::tap_for(ManaColor::Green)]),
);

pub(in crate::card::sets) static ENCROACHING_WASTES: CardRecord = CardRecord::new(
    cards::ENCROACHING_WASTES,
    "Encroaching Wastes",
    CardArt::new("1ad5a84b-ae9b-4ed1-a4de-b91bbf8ed0a5", "Noah Bradley"),
    CardSet::Magic2014,
    CardRules::new_land(&[])
        .land_entry(LandEntry::Untapped)
        .with_abilities(&[
            abilities::tap_for(ManaColor::Colorless),
            AbilityDef::activated(
                "{4}, {T}, Sacrifice this land: Destroy target nonbasic land.",
                &[
                    AbilityCostDef::Mana(mana_cost!("{4}")),
                    AbilityCostDef::TapSource,
                    AbilityCostDef::SacrificeSource,
                ],
                EffectDef::Destroy {
                    object: EffectRecipientDef::Target(TargetSlotId(0)),
                    can_regenerate: true,
                },
            )
            .with_targets(&[AbilityTargetDef::exactly_one_permanent(
                TargetSlotId(0),
                "nonbasic land",
                ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Land),
                    ObjectPredicateDef::Not(&ObjectPredicateDef::Supertype(CardSupertype::Basic)),
                ]),
            )]),
        ]),
);

pub(in crate::card::sets) static LIFEBANE_ZOMBIE: CardRecord = CardRecord::new(
    cards::LIFEBANE_ZOMBIE,
    "Lifebane Zombie",
    CardArt::new("98370735-5303-40d4-9e80-cdb40dee18e2", "Min Yum"),
    CardSet::Magic2014,
    CardRules::new_creature(
        mana_cost!("{1}{B}{B}"),
        &["Zombie", "Warrior"],
        3,
        1,
    )
    .with_abilities(&[
        abilities::intimidate().with_text(
            "Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)",
        ),
        AbilityDef::triggered(
            "When this creature enters, target opponent reveals their hand. You choose a green or white creature card from it and exile that card.",
            TriggerEventDef::ZoneChanged {
                object: ObjectPredicateDef::Source,
                from: None,
                to: Some(ZoneKind::Battlefield),
            },
            EffectDef::None,
        )
        .with_targets(&[AbilityTargetDef::exactly_one(
            TargetSlotId(0),
            "target opponent",
            AbilityTargetPredicate::Player(PlayerRelation::Opponent),
        )])
        .with_implementation(AbilityImplementationDef::CustomFull {
            behavior: Some(CardBehavior::LifebaneZombie),
            explanation: "The targeted trigger uses the shared stack and a card-local hand-reveal and exile resolver.",
        }),
    ]),
);

pub(in crate::card::sets) static MUTAVAULT: CardRecord = CardRecord::new(
    cards::MUTAVAULT,
    "Mutavault",
    CardArt::new("927ed667-c228-4b96-a9f6-7cbadade8134", "Fred Fields"),
    CardSet::Magic2014,
    CardRules::new_land(&[])
    .land_entry(LandEntry::Untapped)
    .with_abilities(&[
        abilities::tap_for(ManaColor::Colorless),
        AbilityDef::not_implemented(
            "{1}: This land becomes a 2/2 creature with all creature types until end of turn. It's still a land.",
            "The animation activated ability is not executed.",
        ),
    ]),
);

pub(in crate::card::sets) static PRIMEVAL_BOUNTY: CardRecord = CardRecord::new(
    cards::PRIMEVAL_BOUNTY,
    "Primeval Bounty",
    CardArt::new("e750d55d-d5e8-4abe-99cf-f6b8ba86cf16", "Christine Choi"),
    CardSet::Magic2014,
    CardRules::new_enchantment(mana_cost!("{5}{G}")).with_abilities(&[
        AbilityDef::triggered(
            "Whenever you cast a creature spell, create a 3/3 green Beast creature token.",
            TriggerEventDef::SpellCast(ObjectPredicateDef::All(&[
                ObjectPredicateDef::HasType(CardType::Creature),
                ObjectPredicateDef::ControlledBy(PlayerRelation::You),
            ])),
            EffectDef::CreateToken {
                token: cards::BEAST_TOKEN_3_3_GREEN,
                count: ValueDef::Constant(1),
            },
        ),
        AbilityDef::triggered(
            "Whenever you cast a noncreature spell, put three +1/+1 counters on target creature you control.",
            TriggerEventDef::SpellCast(ObjectPredicateDef::All(&[
                ObjectPredicateDef::NoncreatureSpell,
                ObjectPredicateDef::ControlledBy(PlayerRelation::You),
            ])),
            EffectDef::AddCounters {
                kind: CounterKind::PlusOnePlusOne,
                object: EffectRecipientDef::Target(TargetSlotId(0)),
                amount: ValueDef::Constant(3),
            },
        )
        .with_targets(&[AbilityTargetDef::exactly_one_permanent(
            TargetSlotId(0),
            "creature you control",
            ObjectPredicateDef::All(&[
                ObjectPredicateDef::HasType(CardType::Creature),
                ObjectPredicateDef::ControlledBy(PlayerRelation::You),
            ]),
        )]),
        AbilityDef::triggered(
            "Landfall — Whenever a land you control enters, you gain 3 life.",
            TriggerEventDef::ZoneChanged {
                object: ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Land),
                    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
                ]),
                from: None,
                to: Some(ZoneKind::Battlefield),
            },
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(3),
            },
        ),
    ]),
);

pub(in crate::card::sets) static QUICKEN: CardRecord = CardRecord::new(
    cards::QUICKEN,
    "Quicken",
    CardArt::new("066bef3d-c785-4b25-9b91-8f676aa9906f", "Aleksi Briclot"),
    CardSet::Magic2014,
    CardRules::new_instant(mana_cost!("{U}")).with_abilities(&[
        AbilityDef::not_implemented(
            "The next sorcery spell you cast this turn can be cast as though it had flash. (It can be cast any time you could cast an instant.)",
            "Granting flash to a later spell is not implemented.",
        ),
        AbilityDef::spell(
            "Draw a card.",
            EffectDef::DrawCards {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(1),
            },
        ),
    ]),
);

pub(in crate::card::sets) static RATCHET_BOMB: CardRecord = CardRecord::new(
    cards::RATCHET_BOMB,
    "Ratchet Bomb",
    CardArt::new("3e9045df-3eff-4236-9bbb-77537b302e27", "Austin Hsu"),
    CardSet::Magic2014,
    CardRules::new_artifact(mana_cost!("{2}")).with_abilities(&[
        AbilityDef::activated(
            "{T}: Put a charge counter on this artifact.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddCounters {
                object: EffectRecipientDef::Source,
                kind: CounterKind::Charge,
                amount: ValueDef::Constant(1),
            },
        ),
        AbilityDef::activated(
            "{T}, Sacrifice this artifact: Destroy each nonland permanent with mana value equal to the number of charge counters on this artifact.",
            &[AbilityCostDef::TapSource, AbilityCostDef::SacrificeSource],
            EffectDef::Destroy {
                object: EffectRecipientDef::MatchingObjects {
                    object: ObjectPredicateDef::All(&[
                        ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Land)),
                        // The Bomb is already gone by the time this resolves,
                        // so the count comes from last-known information.
                        ObjectPredicateDef::ManaValueEqualTo(ValueDef::CountersOnSource(
                            CounterKind::Charge,
                        )),
                    ]),
                    zones: &[ZoneKind::Battlefield],
                    controller: PlayerRelation::Any,
                },
                can_regenerate: true,
            },
        ),
    ]),
);

pub(in crate::card::sets) static SCAVENGING_OOZE: CardRecord = CardRecord::new(
    cards::SCAVENGING_OOZE,
    "Scavenging Ooze",
    CardArt::new("ec30153a-36b5-42f8-beed-9efab09f1051", "Austin Hsu"),
    CardSet::Magic2014,
    CardRules::new_creature(
        mana_cost!("{1}{G}"),
        &["Ooze"],
        2,
        2,
    )
    .with_ability(AbilityDef::not_implemented(
        "{G}: Exile target card from a graveyard. If it was a creature card, put a +1/+1 counter on this creature and you gain 1 life.",
        "Printed rules are cataloged but are not executed by the engine.",
    )),
);

pub(in crate::card::sets) static SHADOWBORN_DEMON: CardRecord = CardRecord::new(
    cards::SHADOWBORN_DEMON,
    "Shadowborn Demon",
    CardArt::new("3884c05b-c10e-4f1d-a8bd-8b5118657972", "Lucas Graciano"),
    CardSet::Magic2014,
    CardRules::new_creature(
        mana_cost!("{3}{B}{B}"),
        &["Demon"],
        5,
        6,
    )
    .with_abilities(&[
        abilities::flying(),
        AbilityDef::triggered(
            "When this creature enters, destroy target non-Demon creature.",
            TriggerEventDef::ZoneChanged {
                object: ObjectPredicateDef::Source,
                from: None,
                to: Some(ZoneKind::Battlefield),
            },
            EffectDef::Destroy {
                object: EffectRecipientDef::Target(TargetSlotId(0)),
                can_regenerate: true,
            },
        )
        .with_targets(&[AbilityTargetDef::exactly_one_permanent(
            TargetSlotId(0),
            "non-Demon creature",
            ObjectPredicateDef::All(&[
                ObjectPredicateDef::HasType(CardType::Creature),
                ObjectPredicateDef::Not(&ObjectPredicateDef::Subtype("Demon")),
            ]),
        )]),
        AbilityDef::not_implemented(
            "At the beginning of your upkeep, if there are fewer than six creature cards in your graveyard, sacrifice a creature.",
            "The conditional upkeep sacrifice trigger is not executed.",
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &ARCHANGEL_OF_THUNE,
    &BURNING_EARTH,
    &CELESTIAL_FLARE,
    &DOOM_BLADE,
    &ELVISH_MYSTIC,
    &ENCROACHING_WASTES,
    &LIFEBANE_ZOMBIE,
    &MUTAVAULT,
    &PRIMEVAL_BOUNTY,
    &QUICKEN,
    &RATCHET_BOMB,
    &SCAVENGING_OOZE,
    &SHADOWBORN_DEMON,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
