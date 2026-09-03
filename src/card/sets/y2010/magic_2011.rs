//! Magic 2011 cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::AbilityCostDef;
use crate::AbilityTargetDef;
use crate::AbilityTargetPredicate;
use crate::BasicLandType;
use crate::ComparisonDef;
use crate::ManaColor;
use crate::ObjectQueryDef;
use crate::ObjectSetDef;
use crate::PlayerRefDef;
use crate::PlayerRelation;
use crate::ResolvedEffectDurationDef;
use crate::TargetChooserDef;
use crate::TargetIndex;
use crate::card::ConditionalStaticEffectDef;
use crate::card::ObjectSetCountConditionDef;
use crate::card::ScaledValueDef;
use crate::card::StaticApplyDef;
use crate::card::{
    AbilityDef, AppliedEffectDef, AppliedRuleDef, CardArt, CardRules, CardSet, CardType,
    CastTimingPermissionDef, DiscardSelectionDef, EffectDef, EffectRecipientDef,
    ObjectPredicateDef, TriggerEventDef, ValueDef, ZoneKind, ZonePlacement, abilities,
};
use crate::mana_cost;

// M11 6 — Assault Griffin
pub(in crate::card::sets) static ASSAULT_GRIFFIN: CardRecord = CardRecord::new_with_legacy_id(
    1056,
    "Assault Griffin",
    CardArt::new("704286a5-e3a8-4517-85e5-6447c5c2530f", "Eric Velhagen"),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{3}{W}"), &["Griffin"], 3, 2)
        .with_ability(abilities::flying()),
);

// M11 21 — Leyline of Sanctity
pub(in crate::card::sets) static LEYLINE_OF_SANCTITY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("262de9ae-d641-4f0e-af6a-03ce0e1c91d3"),
    "Leyline of Sanctity",
    CardArt::new("262de9ae-d641-4f0e-af6a-03ce0e1c91d3", "Ryan Pancoast"),
    CardSet::Magic2011,
    // Four mana for nothing at all, or nothing at all for a wall the
    // discard and the burn cannot see past.
    CardRules::new_enchantment(mana_cost!("{2}{W}{W}")).with_abilities(&[
        abilities::begin_game_on_battlefield("If this card is in your opening hand, you may begin the game with it on the battlefield."),
        AbilityDef::static_ability(
            "You have hexproof. (You can't be the target of spells or abilities your opponents control.)",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Controller,
                // The player, not the permanents: what this stops is a spell
                // that names its controller, and nothing that names a
                // creature they control.
                effect: AppliedEffectDef::Rule(AppliedRuleDef::PlayerRule(
                    crate::card::PlayerRuleDef::Hexproof,
                )),
            },
        ),
    ]),
);

// M11 22 — Mighty Leap
pub(in crate::card::sets) static MIGHTY_LEAP: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("bf8e0f93-a450-4188-a735-d601a59ab108"),
    "Mighty Leap",
    crate::card::CardArt::new("446e1676-ae7d-46ee-af91-bb54e4d18a78", "rk post"),
    crate::card::CardSet::Magic2011,
    CardRules::new_instant(mana_cost!("{1}{W}")).with_ability(AbilityDef::spell_with_targets(
        "Target creature gets +2/+2 and gains flying until end of turn.",
        &[AbilityTargetDef::exactly_one_permanent(
            ObjectPredicateDef::HasType(CardType::Creature),
        )],
        EffectDef::Apply {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            effect: AppliedEffectDef::Composite(&[
                AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(2),
                    ValueDef::Constant(2),
                ),
                AppliedEffectDef::add_ability(&abilities::flying()),
            ]),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    )),
);

