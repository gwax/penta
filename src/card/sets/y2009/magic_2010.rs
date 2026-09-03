//! Magic 2010 card records.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::AbilityCostDef;
use crate::AbilityDef;
use crate::AbilityTargetDef;
use crate::AbilityTargetPredicate;
use crate::AddManaEffectDef;
use crate::AppliedEffectDef;
use crate::BasicLandType;
use crate::CardArt;
use crate::CardRules;
use crate::CardSet;
use crate::CardSupertype;
use crate::CardType;
use crate::ColorSet;
use crate::ControlDurationDef;
use crate::CreatureTypeSetDef;
use crate::DamageEventMatcherDef;
use crate::DamagePreventionDef;
use crate::EffectDef;
use crate::EffectRecipientDef;
use crate::ManaColor;
use crate::ObjectPredicateDef;
use crate::ObjectQueryDef;
use crate::PlayerRefDef;
use crate::PlayerRelation;
use crate::ResolvedEffectDurationDef;
use crate::TargetIndex;
use crate::TriggerEventDef;
use crate::ValueDef;
use crate::ZoneKind;
use crate::ZonePlacement;
use crate::card::abilities;

use crate::mana_cost;

// M10 2 — Angel's Mercy
pub(in crate::card::sets) static ANGELS_MERCY: CardRecord = CardRecord::new_with_legacy_id(
    750,
    "Angel's Mercy",
    CardArt::new("7a437999-26ae-49fa-8647-c8c2b4640702", "Greg Staples"),
    CardSet::Magic2010,
    CardRules::new_instant(mana_cost!("{2}{W}{W}")).with_ability(AbilityDef::spell(
        "You gain 7 life.",
        EffectDef::GainLife {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(7),
        },
    )),
);

// M10 6 — Captain of the Watch
pub(in crate::card::sets) static CAPTAIN_OF_THE_WATCH: CardRecord = CardRecord::new_with_legacy_id(
    968,
    "Captain of the Watch",
    CardArt::new("8e3c18f5-89cd-4d33-8d5b-12dacad9f9b3", "Greg Staples"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{4}{W}{W}"), &["Human", "Soldier"], 3, 3).with_abilities(
        &[
            abilities::vigilance(),
            AbilityDef::static_ability(
                "Other Soldier creatures you control get +1/+1 and have vigilance.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::matching_objects(
                        ObjectPredicateDef::All(&[
                            ObjectPredicateDef::HasType(CardType::Creature),
                            ObjectPredicateDef::Subtype("Soldier"),
                            ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
                        ]),
                        &[ZoneKind::Battlefield],
                        PlayerRelation::You,
                    ),
                    effect: AppliedEffectDef::Composite(&[
                        AppliedEffectDef::modify_power_toughness(
                            ValueDef::Constant(1),
                            ValueDef::Constant(1),
                        ),
                        AppliedEffectDef::add_ability(&abilities::vigilance()),
                    ]),
                },
            ),
            abilities::enters_trigger(
                "When this creature enters, create three 1/1 white Soldier creature tokens.",
                EffectDef::create_creature_token(&["Soldier"], &[ManaColor::White], 1, 1)
                    .with_art(CardArt::new(
                        "86272c08-c5f2-413f-87ea-b135aca2d9c5",
                        "Greg Staples",
                    ))
                    .with_amount(3),
            ),
        ],
    ),
);

// M10 8 — Divine Verdict
pub(in crate::card::sets) static DIVINE_VERDICT: CardRecord = CardRecord::new_with_legacy_id(
    972,
    "Divine Verdict",
    CardArt::new("cc52c269-d44f-449c-af59-4c425aa10bbf", "Kev Walker"),
    CardSet::Magic2010,
    CardRules::new_instant(mana_cost!("{3}{W}")).with_ability(AbilityDef::destroy_target(
        "Destroy target attacking or blocking creature.",
        &AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::All(&[
            ObjectPredicateDef::HasType(CardType::Creature),
            ObjectPredicateDef::AttackingOrBlocking,
        ])),
        true,
    )),
);

// M10 9 — Elite Vanguard
pub(in crate::card::sets) static ELITE_VANGUARD: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("6bda0b4b-ab5a-4d91-9dd1-7a5a145b67f5"),
    "Elite Vanguard",
    crate::card::CardArt::new("f03487e9-f584-4bbd-8335-4dd001a88b52", "Mark Tedin"),
    crate::card::CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{W}"), &["Human", "Soldier"], 2, 1),
);

// M10 11 — Glorious Charge
pub(in crate::card::sets) static GLORIOUS_CHARGE: CardRecord = CardRecord::new_with_legacy_id(
    974,
    "Glorious Charge",
    CardArt::new("f8672cfd-e34b-4587-9e24-015e03c7574d", "Izzy"),
    CardSet::Magic2010,
    CardRules::new_instant(mana_cost!("{1}{W}")).with_ability(AbilityDef::spell(
        "Creatures you control get +1/+1 until end of turn.",
        EffectDef::Apply {
            recipient: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::HasType(CardType::Creature),
                &[ZoneKind::Battlefield],
                PlayerRelation::You,
            ),
            effect: AppliedEffectDef::modify_power_toughness(
                ValueDef::Constant(1),
                ValueDef::Constant(1),
            ),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    )),
);

