//! Portal Second Age cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::DiscardSelectionDef;
use crate::card::abilities;
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef, AppliedRuleDef,
    CardArt, CardRules, CardSet, CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef,
    ObjectQueryDef, PlayerRelation, ResolvedEffectDurationDef, SacrificedAmountDef, ValueDef,
    ZoneKind, ZonePlacement,
};
use crate::{TargetIndex, mana_cost};

// P02 8 — Angel of Mercy
pub(in crate::card::sets) static ANGEL_OF_MERCY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("dac5c913-4eb5-4cfb-9c24-223f14f07064"),
    "Angel of Mercy",
    CardArt::new("dac5c913-4eb5-4cfb-9c24-223f14f07064", "Melissa A. Benson"),
    CardSet::PortalSecondAge,
    CardRules::new_creature(mana_cost!("{4}{W}"), &["Angel"], 3, 3).with_abilities(&[
        abilities::flying(),
        abilities::enters_trigger(
            "When this creature enters, you gain 3 life.",
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(3),
            },
        ),
    ]),
);

// P02 10 — Angelic Wall
pub(in crate::card::sets) static ANGELIC_WALL: CardRecord = CardRecord::new_with_legacy_id(
    751,
    "Angelic Wall",
    CardArt::new("d7b2450d-87a7-46dc-b43a-2db2abeca44f", "Allen Williams"),
    CardSet::PortalSecondAge,
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Wall"], 0, 4)
        .with_abilities(&[abilities::defender(), abilities::flying()]),
);

// P02 15 — Breath of Life (reprint)
const BREATH_OF_LIFE_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y1997::portal::BREATH_OF_LIFE)
        .with_art("a10f24f7-f82e-413e-824f-384607c7d858", "Lubov");

// P02 18 — Path of Peace (reprint)
const PATH_OF_PEACE_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y1997::portal::PATH_OF_PEACE)
        .with_art("cb14d3f4-09f3-4113-bdc3-0fd753137f7c", "David A. Cherry");

// P02 20 — Righteous Charge
pub(in crate::card::sets) static RIGHTEOUS_CHARGE: CardRecord = CardRecord::new_with_legacy_id(
    1064,
    "Righteous Charge",
    CardArt::new("f52cb325-4f16-4cf3-9999-feafe0fde8c2", "Svetlin Velinov"),
    CardSet::PortalSecondAge,
    CardRules::new_sorcery(mana_cost!("{1}{W}{W}")).with_ability(AbilityDef::spell(
        "Creatures you control get +2/+2 until end of turn.",
        EffectDef::Apply {
            recipient: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::HasType(CardType::Creature),
                &[ZoneKind::Battlefield],
                PlayerRelation::You,
            ),
            effect: AppliedEffectDef::modify_power_toughness(
                ValueDef::Constant(2),
                ValueDef::Constant(2),
            ),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    )),
);

// P02 27 — Vengeance (reprint)
const VENGEANCE_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y1997::portal::VENGEANCE)
        .with_art("3209ee48-4485-44fc-b71d-cd6241674e64", "Keith Parkinson");

// P02 37 — Exhaustion (reprint)
const EXHAUSTION_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y1997::portal::EXHAUSTION)
        .with_art("fcc103a6-7888-4e35-b35b-a796a48caf70", "Kaja Foglio");

// P02 46 — Sleight of Hand
pub(in crate::card::sets) static SLEIGHT_OF_HAND: CardRecord = CardRecord::new_with_legacy_id(
    311,
    "Sleight of Hand",
    CardArt::new("f3405184-dcda-4bb6-ade6-c2a87bc3296d", "Phil Foglio"),
    CardSet::PortalSecondAge,
    CardRules::new_sorcery(mana_cost!("{U}")).with_ability(AbilityDef::spell(
        "Look at the top two cards of your library. Put one of them into your hand and the other on the bottom of your library.",
        abilities::look_at_top_cards_choose_to_hand_rest_bottom(
            ValueDef::Constant(2),
            ObjectPredicateDef::Any,
            1,
            1,
        ),
    )),
);

// P02 87 — Ravenous Rats
pub(in crate::card::sets) static RAVENOUS_RATS: CardRecord = CardRecord::new_with_legacy_id(
    1006,
    "Ravenous Rats",
    CardArt::new("0642111c-f668-4acb-9df5-f0b920352407", "Carl Critchlow"),
    CardSet::PortalSecondAge,
    CardRules::new_creature(mana_cost!("{1}{B}"), &["Rat"], 1, 1).with_ability(
        abilities::enters_trigger_with_targets(
            "When this creature enters, target opponent discards a card.",
            &[AbilityTargetDef::exactly_one(
                AbilityTargetPredicate::Player(PlayerRelation::Opponent),
            )],
            EffectDef::Discard {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(1),
                selection: DiscardSelectionDef::RecipientChooses,
                then: None,
            },
        ),
    ),
);