// M11 25 — Roc Egg
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static ROC_EGG: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("1dca2c1f-3835-478b-860c-51b2036221b2"),
    "Roc Egg",
    crate::card::CardArt::new("92ae6206-ff0d-4248-b9cb-4ffbf20504fa", "Paul Bonner"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 30 — Silence (reprint)
const SILENCE_REPRINT: PrintingRecord =
    PrintingRecord::reprint(&crate::card::sets::y2009::magic_2010::SILENCE)
        .with_art("37b70d17-e4ec-4731-8892-b444f82be7a2", "Wayne Reynolds");

// M11 35 — Sun Titan
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SUN_TITAN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("bb07690e-d816-46de-84e7-617149a51b18"),
    "Sun Titan",
    crate::card::CardArt::new("ea3e77ed-9015-4407-b78c-494e46b67b07", "Todd Lockwood"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 38 — War Priest of Thune
pub(in crate::card::sets) static WAR_PRIEST_OF_THUNE: CardRecord = CardRecord::new_with_legacy_id(
    241,
    "War Priest of Thune",
    CardArt::new("d28eb320-aea7-466e-8718-de8652a2b191", "Izzy"),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Human", "Cleric"], 2, 2).with_abilities(&[
        abilities::enters_trigger_with_targets(
            "When this creature enters, you may destroy target enchantment.",
            &[AbilityTargetDef {
                predicate: AbilityTargetPredicate::Object {
                    object: ObjectPredicateDef::HasType(CardType::Enchantment),
                    zones: &[ZoneKind::Battlefield],
                    controller: None,
                    owner: None,
                },
                // "You may" is an optional target: declining to choose one is how the
                // trigger does nothing, so the minimum is zero rather than one.
                minimum: 0,
                maximum: 1,
                exact_count: None,
                divided_total: None,
                another: false,
                excludes_source: false,
                chooser: TargetChooserDef::Controller,
            }],
            EffectDef::Destroy {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                can_regenerate: true,
                then: None,
            },
        ),
    ]),
);

// M11 41 — Aether Adept
pub(in crate::card::sets) static AETHER_ADEPT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("0b551dab-1a81-406d-b708-b3b7300eb02e"),
    "Aether Adept",
    crate::card::CardArt::new("fa6f04ca-cab7-4c86-a56c-79d6ae3b73e6", "Eric Deschamps"),
    crate::card::CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{1}{U}{U}"), &["Human", "Wizard"], 2, 2).with_ability(
        abilities::enters_trigger_with_targets(
            "When this creature enters, return target creature to its owner's hand.",
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::HasType(CardType::Creature),
            )],
            EffectDef::MoveToZone {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                zone: ZoneKind::Hand,
                placement: ZonePlacement::Top,
            },
        ),
    ),
);

// M11 42 — Air Servant
pub(in crate::card::sets) static AIR_SERVANT: CardRecord = CardRecord::new_with_legacy_id(
    1161,
    "Air Servant",
    CardArt::new("0cbc279d-952a-4b8d-b6ff-37166daa2dd5", "Lars Grant-West"),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{4}{U}"), &["Elemental"], 4, 3).with_abilities(&[
        abilities::flying(),
        AbilityDef::activated_with_targets(
            "{2}{U}: Tap target creature with flying.",
            &[AbilityCostDef::Mana(mana_cost!("{2}{U}"))],
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Creature),
                    ObjectPredicateDef::HasKeyword(crate::card::KeywordAbility::Flying),
                ]),
            )],
            EffectDef::Tap {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            },
        ),
    ]),
);

// M11 44 — Armored Cancrix
pub(in crate::card::sets) static ARMORED_CANCRIX: CardRecord = CardRecord::new_with_legacy_id(
    1162,
    "Armored Cancrix",
    CardArt::new("3b455b0f-a69c-43b4-bbf5-605ed41f10e0", "Tomasz Jedruszek"),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{4}{U}"), &["Crab"], 2, 5),
);