// M10 12 — Griffin Sentinel
pub(in crate::card::sets) static GRIFFIN_SENTINEL: CardRecord = CardRecord::new_with_legacy_id(
    1150,
    "Griffin Sentinel",
    CardArt::new("b40d6626-a85f-4116-9721-19e39b83cba0", "Warren Mahy"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{2}{W}"), &["Griffin"], 1, 3)
        .with_abilities(&[abilities::flying(), abilities::vigilance()]),
);

// M10 16 — Honor of the Pure
pub(in crate::card::sets) static HONOR_OF_THE_PURE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("e09a2f0a-333a-4114-8b9b-f0011628cb90"),
    "Honor of the Pure",
    crate::card::CardArt::new("650a6831-c352-4ca7-9f8f-43ea99a1cf33", "Greg Staples"),
    crate::card::CardSet::Magic2010,
    CardRules::new_enchantment(mana_cost!("{1}{W}")).with_ability(AbilityDef::static_ability(
        "White creatures you control get +1/+1.",
        EffectDef::StaticApply {
            recipient: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Creature),
                    ObjectPredicateDef::Color(ManaColor::White),
                ]),
                &[ZoneKind::Battlefield],
                PlayerRelation::You,
            ),
            effect: AppliedEffectDef::modify_power_toughness(
                ValueDef::Constant(1),
                ValueDef::Constant(1),
            ),
        },
    )),
);

// M10 17 — Indestructibility
pub(in crate::card::sets) static INDESTRUCTIBILITY: CardRecord = CardRecord::new_with_legacy_id(
    1153,
    "Indestructibility",
    CardArt::new("e086a062-d39b-4e2a-bde0-f4d6d1797a5f", "Darrell Riche"),
    CardSet::Magic2010,
    CardRules::new_enchantment(mana_cost!("{3}{W}"))
        .with_subtypes(&["Aura"])
        .with_abilities(&[
            AbilityDef::spell_with_targets(
                "Enchant permanent",
                &[AbilityTargetDef::exactly_one_permanent(
                    ObjectPredicateDef::Any,
                )],
                EffectDef::Attach {
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                },
            ),
            AbilityDef::static_ability(
                "Enchanted permanent has indestructible.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::add_ability(&abilities::indestructible()),
                },
            ),
        ]),
);

// M10 18 — Lifelink
pub(in crate::card::sets) static LIFELINK: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f0d881c1-24e7-4ce7-8ab1-474cb040ddd7"),
    "Lifelink",
    crate::card::CardArt::new("a8e207d4-9930-4aff-a7c8-b53bd1b5d566", "Terese Nielsen"),
    crate::card::CardSet::Magic2010,
    CardRules::new_enchantment(mana_cost!("{W}"))
        .with_subtypes(&["Aura"])
        .with_abilities(&[
            abilities::enchant_creature(),
            AbilityDef::static_ability(
                "Enchanted creature has lifelink.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::add_ability(&abilities::lifelink()),
                },
            ),
        ]),
);

// M10 24 — Planar Cleansing
pub(in crate::card::sets) static PLANAR_CLEANSING: CardRecord = CardRecord::new_with_legacy_id(
    979,
    "Planar Cleansing",
    CardArt::new("b5047b71-2359-4d9a-a168-a8eec43c5f1b", "Michael Komarck"),
    CardSet::Magic2010,
    CardRules::new_sorcery(mana_cost!("{3}{W}{W}{W}")).with_ability(AbilityDef::spell(
        "Destroy all nonland permanents.",
        EffectDef::Destroy {
            object: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Land)),
                &[ZoneKind::Battlefield],
                PlayerRelation::Any,
            ),
            can_regenerate: true,
            then: None,
        },
    )),
);