// P02 91 — Blaze (reprint)
const BLAZE_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y1997::portal::BLAZE)
        .with_art("3940d0ca-0ca2-4446-9330-a554c3e89824", "David A. Cherry");

// P02 98 — Goblin Glider
pub(in crate::card::sets) static GOBLIN_GLIDER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("9c29491b-dec1-429d-9950-062582f8164f"),
    "Goblin Glider",
    CardArt::new("9c29491b-dec1-429d-9950-062582f8164f", "Pete Venters"),
    CardSet::PortalSecondAge,
    CardRules::new_creature(mana_cost!("{1}{R}"), &["Goblin"], 1, 1).with_abilities(&[
        abilities::flying(),
        AbilityDef::static_ability(
            "This creature can't block.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::CANNOT_BLOCK),
            },
        ),
    ]),
);

// P02 100 — Goblin Matron
pub(in crate::card::sets) static GOBLIN_MATRON: CardRecord = CardRecord::new_with_legacy_id(
    2018,
    "Goblin Matron",
    CardArt::new("9e9e2e5d-ad06-4378-9afb-ffb174e6a5b4", "DiTerlizzi"),
    CardSet::PortalSecondAge,
    // Any Goblin card, so it fetches the answer rather than the biggest
    // body: Tinkerer against artifacts, Ringleader for more cards.
    CardRules::new_creature(mana_cost!("{2}{R}"), &["Goblin"], 1, 1).with_ability(
        abilities::enters_trigger("When this creature enters, you may search your library for a Goblin card, reveal that card, put it into your hand, then shuffle.", EffectDef::SearchZone {
                player: EffectRecipientDef::Controller,
                source: ZoneKind::Library,
                object: ObjectPredicateDef::Subtype("Goblin"),
                minimum: 0,
                maximum: ValueDef::Constant(1),
                reveal: true,
                destination: ZoneKind::Hand,
                placement: ZonePlacement::Top,
                shuffle: true,
                enters_tapped: false,
                attachment: None,
                binding: None,
                then: None,
            }),
    ),
);

// P02 102 — Goblin Piker
pub(in crate::card::sets) static GOBLIN_PIKER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("2786834d-dbda-40ce-82a4-e518cd554312"),
    "Goblin Piker",
    crate::card::CardArt::new("083ec3e7-950c-4e9d-aba5-02ed13d723f0", "DiTerlizzi"),
    crate::card::CardSet::PortalSecondAge,
    CardRules::new_creature(mana_cost!("{1}{R}"), &["Goblin", "Warrior"], 2, 1),
);

// P02 103 — Goblin Raider
pub(in crate::card::sets) static GOBLIN_RAIDER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("68fe9691-d788-42cb-8d13-005724939b62"),
    "Goblin Raider",
    CardArt::new("68fe9691-d788-42cb-8d13-005724939b62", "Matt Stawicki"),
    CardSet::PortalSecondAge,
    CardRules::new_creature(mana_cost!("{1}{R}"), &["Goblin", "Warrior"], 2, 2).with_ability(
        AbilityDef::static_ability(
            "This creature can't block.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::CANNOT_BLOCK),
            },
        ),
    ),
);

// P02 105 — Goblin War Strike
pub(in crate::card::sets) static GOBLIN_WAR_STRIKE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("738fecfd-1119-4dcb-acd6-ec9715d9c074"),
    "Goblin War Strike",
    CardArt::new("738fecfd-1119-4dcb-acd6-ec9715d9c074", "Michael Weaver"),
    CardSet::PortalSecondAge,
    CardRules::new_sorcery(mana_cost!("{R}")).with_ability(AbilityDef::spell_with_targets(
        "Goblin War Strike deals damage to target player or planeswalker equal to the number of Goblins you control.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::PlayerOrPlaneswalker(PlayerRelation::Any),
        )],
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::CountMatchingObjects(&ObjectQueryDef::matching(
                ObjectPredicateDef::Subtype("Goblin"),
                &[ZoneKind::Battlefield],
                PlayerRelation::You,
            )),
        },
    )),
);