// M11 55 — Frost Titan
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static FROST_TITAN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("065addc8-c235-43cc-a54f-b582826e5df1"),
    "Frost Titan",
    crate::card::CardArt::new("358baa9f-390f-4b99-a274-d28f3bd56824", "Mike Bierek"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 56 — Harbor Serpent
pub(in crate::card::sets) static HARBOR_SERPENT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("aa10b43f-eb63-4999-92a0-56826031b686"),
    "Harbor Serpent",
    crate::card::CardArt::new("af0f7357-08b0-403e-8913-8965662a905e", "Daarken"),
    crate::card::CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{4}{U}{U}"), &["Serpent"], 5, 5).with_abilities(&[
        abilities::landwalk(BasicLandType::Island),
        AbilityDef::static_ability(
            "This creature can't attack unless there are five or more Islands on the battlefield.",
            EffectDef::ConditionalStatic(ConditionalStaticEffectDef {
                condition: ObjectSetCountConditionDef {
                    objects: &ObjectSetDef::Query(ObjectQueryDef::matching(
                        ObjectPredicateDef::HasAnyBasicLandType(&[BasicLandType::Island]),
                        &[ZoneKind::Battlefield],
                        PlayerRelation::Any,
                    )),
                    filter: None,
                    comparison: ComparisonDef::Less,
                    amount: 5,
                },
                then: StaticApplyDef {
                    recipient: EffectRecipientDef::Source,
                    effect: AppliedEffectDef::Rule(AppliedRuleDef::CANNOT_ATTACK),
                },
            }),
        ),
    ]),
);

// M11 59 — Jace's Erasure
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static JACE_S_ERASURE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("3662d1cc-1279-409f-9f0a-9c15c3407103"),
    "Jace's Erasure",
    crate::card::CardArt::new("970f4f34-f834-41a7-aff1-7cef82cefc74", "Jason Chan"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 61 — Leyline of Anticipation
pub(in crate::card::sets) static LEYLINE_OF_ANTICIPATION: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("d7dbb092-3bb0-445e-ab26-d939cac92a73"),
    "Leyline of Anticipation",
    CardArt::new("d7dbb092-3bb0-445e-ab26-d939cac92a73", "Charles Urbach"),
    CardSet::Magic2011,
    CardRules::new_enchantment(mana_cost!("{2}{U}{U}")).with_abilities(&[
        abilities::begin_game_on_battlefield("If this card is in your opening hand, you may begin the game with it on the battlefield."),
        AbilityDef::static_ability(
            "You may cast spells as though they had flash.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Controller,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::MayCastAsThoughItHadFlash(
                    CastTimingPermissionDef::new(ObjectPredicateDef::Any),
                )),
            },
        ),
    ]),
);

// M11 66 — Merfolk Spy
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static MERFOLK_SPY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("b5ae05cc-116b-4268-ba78-709aeff36ab1"),
    "Merfolk Spy",
    crate::card::CardArt::new(
        "b5ae05cc-116b-4268-ba78-709aeff36ab1",
        "Matt Cavotta & Richard Whitters",
    ),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 70 — Preordain
pub(in crate::card::sets) static PREORDAIN: CardRecord = CardRecord::new_with_legacy_id(
    2130,
    "Preordain",
    CardArt::new("e3868c3d-4fcd-444b-866f-0f8e50ce7b67", "Svetlin Velinov"),
    CardSet::Magic2011,
    CardRules::new_sorcery(mana_cost!("{U}")).with_ability(AbilityDef::spell(
        "Scry 2, then draw a card.",
        EffectDef::Sequence(&[
            abilities::scry(ValueDef::Constant(2)),
            EffectDef::DrawCards {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(1),
            },
        ]),
    )),
);

// M11 71 — Redirect
pub(in crate::card::sets) static REDIRECT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("60bae44b-c6f2-40bf-a427-aee5cfbdfea9"),
    "Redirect",
    crate::card::CardArt::new("0eef8431-f63c-44e0-940c-e1a38c338214", "Izzy"),
    crate::card::CardSet::Magic2011,
    CardRules::new_instant(mana_cost!("{U}{U}")).with_ability(AbilityDef::spell_with_targets(
        "You may choose new targets for target spell.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::Spell,
                zones: &[ZoneKind::Stack],
                controller: None,
                owner: None,
            },
        )],
        EffectDef::ChangeStackTargets(&crate::card::ChangeStackTargetsDef {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            chooser: PlayerRefDef::EffectController,
            change: crate::card::StackTargetChangeDef::ChooseNew {
                optional: true,
                restriction: None,
            },
        }),
    )),
);