// M10 28 — Safe Passage
pub(in crate::card::sets) static SAFE_PASSAGE: CardRecord = CardRecord::new_with_legacy_id(
    1498,
    "Safe Passage",
    CardArt::new(
        "9fc65c3f-ad29-4368-bf45-8345a7ec6f31",
        "Christopher Moeller",
    ),
    CardSet::Magic2010,
    CardRules::new_instant(mana_cost!("{2}{W}")).with_ability(AbilityDef::spell(
        "Prevent all damage that would be dealt to you and creatures you control this turn.",
        EffectDef::PreventDamage {
            prevention: DamagePreventionDef::unlimited(
                DamageEventMatcherDef::to_player_and_creatures_controlled_by(
                    PlayerRefDef::EffectController,
                ),
            ),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    )),
);

// M10 30 — Siege Mastodon
pub(in crate::card::sets) static SIEGE_MASTODON: CardRecord = CardRecord::new_with_legacy_id(
    1155,
    "Siege Mastodon",
    CardArt::new("40e7a30f-bb29-4c6b-bf70-53e9e4292814", "Matt Cavotta"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{4}{W}"), &["Elephant"], 3, 5),
);

// M10 31 — Silence
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SILENCE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("1559d660-8a9d-422b-95d3-710a046583dd"),
    "Silence",
    crate::card::CardArt::new("37b70d17-e4ec-4731-8892-b444f82be7a2", "Wayne Reynolds"),
    crate::card::CardSet::Magic2010,
    crate::card::CardRules::unsupported(),
);

// M10 32 — Silvercoat Lion
pub(in crate::card::sets) static SILVERCOAT_LION: CardRecord = CardRecord::new_with_legacy_id(
    982,
    "Silvercoat Lion",
    CardArt::new("9d33e866-cfd8-44e6-8070-df8df1ce965d", "Terese Nielsen"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Cat"], 2, 2),
);

// M10 33 — Solemn Offering
pub(in crate::card::sets) static SOLEMN_OFFERING: CardRecord = CardRecord::new_with_legacy_id(
    1156,
    "Solemn Offering",
    CardArt::new("9ca09fed-f9b3-49ee-be89-404581a4cbd2", "Sam Wood"),
    CardSet::Magic2010,
    CardRules::new_sorcery(mana_cost!("{2}{W}")).with_ability(AbilityDef::spell_with_targets(
        "Destroy target artifact or enchantment. You gain 4 life.",
        &[AbilityTargetDef::exactly_one_permanent(
            ObjectPredicateDef::AnyOf(&[
                ObjectPredicateDef::HasType(CardType::Artifact),
                ObjectPredicateDef::HasType(CardType::Enchantment),
            ]),
        )],
        EffectDef::Sequence(&[
            EffectDef::Destroy {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                can_regenerate: true,
                then: None,
            },
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(4),
            },
        ]),
    )),
);

// M10 35 — Stormfront Pegasus
pub(in crate::card::sets) static STORMFRONT_PEGASUS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("d2429a15-ccbe-463c-9218-968709d9e878"),
    "Stormfront Pegasus",
    crate::card::CardArt::new("bf0ba2d2-09d5-4755-a18f-40cf19d88f25", "rk post"),
    crate::card::CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Pegasus"], 2, 1)
        .with_abilities(&[abilities::flying()]),
);

// M10 43 — Alluring Siren
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static ALLURING_SIREN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("df4e1cc3-4e47-4eff-9047-c6d1cc84d635"),
    "Alluring Siren",
    crate::card::CardArt::new("a6434841-6cca-4397-b1fa-5ce34dc0b7f3", "Chippy"),
    crate::card::CardSet::Magic2010,
    crate::card::CardRules::unsupported(),
);

// M10 49 — Divination
pub(in crate::card::sets) static DIVINATION: CardRecord = CardRecord::new_with_legacy_id(
    696,
    "Divination",
    CardArt::new("4a1340f1-85a4-4551-9871-bb00db6d97a8", "Scott Chou"),
    CardSet::Magic2010,
    CardRules::new_sorcery(mana_cost!("{2}{U}")).with_ability(AbilityDef::spell(
        "Draw two cards.",
        abilities::draw_cards(ValueDef::Constant(2)),
    )),
);

// M10 50 — Djinn of Wishes
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static DJINN_OF_WISHES: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("3e3b0949-17e1-4f12-8999-d4638d32dd3e"),
    "Djinn of Wishes",
    crate::card::CardArt::new("74c621dd-9c60-4951-beaf-eb6b597c2f0f", "Kev Walker"),
    crate::card::CardSet::Magic2010,
    crate::card::CardRules::unsupported(),
);

// M10 51 — Essence Scatter
pub(in crate::card::sets) static ESSENCE_SCATTER: CardRecord = CardRecord::new_with_legacy_id(
    162,
    "Essence Scatter",
    CardArt::new("fcd965f9-bdaa-4434-a9c8-53fc57e997db", "Jon Foster"),
    CardSet::Magic2010,
    CardRules::new_instant(mana_cost!("{1}{U}")).with_ability(AbilityDef::counter_target(
        "Counter target creature spell.",
        &AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::All(&[
                ObjectPredicateDef::Spell,
                ObjectPredicateDef::HasType(CardType::Creature),
            ]),
            zones: &[ZoneKind::Stack],
            controller: None,
            owner: None,
        }),
    )),
);

// M10 56 — Ice Cage
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static ICE_CAGE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("4d18c4d7-c779-473b-9b41-f22b439bb501"),
    "Ice Cage",
    crate::card::CardArt::new("a5e14b62-c050-4d43-aeee-873f46d1e295", "Mike Bierek"),
    crate::card::CardSet::Magic2010,
    crate::card::CardRules::unsupported(),
);

// M10 63 — Mind Control
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static MIND_CONTROL: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("37151305-e489-4df1-9b0a-c5e11c77d2f1"),
    "Mind Control",
    crate::card::CardArt::new("ec7f77af-17d7-4746-bc83-f455b9b6f9ea", "Ryan Pancoast"),
    crate::card::CardSet::Magic2010,
    crate::card::CardRules::unsupported(),
);