// P02 106 — Jagged Lightning
pub(in crate::card::sets) static JAGGED_LIGHTNING: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("148e6704-9cf0-45cf-9bab-db318c016593"),
    "Jagged Lightning",
    CardArt::new("148e6704-9cf0-45cf-9bab-db318c016593", "Michael Weaver"),
    CardSet::PortalSecondAge,
    CardRules::new_sorcery(mana_cost!("{3}{R}{R}")).with_ability(AbilityDef::spell_with_targets(
        "Jagged Lightning deals 3 damage to each of two target creatures.",
        &[AbilityTargetDef::exactly_value(
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::HasType(CardType::Creature),
                zones: &[ZoneKind::Battlefield],
                controller: None,
                owner: None,
            },
            ValueDef::Constant(2),
        )],
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(3),
        },
    )),
);

// P02 112 — Ogre Taskmaster
pub(in crate::card::sets) static OGRE_TASKMASTER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("d674a92e-b268-48f7-b082-f8ca2e63d43b"),
    "Ogre Taskmaster",
    CardArt::new("d674a92e-b268-48f7-b082-f8ca2e63d43b", "Dan Frazier"),
    CardSet::PortalSecondAge,
    CardRules::new_creature(mana_cost!("{3}{R}"), &["Ogre"], 4, 3).with_ability(
        AbilityDef::static_ability(
            "This creature can't block.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::CANNOT_BLOCK),
            },
        ),
    ),
);

// P02 119 — Volcanic Hammer (reprint)
const VOLCANIC_HAMMER_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y1997::portal::VOLCANIC_HAMMER).with_art(
        "58c0489d-b073-4ad4-b044-447fcc865b6c",
        "Edward P. Beard, Jr.",
    );

// P02 120 — Wildfire
pub(in crate::card::sets) static WILDFIRE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("b69cfcb0-db68-4494-a3e1-7c2ca279fcf5"),
    "Wildfire",
    CardArt::new("b69cfcb0-db68-4494-a3e1-7c2ca279fcf5", "Rob Alexander"),
    CardSet::PortalSecondAge,
    CardRules::new_sorcery(mana_cost!("{4}{R}{R}")).with_ability(AbilityDef::spell(
        "Each player sacrifices four lands of their choice. Wildfire deals 4 damage to each creature.",
        EffectDef::Sequence(&[
            EffectDef::SacrificeOfChoice {
                player: EffectRecipientDef::EachPlayer,
                object: ObjectPredicateDef::HasType(CardType::Land),
                count: ValueDef::Constant(4),
                then: None,
                amount: SacrificedAmountDef::Power,
                otherwise: None,
                optional: false,
            },
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::matching_objects(
                    ObjectPredicateDef::HasType(CardType::Creature),
                    &[ZoneKind::Battlefield],
                    PlayerRelation::Any,
                ),
                amount: ValueDef::Constant(4),
            },
        ]),
    )),
);

// P02 131 — Lone Wolf
// Audit: unsupported — Needs a combat-damage assignment option that lets the attacker assign damage as though it were unblocked without actually becoming unblocked.
pub(in crate::card::sets) static LONE_WOLF: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("7ff4d831-7388-4321-a636-79cf7bde25bb"),
    "Lone Wolf",
    crate::card::CardArt::new("7ff4d831-7388-4321-a636-79cf7bde25bb", "Michael Weaver"),
    crate::card::CardSet::PortalSecondAge,
    crate::card::CardRules::unsupported(),
);

// P02 133 — Monstrous Growth (reprint)
const MONSTROUS_GROWTH_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y1997::portal::MONSTROUS_GROWTH)
        .with_art("3816da20-4434-4bf7-a9dd-3eb3bb735f08", "Una Fricker");

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &ANGEL_OF_MERCY,
    &ANGELIC_WALL,
    &RIGHTEOUS_CHARGE,
    &SLEIGHT_OF_HAND,
    &RAVENOUS_RATS,
    &GOBLIN_GLIDER,
    &GOBLIN_MATRON,
    &GOBLIN_PIKER,
    &GOBLIN_RAIDER,
    &GOBLIN_WAR_STRIKE,
    &JAGGED_LIGHTNING,
    &OGRE_TASKMASTER,
    &WILDFIRE,
    &LONE_WOLF,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[
    BREATH_OF_LIFE_REPRINT,
    PATH_OF_PEACE_REPRINT,
    VENGEANCE_REPRINT,
    EXHAUSTION_REPRINT,
    BLAZE_REPRINT,
    VOLCANIC_HAMMER_REPRINT,
    MONSTROUS_GROWTH_REPRINT,
];