// M11 72 — Scroll Thief
pub(in crate::card::sets) static SCROLL_THIEF: CardRecord = CardRecord::new_with_legacy_id(
    991,
    "Scroll Thief",
    CardArt::new(
        "dc201a82-fb48-4bb4-b072-e206e6872aa5",
        "Alex Horley-Orlandelli",
    ),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{2}{U}"), &["Merfolk", "Rogue"], 1, 3).with_ability(
        AbilityDef::triggered(
            "Whenever this creature deals combat damage to a player, draw a card.",
            TriggerEventDef::combat_damage_to_player(ObjectPredicateDef::Source),
            abilities::draw_cards(ValueDef::Constant(1)),
        ),
    ),
);

// M11 74 — Stormtide Leviathan
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static STORMTIDE_LEVIATHAN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("0e7f3fb6-93ce-4bc9-8efd-11af5a46218f"),
    "Stormtide Leviathan",
    crate::card::CardArt::new("0e7f3fb6-93ce-4bc9-8efd-11af5a46218f", "Karl Kopinski"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 75 — Time Reversal
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static TIME_REVERSAL: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("1468c851-b20e-4c78-9fcb-45e60b7149db"),
    "Time Reversal",
    crate::card::CardArt::new("2d6500a1-5aea-4b83-b4dc-560fe547590d", "Howard Lyon"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 80 — Water Servant
pub(in crate::card::sets) static WATER_SERVANT: CardRecord = CardRecord::new_with_legacy_id(
    1174,
    "Water Servant",
    CardArt::new("a2c7562e-3e25-447d-b9f4-eb96960511b8", "Igor Kieryluk"),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{2}{U}{U}"), &["Elemental"], 3, 4).with_abilities(&[
        AbilityDef::activated(
            "{U}: This creature gets +1/-1 until end of turn.",
            &[AbilityCostDef::Mana(mana_cost!("{U}"))],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(1),
                    ValueDef::Constant(-1),
                ),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        ),
        AbilityDef::activated(
            "{U}: This creature gets -1/+1 until end of turn.",
            &[AbilityCostDef::Mana(mana_cost!("{U}"))],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(-1),
                    ValueDef::Constant(1),
                ),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        ),
    ]),
);

// M11 97 — Grave Titan
pub(in crate::card::sets) static GRAVE_TITAN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("5fa6d385-6b8e-45ad-83dc-b477799c05a5"),
    "Grave Titan",
    CardArt::new("5c70da33-ce5d-4b8b-9c1d-9a356a7e196f", "Nils Hamm"),
    CardSet::Magic2011,
    // Ten power over three bodies for six mana, and killing the Titan still
    // leaves four of it behind.
    CardRules::new_creature(mana_cost!("{4}{B}{B}"), &["Giant"], 6, 6)
        .with_abilities(&[
            abilities::deathtouch(),
            AbilityDef::triggered(
                "Whenever this creature enters or attacks, create two 2/2 black Zombie creature tokens.",
                // One printed ability with two ways in, the way every Titan prints it: a
                // Titan that lands and then attacks makes four Zombies, and it makes them
                // as two separate triggers.
                TriggerEventDef::AnyOf(&[
                    TriggerEventDef::zone_changed(
                        ObjectPredicateDef::Source,
                        None,
                        Some(ZoneKind::Battlefield),
                    ),
                    TriggerEventDef::attacks(ObjectPredicateDef::Source),
                ]),
                EffectDef::create_creature_token(&["Zombie"], &[ManaColor::Black], 2, 2).with_amount(2),
            ),
        ]),
);

// M11 104 — Liliana's Specter
pub(in crate::card::sets) static LILIANA_S_SPECTER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("33122581-39fd-44a0-b928-f73e39a0c0f1"),
    "Liliana's Specter",
    crate::card::CardArt::new("33122581-39fd-44a0-b928-f73e39a0c0f1", "Vance Kovacs"),
    crate::card::CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{1}{B}{B}"), &["Specter"], 2, 1).with_abilities(&[
        abilities::flying(),
        abilities::enters_trigger(
            "When this creature enters, each opponent discards a card.",
            EffectDef::Discard {
                recipient: EffectRecipientDef::Opponent,
                amount: ValueDef::Constant(1),
                selection: DiscardSelectionDef::RecipientChooses,
                then: None,
            },
        ),
    ]),
);