// M10 71 — Sleep
/// Both clauses reach the same set, so the skip lands on exactly the
/// creatures the tap found rather than on whatever is tapped later.
static SLEEP_THEIR_CREATURES: EffectRecipientDef = EffectRecipientDef::objects_controlled_by_target(
    ObjectPredicateDef::HasType(CardType::Creature),
    TargetIndex::PRIMARY,
);

pub(in crate::card::sets) static SLEEP: CardRecord = CardRecord::new_with_legacy_id(
    1860,
    "Sleep",
    CardArt::new("1e352497-1454-4917-b38c-4cc45424d876", "Chris Rahn"),
    CardSet::Magic2010,
    CardRules::new_sorcery(mana_cost!("{2}{U}{U}")).with_ability(AbilityDef::spell_with_targets(
        "Tap all creatures target player controls. Those creatures don't untap during that \
         player's next untap step.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Player(PlayerRelation::Any),
        )],
        EffectDef::Sequence(&[
            EffectDef::Tap {
                object: SLEEP_THEIR_CREATURES,
            },
            EffectDef::SkipNextUntapSteps {
                object: SLEEP_THEIR_CREATURES,
                count: 1,
            },
        ]),
    )),
);

// M10 76 — Tome Scour
pub(in crate::card::sets) static TOME_SCOUR: CardRecord = CardRecord::new_with_legacy_id(
    1172,
    "Tome Scour",
    CardArt::new("aed4cfec-5cea-4987-890e-825b2802e9f9", "Steven Belledin"),
    CardSet::Magic2010,
    CardRules::new_sorcery(mana_cost!("{U}")).with_ability(AbilityDef::spell_with_targets(
        "Target player mills five cards.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Player(PlayerRelation::Any),
        )],
        EffectDef::Mill {
            player: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(5),
        },
    )),
);

// M10 80 — Wall of Frost
pub(in crate::card::sets) static WALL_OF_FROST: CardRecord = CardRecord::new_with_legacy_id(
    1862,
    "Wall of Frost",
    CardArt::new("d4000b46-7843-4c07-8332-a10f207e2cdc", "Mike Bierek"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{1}{U}{U}"), &["Wall"], 0, 7).with_abilities(&[
        abilities::defender(),
        // The blocked creature is the trigger's own object, so the skip
        // lands on it rather than on whatever else is in the combat.
        AbilityDef::triggered(
            "Whenever this creature blocks a creature, that creature doesn't untap during its \
             controller's next untap step.",
            TriggerEventDef::Blocks {
                blocked: ObjectPredicateDef::Any,
            },
            EffectDef::SkipNextUntapSteps {
                object: EffectRecipientDef::TriggeringObject,
                count: 1,
            },
        ),
    ]),
);

// M10 87 — Cemetery Reaper
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static CEMETERY_REAPER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("639b48f0-3426-46cf-b857-4611f7de4826"),
    "Cemetery Reaper",
    crate::card::CardArt::new("56494d1e-0d7e-4c29-942c-b376ff07cdf8", "Dave Allsop"),
    crate::card::CardSet::Magic2010,
    crate::card::CardRules::unsupported(),
);

// M10 88 — Child of Night
pub(in crate::card::sets) static CHILD_OF_NIGHT: CardRecord = CardRecord::new_with_legacy_id(
    1180,
    "Child of Night",
    CardArt::new("c21b5476-5f5f-46b5-b627-398e9fcd04aa", "Ash Wood"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{1}{B}"), &["Vampire"], 2, 1)
        .with_abilities(&[abilities::lifelink()]),
);

// M10 92 — Disentomb
pub(in crate::card::sets) static DISENTOMB: CardRecord = CardRecord::new_with_legacy_id(
    1000,
    "Disentomb",
    CardArt::new(
        "ce7473bb-d092-4d76-b3c3-5036222dbdf7",
        "Alex Horley-Orlandelli",
    ),
    CardSet::Magic2010,
    CardRules::new_sorcery(mana_cost!("{B}")).with_ability(AbilityDef::spell_with_targets(
        "Return target creature card from your graveyard to your hand.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::HasType(CardType::Creature),
                zones: &[ZoneKind::Graveyard],
                controller: None,
                owner: Some(PlayerRelation::You),
            },
        )],
        EffectDef::MoveToZone {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            zone: ZoneKind::Hand,
            placement: ZonePlacement::Top,
        },
    )),
);

// M10 93 — Doom Blade
pub(in crate::card::sets) static DOOM_BLADE: CardRecord = CardRecord::new_with_legacy_id(
    158,
    "Doom Blade",
    CardArt::new("75d96a37-bdbe-46ae-926f-8742699a0b20", "Chippy"),
    CardSet::Magic2010,
    CardRules::new_instant(mana_cost!("{1}{B}")).with_ability(AbilityDef::destroy_target(
        "Destroy target nonblack creature.",
        &AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::All(&[
            ObjectPredicateDef::HasType(CardType::Creature),
            ObjectPredicateDef::Not(&ObjectPredicateDef::Color(ManaColor::Black)),
        ])),
        true,
    )),
);