// M11 109 — Nightwing Shade
pub(in crate::card::sets) static NIGHTWING_SHADE: CardRecord = CardRecord::new_with_legacy_id(
    1188,
    "Nightwing Shade",
    CardArt::new("a3112a8a-dc80-4099-966c-8fa1807a189b", "Lucas Graciano"),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{4}{B}"), &["Shade"], 2, 2).with_abilities(&[
        abilities::flying(),
        AbilityDef::activated(
            "{1}{B}: This creature gets +1/+1 until end of turn.",
            &[AbilityCostDef::Mana(mana_cost!("{1}{B}"))],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(1),
                    ValueDef::Constant(1),
                ),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        ),
    ]),
);

// M11 110 — Phylactery Lich
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static PHYLACTERY_LICH: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("9d088983-92c1-4f4d-8abf-dd20347495b5"),
    "Phylactery Lich",
    crate::card::CardArt::new("9d088983-92c1-4f4d-8abf-dd20347495b5", "Michael Komarck"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 111 — Quag Sickness
static QUAG_SICKNESS_PENALTY: ValueDef = ValueDef::Scaled(&ScaledValueDef::new(
    ValueDef::CountMatchingObjects(&ObjectQueryDef::matching(
        ObjectPredicateDef::Subtype("Swamp"),
        &[ZoneKind::Battlefield],
        PlayerRelation::You,
    )),
    -1,
));

pub(in crate::card::sets) static QUAG_SICKNESS: CardRecord = CardRecord::new_with_legacy_id(
    1189,
    "Quag Sickness",
    CardArt::new("a759dcd2-ca07-4428-a3ea-b2e829b1fcb4", "Martina Pilcerova"),
    CardSet::Magic2011,
    CardRules::new_enchantment(mana_cost!("{2}{B}"))
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
                "Enchanted creature gets -1/-1 for each Swamp you control.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::modify_power_toughness(
                        QUAG_SICKNESS_PENALTY,
                        QUAG_SICKNESS_PENALTY,
                    ),
                },
            ),
        ]),
);

// M11 130 — Combust
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static COMBUST: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("cf23a422-25a7-4c8a-9cff-24563ec20ea7"),
    "Combust",
    crate::card::CardArt::new("f10346e2-46bd-4257-b191-c36c2577c534", "Jaime Jones"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 146 — Inferno Titan
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static INFERNO_TITAN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f1e4a028-6462-4373-9864-a8adfc78d52b"),
    "Inferno Titan",
    crate::card::CardArt::new("e04c24cb-3c3b-4a35-9694-db512bf394fa", "Kev Walker"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 148 — Leyline of Punishment
pub(in crate::card::sets) static LEYLINE_OF_PUNISHMENT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("51a2eec5-f892-4466-b6c6-960626ba5640"),
    "Leyline of Punishment",
    CardArt::new("51a2eec5-f892-4466-b6c6-960626ba5640", "Charles Urbach"),
    CardSet::Magic2011,
    CardRules::new_enchantment(mana_cost!("{2}{R}{R}")).with_abilities(&[
        abilities::begin_game_on_battlefield("If this card is in your opening hand, you may begin the game with it on the battlefield."),
        AbilityDef::static_ability(
            "Players can't gain life. Damage can't be prevented.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::EachPlayer,
                effect: AppliedEffectDef::Composite(&[
                    AppliedEffectDef::Rule(AppliedRuleDef::CannotGainLife),
                    AppliedEffectDef::Rule(AppliedRuleDef::PlayerRule(
                        crate::card::PlayerRuleDef::DamageCannotBePrevented,
                    )),
                ]),
            },
        ),
    ]),
);

// M11 151 — Manic Vandal
pub(in crate::card::sets) static MANIC_VANDAL: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("a503697a-4940-4b8f-98b1-5ea9151866fa"),
    "Manic Vandal",
    crate::card::CardArt::new(
        "985a5866-8c62-46af-a0c0-e69d01d87f4f",
        "Christopher Moeller",
    ),
    crate::card::CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{2}{R}"), &["Human", "Warrior"], 2, 2).with_ability(
        abilities::enters_trigger_with_targets(
            "When this creature enters, destroy target artifact.",
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::HasType(CardType::Artifact),
            )],
            EffectDef::Destroy {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                can_regenerate: true,
                then: None,
            },
        ),
    ),
);