// M10 109 — Rise from the Grave
pub(in crate::card::sets) static RISE_FROM_THE_GRAVE: CardRecord = CardRecord::new_with_legacy_id(
    2002,
    "Rise from the Grave",
    CardArt::new("5d2b187e-c489-4652-a638-390fc9ecef0e", "Vance Kovacs"),
    CardSet::Magic2010,
    // Any graveyard, so it steals as readily as it recurs.
    CardRules::new_sorcery(mana_cost!("{4}{B}")).with_ability(AbilityDef::spell_with_targets(
        "Put target creature card from a graveyard onto the battlefield under your control. That creature is a black Zombie in addition to its other colors and types.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::HasType(CardType::Creature),
                zones: &[ZoneKind::Graveyard],
                controller: None,
                owner: None,
            },
        )],
        EffectDef::WithZoneMoveResult {
            effect: &EffectDef::WithBattlefieldArrival {
                effect: &EffectDef::MoveToZone {
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    zone: ZoneKind::Battlefield,
                    placement: ZonePlacement::Top,
                },
                arrival: crate::card::BattlefieldArrivalDef {
                    controller: Some(PlayerRelation::You),
                    ..crate::card::BattlefieldArrivalDef::DEFAULT
                },
            },
            binding: crate::ParentBinding,
            then: &EffectDef::Apply {
                recipient: EffectRecipientDef::binding_zone_change_successors(
                    crate::ParentBinding,
                ),
                // "In addition to its other colors and types", so both leaves add rather
                // than set.
                effect: AppliedEffectDef::Composite(&[
                    AppliedEffectDef::add_colors(ColorSet::from_colors(&[ManaColor::Black])),
                    AppliedEffectDef::add_creature_types(CreatureTypeSetDef::named(&["Zombie"])),
                ]),
                duration: ResolvedEffectDurationDef::Permanent,
            },
        },
    )),
);

// M10 111 — Sanguine Bond
pub(in crate::card::sets) static SANGUINE_BOND: CardRecord = CardRecord::new_with_legacy_id(
    1190,
    "Sanguine Bond",
    CardArt::new("e50e807d-b2eb-4b62-8663-8ad17eed2a39", "Jaime Jones"),
    CardSet::Magic2010,
    CardRules::new_enchantment(mana_cost!("{3}{B}{B}")).with_ability(
        AbilityDef::triggered_with_targets(
            "Whenever you gain life, target opponent loses that much life.",
            TriggerEventDef::LifeGained(PlayerRelation::You),
            &[AbilityTargetDef::exactly_one(
                AbilityTargetPredicate::Player(PlayerRelation::Opponent),
            )],
            EffectDef::LoseLife {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::TriggerEventAmount,
            },
        ),
    ),
);

// M10 112 — Sign in Blood
pub(in crate::card::sets) static SIGN_IN_BLOOD: CardRecord = CardRecord::new_with_legacy_id(
    213,
    "Sign in Blood",
    CardArt::new("64f6600b-36c4-43bd-8c01-cfbca402ecd6", "Howard Lyon"),
    CardSet::Magic2010,
    CardRules::new_sorcery(mana_cost!("{B}{B}")).with_abilities(&[AbilityDef::spell_with_targets(
        "Target player draws two cards and loses 2 life.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Player(PlayerRelation::Any),
        )],
        EffectDef::Sequence(&[
            EffectDef::DrawCards {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(2),
            },
            EffectDef::LoseLife {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(2),
            },
        ]),
    )]),
);

// M10 118 — Vampire Nocturnus
// Audit: unsupported — PlaysWithTopOfLibraryRevealed exists, but static conditions cannot inspect the top card's color for the Vampire mass bonus and flying grant.
pub(in crate::card::sets) static VAMPIRE_NOCTURNUS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("3194ae81-90fb-49e9-90de-9d161e296770"),
    "Vampire Nocturnus",
    crate::card::CardArt::new("8daccbbb-6600-4467-810f-277f01a11771", "Raymond Swanland"),
    crate::card::CardSet::Magic2010,
    crate::card::CardRules::unsupported(),
);

// M10 120 — Warpath Ghoul
pub(in crate::card::sets) static WARPATH_GHOUL: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("2c6cc262-ba0c-4cca-ae9c-24a1824753e4"),
    "Warpath Ghoul",
    crate::card::CardArt::new("94785274-fa79-47cc-9896-0f5f695abb21", "rk post"),
    crate::card::CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{2}{B}"), &["Zombie"], 3, 2),
);

// M10 123 — Zombie Goliath
pub(in crate::card::sets) static ZOMBIE_GOLIATH: CardRecord = CardRecord::new_with_legacy_id(
    1008,
    "Zombie Goliath",
    CardArt::new("8638edec-ddcd-4f50-9c2f-2e1668e3d175", "E. M. Gist"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{4}{B}"), &["Zombie", "Giant"], 4, 3),
);

// M10 124 — Act of Treason
pub(in crate::card::sets) static ACT_OF_TREASON: CardRecord = CardRecord::new_with_legacy_id(
    1084,
    "Act of Treason",
    CardArt::new("a04c8c6f-14e9-427c-918e-208ccd39ec4a", "Matt Stewart"),
    CardSet::Magic2010,
    CardRules::new_sorcery(mana_cost!("{2}{R}")).with_ability(
        AbilityDef::spell_with_targets(
            "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn.",
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::HasType(CardType::Creature),
            )],
            EffectDef::Sequence(&[
                EffectDef::GainControl {
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    duration: ControlDurationDef::UntilEndOfTurn,
                    controller: PlayerRefDef::EffectController,
                },
                EffectDef::Untap {
                    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                },
                EffectDef::Apply {
                    recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    effect: AppliedEffectDef::add_ability(&abilities::haste()),
                    duration: ResolvedEffectDurationDef::UntilEndOfTurn,
                },
            ]),
        ),
    ),
);

// M10 135 — Fiery Hellhound
pub(in crate::card::sets) static FIERY_HELLHOUND: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("6d6b2c8a-8019-4e4b-8f4e-058ab5284153"),
    "Fiery Hellhound",
    crate::card::CardArt::new("7c96f7a0-99a3-4ba4-b0f0-9ea36c45d5d5", "Ted Galaday"),
    crate::card::CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{1}{R}{R}"), &["Elemental", "Dog"], 2, 2).with_ability(
        AbilityDef::activated(
            "{R}: This creature gets +1/+0 until end of turn.",
            &[AbilityCostDef::Mana(mana_cost!("{R}"))],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(1),
                    ValueDef::Constant(0),
                ),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        ),
    ),
);

// M10 139 — Goblin Chieftain
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static GOBLIN_CHIEFTAIN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f5c8a4a4-1611-4188-9c59-8aefb016b5ad"),
    "Goblin Chieftain",
    crate::card::CardArt::new("2540ec6b-9ffa-4ab0-bbd3-ddf1efd2db60", "Sam Wood"),
    crate::card::CardSet::Magic2010,
    crate::card::CardRules::unsupported(),
);

// M10 165 — Acidic Slime
pub(in crate::card::sets) static ACIDIC_SLIME: CardRecord = CardRecord::new_with_legacy_id(
    1028,
    "Acidic Slime",
    CardArt::new("bd7bef5a-e0ab-46d3-a802-620bf2a7546f", "Karl Kopinski"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{3}{G}{G}"), &["Ooze"], 2, 2).with_abilities(&[
        abilities::deathtouch(),
        abilities::enters_trigger_with_targets(
            "When this creature enters, destroy target artifact, enchantment, or land.",
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::AnyOf(&[
                    ObjectPredicateDef::HasType(CardType::Artifact),
                    ObjectPredicateDef::HasType(CardType::Enchantment),
                    ObjectPredicateDef::HasType(CardType::Land),
                ]),
            )],
            EffectDef::Destroy {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                can_regenerate: true,
                then: None,
            },
        ),
    ]),
);

// M10 169 — Borderland Ranger
pub(in crate::card::sets) static BORDERLAND_RANGER: CardRecord = CardRecord::new_with_legacy_id(
    820,
    "Borderland Ranger",
    CardArt::new("8f067c26-c51d-44d0-a0af-106b5778f06a", "Zoltan Boros"),
    CardSet::Magic2010,
    CardRules::new_creature(
        mana_cost!("{2}{G}"),
        &["Human", "Scout", "Ranger"],
        2,
        2,
    )
    .with_ability(abilities::enters_trigger("When this creature enters, you may search your library for a basic land card, reveal it, put it into your hand, then shuffle.", EffectDef::May {
            player: EffectRecipientDef::Controller,
            effect: &EffectDef::SearchZone {
                player: EffectRecipientDef::Controller,
                source: ZoneKind::Library,
                object: ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Land),
                    ObjectPredicateDef::Supertype(CardSupertype::Basic),
                ]),
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
            },
        })),
);

// M10 170 — Bountiful Harvest
pub(in crate::card::sets) static BOUNTIFUL_HARVEST: CardRecord = CardRecord::new_with_legacy_id(
    1030,
    "Bountiful Harvest",
    CardArt::new("8d7a4494-2ced-4405-9204-d2617961a1d6", "Jason Chan"),
    CardSet::Magic2010,
    CardRules::new_sorcery(mana_cost!("{4}{G}")).with_ability(AbilityDef::spell(
        "You gain 1 life for each land you control.",
        EffectDef::GainLife {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::CountMatchingObjects(&ObjectQueryDef::matching(
                ObjectPredicateDef::HasType(CardType::Land),
                &[ZoneKind::Battlefield],
                PlayerRelation::You,
            )),
        },
    )),
);

// M10 172 — Centaur Courser
pub(in crate::card::sets) static CENTAUR_COURSER: CardRecord = CardRecord::new_with_legacy_id(
    1031,
    "Centaur Courser",
    CardArt::new("44a5f7db-ea4e-4af5-9d4a-0335db6ea0e9", "Vance Kovacs"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{2}{G}"), &["Centaur", "Warrior"], 3, 3),
);

// M10 174 — Cudgel Troll
pub(in crate::card::sets) static CUDGEL_TROLL: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("d779b14c-a100-4382-9e7c-0969efda73ec"),
    "Cudgel Troll",
    crate::card::CardArt::new("e156b8d8-5309-494e-9709-44f98826a69f", "Jesper Ejsing"),
    crate::card::CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{2}{G}{G}"), &["Troll"], 4, 3).with_ability(
        abilities::regenerate_self(
            "{G}: Regenerate this creature.",
            &[AbilityCostDef::Mana(mana_cost!("{G}"))],
        ),
    ),
);