// M11 155 — Reverberate
pub(in crate::card::sets) static REVERBERATE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("dd435013-0ab9-42f4-985c-66ea2b3760e9"),
    "Reverberate",
    crate::card::CardArt::new("5996feb4-02ac-45e8-a7f2-966cf74391dc", "jD"),
    crate::card::CardSet::Magic2011,
    CardRules::new_instant(mana_cost!("{R}{R}")).with_ability(AbilityDef::spell_with_targets(
        "Copy target instant or sorcery spell. You may choose new targets for the copy.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::All(&[
                    ObjectPredicateDef::Spell,
                    ObjectPredicateDef::AnyOf(&[
                        ObjectPredicateDef::HasType(CardType::Instant),
                        ObjectPredicateDef::HasType(CardType::Sorcery),
                    ]),
                ]),
                zones: &[ZoneKind::Stack],
                controller: None,
                owner: None,
            },
        )],
        EffectDef::CopyStackObject(&crate::card::CopyStackObjectDef {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            controller: PlayerRefDef::EffectController,
            count: ValueDef::Constant(1),
            retarget: true,
            colors: None,
        }),
    )),
);

// M11 157 — Thunder Strike
pub(in crate::card::sets) static THUNDER_STRIKE: CardRecord = CardRecord::new_with_legacy_id(
    1212,
    "Thunder Strike",
    CardArt::new("61aa445d-d734-4e4f-800d-fe7bea86eb70", "Wayne Reynolds"),
    CardSet::Magic2011,
    CardRules::new_instant(mana_cost!("{1}{R}")).with_ability(AbilityDef::spell_with_targets(
        "Target creature gets +2/+0 and gains first strike until end of turn.",
        &[AbilityTargetDef::exactly_one_permanent(
            ObjectPredicateDef::HasType(CardType::Creature),
        )],
        EffectDef::Apply {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            effect: AppliedEffectDef::Composite(&[
                AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(2),
                    ValueDef::Constant(0),
                ),
                AppliedEffectDef::add_ability(&abilities::first_strike()),
            ]),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    )),
);

// M11 158 — Volcanic Strength
pub(in crate::card::sets) static VOLCANIC_STRENGTH: CardRecord = CardRecord::new_with_legacy_id(
    239,
    "Volcanic Strength",
    CardArt::new("f1963f08-1765-4f3e-92be-479773de47a0", "Izzy"),
    CardSet::Magic2011,
    CardRules::new_enchantment(mana_cost!("{1}{R}"))
        .with_subtypes(&["Aura"])
        .with_abilities(&[
        AbilityDef::spell_with_targets("Enchant creature", &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::HasType(CardType::Creature),
                zones: &[ZoneKind::Battlefield],
                controller: None,
                owner: None,
            },
        )], EffectDef::Attach {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            }),
        AbilityDef::static_ability(
            "Enchanted creature gets +2/+2 and has mountainwalk. (It can't be blocked as long as defending player controls a Mountain.)",
            EffectDef::Sequence(&[
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::modify_power_toughness(ValueDef::Constant(2), ValueDef::Constant(2)),
                },
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::add_ability(&abilities::mountainwalk()),
                },
            ]),
        ),
    ]),
);

// M11 162 — Autumn's Veil
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static AUTUMN_S_VEIL: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("7e354ce5-b4c1-4a9c-99d1-7624301b594b"),
    "Autumn's Veil",
    crate::card::CardArt::new("b911fee0-c30b-4d68-a9e2-61c40ece68b0", "Kekai Kotaki"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 166 — Brindle Boar
pub(in crate::card::sets) static BRINDLE_BOAR: CardRecord = CardRecord::new_with_legacy_id(
    1215,
    "Brindle Boar",
    CardArt::new("a30b4a78-afdd-4067-810e-1fa0ddf8fb0e", "Dave Allsop"),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{2}{G}"), &["Boar"], 2, 2).with_ability(
        AbilityDef::activated(
            "Sacrifice this creature: You gain 4 life.",
            &[AbilityCostDef::SacrificeSource],
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(4),
            },
        ),
    ),
);