// M10 175 — Deadly Recluse
pub(in crate::card::sets) static DEADLY_RECLUSE: CardRecord = CardRecord::new_with_legacy_id(
    1032,
    "Deadly Recluse",
    CardArt::new("a32a5f77-7c1f-4da4-9ae6-3947504a8dea", "Warren Mahy"),
    CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{1}{G}"), &["Spider"], 1, 2)
        .with_abilities(&[abilities::reach(), abilities::deathtouch()]),
);

// M10 176 — Elvish Archdruid
pub(in crate::card::sets) static ELVISH_ARCHDRUID: CardRecord = CardRecord::new_with_legacy_id(
    1872,
    "Elvish Archdruid",
    CardArt::new("bf8eba57-8c51-490b-995f-53eeb7ad574f", "Karl Kopinski"),
    CardSet::Magic2010,
    // The count includes the Archdruid itself, which is an Elf: a lone one
    // taps for a single green rather than none.
    CardRules::new_creature(mana_cost!("{1}{G}{G}"), &["Elf", "Druid"], 2, 2).with_abilities(&[
        AbilityDef::static_ability(
            "Other Elf creatures you control get +1/+1.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::matching_objects(
                    ObjectPredicateDef::All(&[
                        ObjectPredicateDef::HasType(CardType::Creature),
                        ObjectPredicateDef::Subtype("Elf"),
                        ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
                    ]),
                    &[ZoneKind::Battlefield],
                    PlayerRelation::You,
                ),
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(1),
                    ValueDef::Constant(1),
                ),
            },
        ),
        AbilityDef::activated_mana(
            "{T}: Add {G} for each Elf you control.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddManaEqualTo {
                color: ManaColor::Green,
                amount: ValueDef::CountMatchingObjects(&ObjectQueryDef::matching(
                    ObjectPredicateDef::Subtype("Elf"),
                    &[ZoneKind::Battlefield],
                    PlayerRelation::You,
                )),
            },
        ),
    ]),
);

// M10 203 — Runeclaw Bear
pub(in crate::card::sets) static RUNECLAW_BEAR: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("268bd9d5-4da1-4cbf-83f9-47f7aac1cfc3"),
    "Runeclaw Bear",
    crate::card::CardArt::new("6caf2b93-1971-4702-9aa5-bd223eb37a39", "Jesper Ejsing"),
    crate::card::CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{1}{G}"), &["Bear"], 2, 2),
);

// M10 204 — Stampeding Rhino
pub(in crate::card::sets) static STAMPEDING_RHINO: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f5a33394-d26c-4dcd-948c-e7d370059b11"),
    "Stampeding Rhino",
    crate::card::CardArt::new("09d34690-f7cc-4161-9a6f-bfc5393e40b2", "Steven Belledin"),
    crate::card::CardSet::Magic2010,
    CardRules::new_creature(mana_cost!("{4}{G}"), &["Rhino"], 4, 4)
        .with_abilities(&[abilities::trample()]),
);

// M10 205 — Windstorm
pub(in crate::card::sets) static WINDSTORM: CardRecord = CardRecord::new_with_legacy_id(
    1227,
    "Windstorm",
    CardArt::new("3cb7d122-34e8-48e1-a978-831c78a37d0c", "Rob Alexander"),
    CardSet::Magic2010,
    CardRules::new_instant(mana_cost!("{X}{G}")).with_ability(AbilityDef::spell(
        "Windstorm deals X damage to each creature with flying.",
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Creature),
                    ObjectPredicateDef::HasKeyword(crate::card::KeywordAbility::Flying),
                ]),
                &[ZoneKind::Battlefield],
                PlayerRelation::Any,
            ),
            amount: ValueDef::ChosenX,
        },
    )),
);

// M10 223 — Dragonskull Summit
pub(in crate::card::sets) static DRAGONSKULL_SUMMIT: CardRecord = CardRecord::new_with_legacy_id(
    1049,
    "Dragonskull Summit",
    CardArt::new("5e49c561-570c-43dd-a369-48bc7ad7edac", "Jon Foster"),
    CardSet::Magic2010,
    CardRules::new_land(&[]).with_abilities(&[
        abilities::check_land_enters(
            "This land enters tapped unless you control a Swamp or a Mountain.",
            &[BasicLandType::Swamp, BasicLandType::Mountain],
        ),
        AbilityDef::activated_mana(
            "{T}: Add {B} or {R}.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::choice(&[
                ManaColor::Black,
                ManaColor::Red,
            ])),
        ),
    ]),
);