// M11 176 — Garruk's Companion
pub(in crate::card::sets) static GARRUK_S_COMPANION: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("863c9a10-d83f-415b-adf2-2d0f870410b2"),
    "Garruk's Companion",
    crate::card::CardArt::new("b8d8806c-43c5-4c6c-9420-6210a17ec2b0", "Efrem Palacios"),
    crate::card::CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{G}{G}"), &["Beast"], 3, 2)
        .with_abilities(&[abilities::trample()]),
);

// M11 177 — Garruk's Packleader
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static GARRUK_S_PACKLEADER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("dfaef299-7879-4f52-8ee4-701ed150b930"),
    "Garruk's Packleader",
    crate::card::CardArt::new("dfaef299-7879-4f52-8ee4-701ed150b930", "Nils Hamm"),
    crate::card::CardSet::Magic2011,
    crate::card::CardRules::unsupported(),
);

// M11 180 — Greater Basilisk
pub(in crate::card::sets) static GREATER_BASILISK: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("482f169d-8acd-4ee3-a54c-6df6cbeb7eca"),
    "Greater Basilisk",
    crate::card::CardArt::new("994711cb-e85b-4acb-9460-17231e1d66ad", "James Ryman"),
    crate::card::CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{3}{G}{G}"), &["Basilisk"], 3, 5)
        .with_abilities(&[abilities::deathtouch()]),
);

// M11 183 — Leyline of Vitality
pub(in crate::card::sets) static LEYLINE_OF_VITALITY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f5318113-9dfb-492c-9151-de90951d881e"),
    "Leyline of Vitality",
    CardArt::new("f5318113-9dfb-492c-9151-de90951d881e", "Jim Nelson"),
    CardSet::Magic2011,
    CardRules::new_enchantment(mana_cost!("{2}{G}{G}")).with_abilities(&[
        abilities::begin_game_on_battlefield("If this card is in your opening hand, you may begin the game with it on the battlefield."),
        AbilityDef::static_ability(
            "Creatures you control get +0/+1.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::matching_objects(
                    ObjectPredicateDef::HasType(CardType::Creature),
                    &[ZoneKind::Battlefield],
                    crate::card::PlayerRelation::You,
                ),
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(0),
                    ValueDef::Constant(1),
                ),
            },
        ),
        AbilityDef::triggered(
            "Whenever a creature you control enters, you may gain 1 life.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Creature),
                    ObjectPredicateDef::ControlledBy(crate::card::PlayerRelation::You),
                ]),
                None,
                Some(ZoneKind::Battlefield),
            ),
            EffectDef::May {
                player: EffectRecipientDef::Controller,
                effect: &EffectDef::GainLife {
                    recipient: EffectRecipientDef::Controller,
                    amount: ValueDef::Constant(1),
                },
            },
        ),
    ]),
);

// M11 192 — Primeval Titan
pub(in crate::card::sets) static PRIMEVAL_TITAN: CardRecord = CardRecord::new_with_legacy_id(
    2128,
    "Primeval Titan",
    CardArt::new("feee9327-b937-46ba-a2aa-6c015ab6cdd5", "Aleksi Briclot"),
    CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{4}{G}{G}"), &["Giant"], 6, 6).with_abilities(&[
        abilities::trample(),
        AbilityDef::triggered(
            "Whenever this creature enters or attacks, you may search your library for up to two land cards, put them onto the battlefield tapped, then shuffle.",
            // One printed ability with two ways in, not two abilities: the card says
            // "enters or attacks", and a Titan that does both in a turn triggers twice
            // for the same reason it would have anyway.
            TriggerEventDef::AnyOf(&[
                TriggerEventDef::zone_changed(
                    ObjectPredicateDef::Source,
                    None,
                    Some(ZoneKind::Battlefield),
                ),
                TriggerEventDef::attacks(ObjectPredicateDef::Source),
            ]),
            EffectDef::May {
                player: EffectRecipientDef::Controller,
                // Any land card, not just a basic: the two it finds are usually the two the
                // deck was built around.
                effect: &EffectDef::SearchZone {
                    player: EffectRecipientDef::Controller,
                    source: ZoneKind::Library,
                    object: ObjectPredicateDef::HasType(CardType::Land),
                    minimum: 0,
                    maximum: ValueDef::Constant(2),
                    reveal: false,
                    destination: ZoneKind::Battlefield,
                    placement: ZonePlacement::Top,
                    shuffle: true,
                    enters_tapped: true,
                    attachment: None,
                    binding: None,
                    then: None,
                },
            },
        ),
    ]),
);