// M10 224 — Drowned Catacomb
pub(in crate::card::sets) static DROWNED_CATACOMB: CardRecord = CardRecord::new_with_legacy_id(
    1050,
    "Drowned Catacomb",
    CardArt::new("8b41b86b-58e1-4601-b8ed-0ad31f03a78d", "Dave Kendall"),
    CardSet::Magic2010,
    CardRules::new_land(&[]).with_abilities(&[
        abilities::check_land_enters(
            "This land enters tapped unless you control an Island or a Swamp.",
            &[BasicLandType::Island, BasicLandType::Swamp],
        ),
        AbilityDef::activated_mana(
            "{T}: Add {U} or {B}.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::choice(&[
                ManaColor::Blue,
                ManaColor::Black,
            ])),
        ),
    ]),
);

// M10 226 — Glacial Fortress
pub(in crate::card::sets) static GLACIAL_FORTRESS: CardRecord = CardRecord::new_with_legacy_id(
    170,
    "Glacial Fortress",
    CardArt::new("bc9d29ee-1a21-4c3e-99c1-f815d40e8f19", "Franz Vohwinkel"),
    CardSet::Magic2010,
    CardRules::new_land(&[]).with_abilities(&[
        abilities::check_land_enters(
            "This land enters tapped unless you control a Plains or an Island.",
            &[BasicLandType::Plains, BasicLandType::Island],
        ),
        AbilityDef::activated_mana(
            "{T}: Add {W} or {U}.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::choice(&[
                ManaColor::White,
                ManaColor::Blue,
            ])),
        ),
    ]),
);

// M10 227 — Rootbound Crag
pub(in crate::card::sets) static ROOTBOUND_CRAG: CardRecord = CardRecord::new_with_legacy_id(
    205,
    "Rootbound Crag",
    CardArt::new("76364643-bfcb-4c50-9224-bf9e35648ddf", "Matt Stewart"),
    CardSet::Magic2010,
    CardRules::new_land(&[]).with_abilities(&[
        abilities::check_land_enters(
            "This land enters tapped unless you control a Mountain or a Forest.",
            &[BasicLandType::Mountain, BasicLandType::Forest],
        ),
        AbilityDef::activated_mana(
            "{T}: Add {R} or {G}.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::choice(&[
                ManaColor::Red,
                ManaColor::Green,
            ])),
        ),
    ]),
);

// M10 228 — Sunpetal Grove
pub(in crate::card::sets) static SUNPETAL_GROVE: CardRecord = CardRecord::new_with_legacy_id(
    221,
    "Sunpetal Grove",
    CardArt::new("15663129-9deb-4c34-84a0-f94cf1a723f0", "Jason Chan"),
    CardSet::Magic2010,
    CardRules::new_land(&[]).with_abilities(&[
        abilities::check_land_enters(
            "This land enters tapped unless you control a Forest or a Plains.",
            &[BasicLandType::Forest, BasicLandType::Plains],
        ),
        AbilityDef::activated_mana(
            "{T}: Add {G} or {W}.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::choice(&[
                ManaColor::Green,
                ManaColor::White,
            ])),
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &ANGELS_MERCY,
    &CAPTAIN_OF_THE_WATCH,
    &DIVINE_VERDICT,
    &ELITE_VANGUARD,
    &GLORIOUS_CHARGE,
    &GRIFFIN_SENTINEL,
    &HONOR_OF_THE_PURE,
    &INDESTRUCTIBILITY,
    &LIFELINK,
    &PLANAR_CLEANSING,
    &SAFE_PASSAGE,
    &SIEGE_MASTODON,
    &SILENCE,
    &SILVERCOAT_LION,
    &SOLEMN_OFFERING,
    &STORMFRONT_PEGASUS,
    &ALLURING_SIREN,
    &DIVINATION,
    &DJINN_OF_WISHES,
    &ESSENCE_SCATTER,
    &ICE_CAGE,
    &MIND_CONTROL,
    &SLEEP,
    &TOME_SCOUR,
    &WALL_OF_FROST,
    &CEMETERY_REAPER,
    &CHILD_OF_NIGHT,
    &DISENTOMB,
    &DOOM_BLADE,
    &RISE_FROM_THE_GRAVE,
    &SANGUINE_BOND,
    &SIGN_IN_BLOOD,
    &VAMPIRE_NOCTURNUS,
    &WARPATH_GHOUL,
    &ZOMBIE_GOLIATH,
    &ACT_OF_TREASON,
    &FIERY_HELLHOUND,
    &GOBLIN_CHIEFTAIN,
    &ACIDIC_SLIME,
    &BORDERLAND_RANGER,
    &BOUNTIFUL_HARVEST,
    &CENTAUR_COURSER,
    &CUDGEL_TROLL,
    &DEADLY_RECLUSE,
    &ELVISH_ARCHDRUID,
    &RUNECLAW_BEAR,
    &STAMPEDING_RHINO,
    &WINDSTORM,
    &DRAGONSKULL_SUMMIT,
    &DROWNED_CATACOMB,
    &GLACIAL_FORTRESS,
    &ROOTBOUND_CRAG,
    &SUNPETAL_GROVE,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