// M11 196 — Sacred Wolf
pub(in crate::card::sets) static SACRED_WOLF: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("a2bffe20-c469-4ac8-a8a9-361a244f4cfe"),
    "Sacred Wolf",
    crate::card::CardArt::new("ff4661dd-2075-48c3-b19b-fc7f8aaba1b8", "Matt Stewart"),
    crate::card::CardSet::Magic2011,
    CardRules::new_creature(mana_cost!("{2}{G}"), &["Wolf"], 3, 1)
        .with_abilities(&[abilities::hexproof()]),
);

// M11 206 — Elixir of Immortality
pub(in crate::card::sets) static ELIXIR_OF_IMMORTALITY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("99bd4740-9b1f-40a6-a14d-2c0d642b848b"),
    "Elixir of Immortality",
    crate::card::CardArt::new(
        "813d6a95-719d-474d-942a-b4c5156af7ba",
        "Zoltan Boros & Gabor Szikszai",
    ),
    crate::card::CardSet::Magic2011,
    CardRules::new_artifact(mana_cost!("{1}")).with_ability(AbilityDef::activated(
        "{2}, {T}: You gain 5 life. Shuffle this artifact and your graveyard into their owner's library.",
        &[
            AbilityCostDef::Mana(mana_cost!("{2}")),
            AbilityCostDef::TapSource,
        ],
        EffectDef::Sequence(&[
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(5),
            },
            EffectDef::MoveToZone {
                object: EffectRecipientDef::Source,
                zone: ZoneKind::Library,
                placement: ZonePlacement::Top,
            },
            EffectDef::MoveToZone {
                object: EffectRecipientDef::matching_objects(
                    ObjectPredicateDef::Any,
                    &[ZoneKind::Graveyard],
                    PlayerRelation::You,
                ),
                zone: ZoneKind::Library,
                placement: ZonePlacement::Top,
            },
            EffectDef::ShuffleLibrary {
                player: EffectRecipientDef::Controller,
            },
        ]),
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &ASSAULT_GRIFFIN,
    &LEYLINE_OF_SANCTITY,
    &MIGHTY_LEAP,
    &ROC_EGG,
    &SUN_TITAN,
    &WAR_PRIEST_OF_THUNE,
    &AETHER_ADEPT,
    &AIR_SERVANT,
    &ARMORED_CANCRIX,
    &FROST_TITAN,
    &HARBOR_SERPENT,
    &JACE_S_ERASURE,
    &LEYLINE_OF_ANTICIPATION,
    &MERFOLK_SPY,
    &PREORDAIN,
    &REDIRECT,
    &SCROLL_THIEF,
    &STORMTIDE_LEVIATHAN,
    &TIME_REVERSAL,
    &WATER_SERVANT,
    &GRAVE_TITAN,
    &LILIANA_S_SPECTER,
    &NIGHTWING_SHADE,
    &PHYLACTERY_LICH,
    &QUAG_SICKNESS,
    &COMBUST,
    &INFERNO_TITAN,
    &LEYLINE_OF_PUNISHMENT,
    &MANIC_VANDAL,
    &REVERBERATE,
    &THUNDER_STRIKE,
    &VOLCANIC_STRENGTH,
    &AUTUMN_S_VEIL,
    &BRINDLE_BOAR,
    &GARRUK_S_COMPANION,
    &GARRUK_S_PACKLEADER,
    &GREATER_BASILISK,
    &LEYLINE_OF_VITALITY,
    &PRIMEVAL_TITAN,
    &SACRED_WOLF,
    &ELIXIR_OF_IMMORTALITY,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[SILENCE_REPRINT];
