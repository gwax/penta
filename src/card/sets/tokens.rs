//! Token definitions.
//!
//! A token is not a printed card, but it is a permanent with characteristics,
//! so it is cataloged like anything else. `CardSet::Token` belongs to no
//! format's allowed sets, which is what keeps a token out of every decklist
//! while still letting a client resolve one by definition.
//!
//! A token has no mana cost, so its colors come from a printed color rather
//! than from a cost, and it carries no art: a Scryfall identifier names a
//! printing, and the client already falls back to the type glyph without one.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityCoverageDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate,
    ActivationTimingDef, AddManaEffectDef, AppliedEffectDef, AppliedRuleDef, BandingQuality,
    CardArt, CardComposition, CardPart, CardRules, CardSet, CardStructure, CardType,
    ControlDurationDef, DoubleFacedKind, EffectDef, EffectRecipientDef, ManaColor,
    ObjectPredicateDef, ObjectQueryDef, ObjectRefDef, ObjectSetDef, PlayerRefDef, PlayerRelation,
    ResolvedEffectDurationDef, TriggerEventDef, ValueDef, ZoneKind, abilities, cards,
};
use crate::ids::CardPartId;
use crate::{TargetIndex, mana_cost};

/// Not a token: the body a face-down permanent presents while it is face
/// down. It lives here because it needs the same thing a token needs -- a
/// catalog definition no format allows -- and because nothing may ever put a
/// card of it into a deck. A face-down permanent's own `card.definition`
/// stays the card underneath, so it is never treated as a token.
pub(in crate::card::sets) static FACE_DOWN_CREATURE: CardRecord = CardRecord::new(
    cards::FACE_DOWN_CREATURE,
    "Face-down creature",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&[], 2, 2),
);

pub(in crate::card::sets) static GERM_TOKEN_0_0_BLACK: CardRecord = CardRecord::new(
    cards::GERM_TOKEN_0_0_BLACK,
    "Phyrexian Germ",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Phyrexian", "Germ"], 0, 0)
        .printed_colors(&[ManaColor::Black]),
);

/// What amass makes when you control no Army yet. It arrives with no counters
/// on it at all, so it is a creature that dies to state-based actions the
/// moment the counters the amass promised fail to land.
pub(in crate::card::sets) static ORC_ARMY_TOKEN_0_0_BLACK: CardRecord = CardRecord::new(
    cards::ORC_ARMY_TOKEN_0_0_BLACK,
    "Orc Army",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Orc", "Army"], 0, 0)
        .printed_colors(&[ManaColor::Black]),
);

pub(in crate::card::sets) static BEAST_TOKEN_3_3_GREEN: CardRecord = CardRecord::new(
    cards::BEAST_TOKEN_3_3_GREEN,
    "Beast",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Beast"], 3, 3).printed_colors(&[ManaColor::Green]),
);

pub(in crate::card::sets) static KNIGHT_TOKEN_2_2_WHITE: CardRecord = CardRecord::new(
    cards::KNIGHT_TOKEN_2_2_WHITE,
    "Knight",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Knight"], 2, 2)
        .printed_colors(&[ManaColor::White])
        .with_abilities(&[abilities::vigilance()]),
);

pub(in crate::card::sets) static SOLDIER_TOKEN_1_1_RED_WHITE: CardRecord = CardRecord::new(
    cards::SOLDIER_TOKEN_1_1_RED_WHITE,
    "Soldier",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Soldier"], 1, 1)
        .printed_colors(&[ManaColor::Red, ManaColor::White])
        .with_abilities(&[abilities::haste()]),
);

pub(in crate::card::sets) static DEMON_TOKEN_5_5_BLACK: CardRecord = CardRecord::new(
    cards::DEMON_TOKEN_5_5_BLACK,
    "Demon",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Demon"], 5, 5)
        .printed_colors(&[ManaColor::Black])
        .with_abilities(&[abilities::flying()]),
);

/// Voice of Resurgence's token. Its printed power and toughness are defined
/// by the board, which a zero-power body plus a counting static bonus says
/// exactly: the count includes the token itself, so it is never a 0/0.
pub(in crate::card::sets) static ELEMENTAL_TOKEN_GREEN_WHITE: CardRecord = CardRecord::new(
    cards::ELEMENTAL_TOKEN_GREEN_WHITE,
    "Elemental",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Elemental"], 0, 0)
        .printed_colors(&[ManaColor::Green, ManaColor::White])
        .with_ability(AbilityDef::static_ability(
            "This token's power and toughness are each equal to the number of creatures you control.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(ValueDef::CountMatchingObjects(&CREATURES_YOU_CONTROL), ValueDef::CountMatchingObjects(&CREATURES_YOU_CONTROL)),
            },
        )),
);

static CREATURES_YOU_CONTROL: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::HasType(CardType::Creature),
    &[ZoneKind::Battlefield],
    PlayerRelation::You,
);

pub(in crate::card::sets) static SPIRIT_TOKEN_1_1_WHITE: CardRecord = CardRecord::new(
    cards::SPIRIT_TOKEN_1_1_WHITE,
    "Spirit",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Spirit"], 1, 1)
        .printed_colors(&[ManaColor::White])
        .with_abilities(&[abilities::flying()]),
);

pub(in crate::card::sets) static WOLF_TOKEN_2_2_GREEN: CardRecord = CardRecord::new(
    cards::WOLF_TOKEN_2_2_GREEN,
    "Wolf",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Wolf"], 2, 2).printed_colors(&[ManaColor::Green]),
);

/// Master of the Hunt's pack. The name is the whole point: its "bands with
/// other" names the same name, so the tokens band with each other and with
/// nothing else.
pub(in crate::card::sets) static WOLVES_OF_THE_HUNT_TOKEN_1_1_GREEN: CardRecord = CardRecord::new(
    cards::WOLVES_OF_THE_HUNT_TOKEN_1_1_GREEN,
    "Wolves of the Hunt",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Wolf"], 1, 1)
        .printed_colors(&[ManaColor::Green])
        .with_abilities(&[abilities::bands_with_other(BandingQuality::WolvesOfTheHunt)]),
);

pub(in crate::card::sets) static WOLF_TOKEN_1_1_BLACK: CardRecord = CardRecord::new(
    cards::WOLF_TOKEN_1_1_BLACK,
    "Wolf",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Wolf"], 1, 1)
        .printed_colors(&[ManaColor::Black])
        .with_abilities(&[abilities::deathtouch()]),
);

/// Bottle of Suleiman's reward for winning its flip.
pub(in crate::card::sets) static DJINN_TOKEN_5_5_COLORLESS: CardRecord = CardRecord::new(
    cards::DJINN_TOKEN_5_5_COLORLESS,
    "Djinn",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Djinn"], 5, 5)
        .with_type(CardType::Artifact)
        .with_abilities(&[abilities::flying()]),
);

/// Tetravus detaches these, and can exile its own back to rebuild itself.
pub(in crate::card::sets) static RABBIT_TOKEN_1_1_WHITE: CardRecord = CardRecord::new(
    cards::RABBIT_TOKEN_1_1_WHITE,
    "Rabbit",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Rabbit"], 1, 1)
        .printed_colors(&[ManaColor::White]),
);

pub(in crate::card::sets) static SERVO_TOKEN_1_1_COLORLESS: CardRecord = CardRecord::new(
    cards::SERVO_TOKEN_1_1_COLORLESS,
    "Servo",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Servo"], 1, 1).with_type(CardType::Artifact),
);

pub(in crate::card::sets) static GOLEM_TOKEN_3_3_COLORLESS: CardRecord = CardRecord::new(
    cards::GOLEM_TOKEN_3_3_COLORLESS,
    "Golem",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Golem"], 3, 3).with_type(CardType::Artifact),
);

pub(in crate::card::sets) static TETRAVITE_TOKEN: CardRecord = CardRecord::new(
    cards::TETRAVITE_TOKEN,
    "Tetravite",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Tetravite"], 1, 1)
        .with_type(CardType::Artifact)
        .with_abilities(&[
            abilities::flying(),
            AbilityDef::static_ability(
                "This token can't be enchanted.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::Source,
                    effect: AppliedEffectDef::Rule(AppliedRuleDef::CannotBeEnchanted),
                },
            )
            .with_coverage(AbilityCoverageDef::explained_complete(
                "The shared targetability check refuses the token to an Aura spell, and an Aura that arrives some other way still falls off.",
            )),
        ]),
);

/// Vraska's ultimate. One connection ends the game, so the token's whole
/// point is the trigger rather than its body.
pub(in crate::card::sets) static ASSASSIN_TOKEN_1_1_BLACK: CardRecord = CardRecord::new(
    cards::ASSASSIN_TOKEN_1_1_BLACK,
    "Assassin",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Assassin"], 1, 1)
        .printed_colors(&[ManaColor::Black])
        .with_ability(AbilityDef::triggered(
            "Whenever this token deals combat damage to a player, that player loses the game.",
            TriggerEventDef::combat_damage_to_player(ObjectPredicateDef::Source),
            EffectDef::LoseTheGame {
                player: EffectRecipientDef::EventPlayer,
            },
        )),
);

pub(in crate::card::sets) static BIRD_TOKEN_4_4_RED: CardRecord = CardRecord::new(
    cards::BIRD_TOKEN_4_4_RED,
    "Bird",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Bird"], 4, 4)
        .printed_colors(&[ManaColor::Red])
        .with_abilities(&[abilities::flying()]),
);

pub(in crate::card::sets) static CITIZEN_TOKEN_1_1_WHITE: CardRecord = CardRecord::new(
    cards::CITIZEN_TOKEN_1_1_WHITE,
    "Citizen",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Citizen"], 1, 1)
        .printed_colors(&[ManaColor::White]),
);

pub(in crate::card::sets) static CAT_TOKEN_2_2_WHITE: CardRecord = CardRecord::new(
    cards::CAT_TOKEN_2_2_WHITE,
    "Cat",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Cat"], 2, 2).printed_colors(&[ManaColor::White]),
);

pub(in crate::card::sets) static CAT_TOKEN_1_1_WHITE: CardRecord = CardRecord::new(
    cards::CAT_TOKEN_1_1_WHITE,
    "Cat",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Cat"], 1, 1).printed_colors(&[ManaColor::White]),
);

pub(in crate::card::sets) static CAT_WARRIOR_TOKEN_2_1_WHITE: CardRecord = CardRecord::new(
    cards::CAT_WARRIOR_TOKEN_2_1_WHITE,
    "Cat Warrior",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Cat", "Warrior"], 2, 1)
        .printed_colors(&[ManaColor::White]),
);

pub(in crate::card::sets) static THRULL_TOKEN_0_1_BLACK: CardRecord = CardRecord::new(
    cards::THRULL_TOKEN_0_1_BLACK,
    "Thrull",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Thrull"], 0, 1)
        .printed_colors(&[ManaColor::Black]),
);

pub(in crate::card::sets) static WASP_TOKEN_1_1_COLORLESS: CardRecord = CardRecord::new(
    cards::WASP_TOKEN_1_1_COLORLESS,
    "Wasp",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Insect"], 1, 1)
        .with_type(CardType::Artifact)
        .with_ability(abilities::flying()),
);

pub(in crate::card::sets) static MINOR_DEMON_TOKEN_1_1_BLACK_RED: CardRecord = CardRecord::new(
    cards::MINOR_DEMON_TOKEN_1_1_BLACK_RED,
    "Minor Demon",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Demon"], 1, 1)
        .printed_colors(&[ManaColor::Black, ManaColor::Red]),
);

pub(in crate::card::sets) static OTTER_TOKEN_1_1_BLUE_RED: CardRecord = CardRecord::new(
    cards::OTTER_TOKEN_1_1_BLUE_RED,
    "Otter",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Otter"], 1, 1)
        .printed_colors(&[ManaColor::Blue, ManaColor::Red])
        .with_abilities(&[abilities::prowess()]),
);

pub(in crate::card::sets) static WURM_TOKEN_5_5_GREEN: CardRecord = CardRecord::new(
    cards::WURM_TOKEN_5_5_GREEN,
    "Wurm",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Wurm"], 5, 5)
        .printed_colors(&[ManaColor::Green])
        .with_ability(abilities::trample()),
);

pub(in crate::card::sets) static CENTAUR_TOKEN_3_3_GREEN: CardRecord = CardRecord::new(
    cards::CENTAUR_TOKEN_3_3_GREEN,
    "Centaur",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Centaur"], 3, 3)
        .printed_colors(&[ManaColor::Green]),
);

pub(in crate::card::sets) static RHINO_TOKEN_4_4_GREEN: CardRecord = CardRecord::new(
    cards::RHINO_TOKEN_4_4_GREEN,
    "Rhino",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Rhino"], 4, 4)
        .printed_colors(&[ManaColor::Green])
        .with_ability(abilities::trample()),
);

pub(in crate::card::sets) static ZOMBIE_TOKEN_2_2_BLACK: CardRecord = CardRecord::new(
    cards::ZOMBIE_TOKEN_2_2_BLACK,
    "Zombie",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Zombie"], 2, 2)
        .printed_colors(&[ManaColor::Black]),
);

pub(in crate::card::sets) static HUMAN_TOKEN_1_1_WHITE: CardRecord = CardRecord::new(
    cards::HUMAN_TOKEN_1_1_WHITE,
    "Human",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Human"], 1, 1).printed_colors(&[ManaColor::White]),
);

pub(in crate::card::sets) static ANGEL_TOKEN_4_4_WHITE: CardRecord = CardRecord::new(
    cards::ANGEL_TOKEN_4_4_WHITE,
    "Angel",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Angel"], 4, 4)
        .printed_colors(&[ManaColor::White])
        .with_ability(abilities::flying()),
);

pub(in crate::card::sets) static SPIRIT_TOKEN_1_1_BLUE: CardRecord = CardRecord::new(
    cards::SPIRIT_TOKEN_1_1_BLUE,
    "Spirit",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Spirit"], 1, 1)
        .printed_colors(&[ManaColor::Blue])
        .with_ability(abilities::flying()),
);

pub(in crate::card::sets) static HOMUNCULUS_TOKEN_2_2_BLUE: CardRecord = CardRecord::new(
    cards::HOMUNCULUS_TOKEN_2_2_BLUE,
    "Homunculus",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Homunculus"], 2, 2)
        .printed_colors(&[ManaColor::Blue]),
);

pub(in crate::card::sets) static SPIDER_TOKEN_1_2_GREEN: CardRecord = CardRecord::new(
    cards::SPIDER_TOKEN_1_2_GREEN,
    "Spider",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Spider"], 1, 2)
        .printed_colors(&[ManaColor::Green])
        .with_ability(abilities::reach()),
);

pub(in crate::card::sets) static SOLDIER_TOKEN_1_1_WHITE: CardRecord = CardRecord::new(
    cards::SOLDIER_TOKEN_1_1_WHITE,
    "Soldier",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Soldier"], 1, 1)
        .printed_colors(&[ManaColor::White]),
);

pub(in crate::card::sets) static DRAKE_TOKEN_2_2_BLUE: CardRecord = CardRecord::new(
    cards::DRAKE_TOKEN_2_2_BLUE,
    "Drake",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Drake"], 2, 2)
        .printed_colors(&[ManaColor::Blue])
        .with_ability(abilities::flying()),
);

pub(in crate::card::sets) static GOBLIN_TOKEN_1_1_RED: CardRecord = CardRecord::new(
    cards::GOBLIN_TOKEN_1_1_RED,
    "Goblin",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Goblin"], 1, 1).printed_colors(&[ManaColor::Red]),
);

/// The same body as the plain Goblin next door, with the haste Rabblemaster
/// prints on it: a token made at the beginning of combat is worth nothing
/// without it.
pub(in crate::card::sets) static GOBLIN_TOKEN_1_1_RED_HASTE: CardRecord = CardRecord::new(
    cards::GOBLIN_TOKEN_1_1_RED_HASTE,
    "Goblin",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Goblin"], 1, 1)
        .printed_colors(&[ManaColor::Red])
        .with_ability(abilities::haste()),
);

static A_CREATURE_YOU_CONTROL: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::HasType(CardType::Creature),
        zones: &[ZoneKind::Battlefield],
        controller: Some(PlayerRelation::You),
        owner: None,
    },
)];

static MAP_COST: [AbilityCostDef; 3] = [
    AbilityCostDef::Mana(mana_cost!("{1}")),
    AbilityCostDef::TapSource,
    AbilityCostDef::SacrificeSource,
];

/// The Map is the whole of what Get Lost gives back, so it carries its
/// printed ability rather than being a blank artifact: a card off the top
/// when it is a land, and a bigger creature when it is not.
static TREASURE_COST: [AbilityCostDef; 2] =
    [AbilityCostDef::TapSource, AbilityCostDef::SacrificeSource];

/// A Treasure is one mana of any colour that has to be spent to be spent:
/// tapping it is only half the cost, and the token goes with it.
pub(in crate::card::sets) static TREASURE_TOKEN: CardRecord = CardRecord::new(
    cards::TREASURE_TOKEN,
    "Treasure",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_artifact_without_mana_cost(&["Treasure"]).with_ability(
        AbilityDef::activated_mana(
            "{T}, Sacrifice this artifact: Add one mana of any color.",
            &TREASURE_COST,
            EffectDef::AddMana(AddManaEffectDef::any_color()),
        ),
    ),
);

pub(in crate::card::sets) static ELEMENTAL_TOKEN_5_3_GREEN: CardRecord = CardRecord::new(
    cards::ELEMENTAL_TOKEN_5_3_GREEN,
    "Elemental",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Elemental"], 5, 3)
        .printed_colors(&[ManaColor::Green]),
);

pub(in crate::card::sets) static MAP_TOKEN: CardRecord = CardRecord::new(
    cards::MAP_TOKEN,
    "Map",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_artifact_without_mana_cost(&[]).with_ability(
        AbilityDef::activated_with_targets(
            "{1}, {T}, Sacrifice this token: Target creature you control explores.",
            &MAP_COST,
            &A_CREATURE_YOU_CONTROL,
            EffectDef::Explore {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            },
        )
        .with_activation_timing(ActivationTimingDef::SorcerySpeed),
    ),
);

pub(in crate::card::sets) static WARRIOR_TOKEN_1_1_RED: CardRecord = CardRecord::new(
    cards::WARRIOR_TOKEN_1_1_RED,
    "Warrior",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Warrior"], 1, 1).printed_colors(&[ManaColor::Red]),
);

pub(in crate::card::sets) static SPIRIT_TOKEN_1_1_WHITE_BLACK: CardRecord = CardRecord::new(
    cards::SPIRIT_TOKEN_1_1_WHITE_BLACK,
    "Spirit",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Spirit"], 1, 1)
        .printed_colors(&[ManaColor::White, ManaColor::Black])
        .with_ability(abilities::flying()),
);

pub(in crate::card::sets) static SLIVER_TOKEN_1_1_COLORLESS: CardRecord = CardRecord::new(
    cards::SLIVER_TOKEN_1_1_COLORLESS,
    "Sliver",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Sliver"], 1, 1),
);

pub(in crate::card::sets) static DRAGON_TOKEN_2_2_RED: CardRecord = CardRecord::new(
    cards::DRAGON_TOKEN_2_2_RED,
    "Dragon",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Dragon"], 2, 2)
        .printed_colors(&[ManaColor::Red])
        .with_ability(abilities::flying())
        .with_ability(AbilityDef::activated(
            "{R}: This creature gets +1/+0 until end of turn.",
            &[crate::card::AbilityCostDef::Mana(crate::mana_cost!("{R}"))],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(1),
                    ValueDef::Constant(0),
                ),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        )),
);

pub(in crate::card::sets) static ELEMENTAL_TOKEN_1_1_RED: CardRecord = CardRecord::new(
    cards::ELEMENTAL_TOKEN_1_1_RED,
    "Elemental",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Elemental"], 1, 1)
        .printed_colors(&[ManaColor::Red]),
);

pub(in crate::card::sets) static SAPROLING_TOKEN_1_1_GREEN: CardRecord = CardRecord::new(
    cards::SAPROLING_TOKEN_1_1_GREEN,
    "Saproling",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Saproling"], 1, 1)
        .printed_colors(&[ManaColor::Green]),
);

pub(in crate::card::sets) static BIRD_TOKEN_1_1_WHITE: CardRecord = CardRecord::new(
    cards::BIRD_TOKEN_1_1_WHITE,
    "Bird",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Bird"], 1, 1)
        .printed_colors(&[ManaColor::White])
        .with_ability(abilities::flying()),
);

pub(in crate::card::sets) static DRAGON_TOKEN_6_6_RED: CardRecord = CardRecord::new(
    cards::DRAGON_TOKEN_6_6_RED,
    "Dragon",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Dragon"], 6, 6)
        .printed_colors(&[ManaColor::Red])
        .with_ability(abilities::flying()),
);

pub(in crate::card::sets) static WURM_TOKEN_6_6_GREEN: CardRecord = CardRecord::new(
    cards::WURM_TOKEN_6_6_GREEN,
    "Wurm",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Wurm"], 6, 6).printed_colors(&[ManaColor::Green]),
);

/// Any permanent at all, read off what the spell already targets. A spell
/// that points at a player as well is still a spell that targets a permanent.
static ANY_PERMANENT: ObjectPredicateDef = ObjectPredicateDef::Any;

/// Dack's emblem. The clause reads the targets a spell has already chosen, so
/// the theft happens before the spell resolves: the removal you pointed at
/// their creature kills a creature you now own.
pub(in crate::card::sets) static DACK_FAYDEN_EMBLEM: CardRecord = CardRecord::new(
    cards::DACK_FAYDEN_EMBLEM,
    "Dack Fayden emblem",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_emblem().with_ability(AbilityDef::triggered(
        "Whenever you cast a spell that targets one or more permanents, gain control of those \
         permanents.",
        TriggerEventDef::SpellCast(ObjectPredicateDef::All(&[
            ObjectPredicateDef::ControlledBy(PlayerRelation::You),
            ObjectPredicateDef::TargetsObjectMatching(&ANY_PERMANENT),
        ])),
        EffectDef::GainControl {
            object: EffectRecipientDef::objects(ObjectSetDef::PermanentsTargetedBy(
                ObjectRefDef::TriggeringObject,
            )),
            controller: PlayerRefDef::EffectController,
            duration: ControlDurationDef::Indefinitely,
        },
    )),
);

static CHANDRA_EMBLEM_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::AnyTarget,
)];

/// Chandra's emblem. Five damage per spell is a clock nobody outruns, so the
/// only question the emblem ever asks is where to point it.
pub(in crate::card::sets) static CHANDRA_TORCH_OF_DEFIANCE_EMBLEM: CardRecord = CardRecord::new(
    cards::CHANDRA_TORCH_OF_DEFIANCE_EMBLEM,
    "Chandra, Torch of Defiance emblem",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_emblem().with_ability(AbilityDef::triggered_with_targets(
        "Whenever you cast a spell, this emblem deals 5 damage to any target.",
        TriggerEventDef::SpellCast(ObjectPredicateDef::ControlledBy(PlayerRelation::You)),
        &CHANDRA_EMBLEM_TARGETS,
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(5),
        },
    )),
);

/// Domri's emblem. An emblem is an object with abilities and no other
/// characteristics, so it is cataloged like a token and lives in its own
/// list rather than on the battlefield.
pub(in crate::card::sets) static DOMRI_RADE_EMBLEM: CardRecord = CardRecord::new(
    cards::DOMRI_RADE_EMBLEM,
    "Domri Rade emblem",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_emblem().with_ability(AbilityDef::static_ability(
        "Creatures you control have double strike, trample, hexproof, and haste.",
        EffectDef::StaticApply {
            recipient: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::HasType(CardType::Creature),
                &[ZoneKind::Battlefield],
                PlayerRelation::You,
            ),
            effect: AppliedEffectDef::Composite(&DOMRI_EMBLEM_KEYWORDS),
        },
    )),
);

static DOMRI_EMBLEM_KEYWORDS: [AppliedEffectDef; 4] = [
    AppliedEffectDef::add_ability(&DOMRI_DOUBLE_STRIKE),
    AppliedEffectDef::add_ability(&DOMRI_TRAMPLE),
    AppliedEffectDef::add_ability(&DOMRI_HEXPROOF),
    AppliedEffectDef::add_ability(&DOMRI_HASTE),
];

static DOMRI_DOUBLE_STRIKE: AbilityDef = abilities::double_strike();
static DOMRI_TRAMPLE: AbilityDef = abilities::trample();
static DOMRI_HEXPROOF: AbilityDef = abilities::hexproof();
static DOMRI_HASTE: AbilityDef = abilities::haste();

/// Serpent Generator's Snake, which carries the poison trigger the artifact
/// prints in quotation marks rather than an ability of its own.
pub(in crate::card::sets) static SNAKE_TOKEN_1_1_POISONOUS: CardRecord = CardRecord::new(
    cards::SNAKE_TOKEN_1_1_POISONOUS,
    "Snake",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Snake"], 1, 1)
        .with_type(CardType::Artifact)
        .with_ability(abilities::poisonous_damage(
            1,
            "Whenever this creature deals damage to a player, that player gets a poison counter.",
        )),
);

pub(in crate::card::sets) static RAT_TOKEN_1_1_BLACK: CardRecord = CardRecord::new(
    cards::RAT_TOKEN_1_1_BLACK,
    "Rat",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Rat"], 1, 1).printed_colors(&[ManaColor::Black]),
);

/// The first token here that is not a creature. Food is an artifact type
/// rather than a creature type, and the token is nothing but the ability
/// printed in its own reminder text.
pub(in crate::card::sets) static FOOD_TOKEN: CardRecord = CardRecord::new(
    cards::FOOD_TOKEN,
    "Food",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_artifact_without_mana_cost(&["Food"]).with_ability(AbilityDef::activated(
        "{2}, {T}, Sacrifice this token: You gain 3 life.",
        &[
            AbilityCostDef::Mana(mana_cost!("{2}")),
            AbilityCostDef::TapSource,
            AbilityCostDef::SacrificeSource,
        ],
        EffectDef::GainLife {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(3),
        },
    )),
);

/// The one token in the catalog with two faces. Incubate makes it, the
/// counters on it come from whatever made it, and two mana turns it over
/// into a body the size of those counters -- a 0/0 with the counters still
/// on it, which is the whole of what the back face is for.
const fn incubator_front_rules() -> CardRules {
    CardRules::new_artifact_without_mana_cost(&["Incubator"]).with_ability(AbilityDef::activated(
        "{2}: Transform this token.",
        &INCUBATOR_TRANSFORM_COST,
        EffectDef::Transform {
            object: EffectRecipientDef::Source,
        },
    ))
}

static INCUBATOR_TRANSFORM_COST: [AbilityCostDef; 1] = [AbilityCostDef::Mana(mana_cost!("{2}"))];

fn incubator_composition() -> CardComposition {
    CardComposition {
        parts: vec![
            CardPart::new(CardPartId::PRIMARY, "Incubator", incubator_front_rules()),
            CardPart::new(
                CardPartId(1),
                "Phyrexian",
                CardRules::new_artifact_creature_without_mana_cost(&["Phyrexian"], 0, 0),
            ),
        ],
        structure: CardStructure::DoubleFaced {
            front: CardPartId::PRIMARY,
            back: CardPartId(1),
            kind: DoubleFacedKind::Transforming,
        },
        // A token is never cast, so it has nothing to be cast as.
        play_options: Vec::new(),
    }
}

pub(in crate::card::sets) static INCUBATOR_TOKEN: CardRecord = CardRecord::new(
    cards::INCUBATOR_TOKEN,
    "Incubator",
    CardArt::new("", ""),
    CardSet::Token,
    incubator_front_rules(),
)
.with_composition(incubator_composition);

/// The body Gut makes out of anything else you were done with. Menace is
/// what makes a 4/1 that arrives already attacking hard to answer.
pub(in crate::card::sets) static SKELETON_TOKEN_4_1_BLACK: CardRecord = CardRecord::new(
    cards::SKELETON_TOKEN_4_1_BLACK,
    "Skeleton",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_creature_without_mana_cost(&["Skeleton"], 4, 1)
        .printed_colors(&[ManaColor::Black])
        .with_ability(abilities::menace()),
);

/// What investigating makes. Unlike Food it does not tap to be spent, so a
/// Clue made this turn is already a card.
pub(in crate::card::sets) static CLUE_TOKEN: CardRecord = CardRecord::new(
    cards::CLUE_TOKEN,
    "Clue",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_artifact_without_mana_cost(&["Clue"]).with_ability(AbilityDef::activated(
        "{2}, Sacrifice this token: Draw a card.",
        &[
            AbilityCostDef::Mana(mana_cost!("{2}")),
            AbilityCostDef::SacrificeSource,
        ],
        EffectDef::DrawCards {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
        },
    )),
);

/// The other noncreature token here, and the one whose ability spends a card
/// to find a better one.
pub(in crate::card::sets) static BLOOD_TOKEN: CardRecord = CardRecord::new(
    cards::BLOOD_TOKEN,
    "Blood",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_artifact_without_mana_cost(&["Blood"]).with_ability(AbilityDef::activated(
        "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.",
        &[
            AbilityCostDef::Mana(mana_cost!("{1}")),
            AbilityCostDef::TapSource,
            AbilityCostDef::DiscardCardMatching(ObjectPredicateDef::Any),
            AbilityCostDef::SacrificeSource,
        ],
        EffectDef::DrawCards {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
        },
    )),
);

static NISSA_LANDS_ARE_INDESTRUCTIBLE: AbilityDef = abilities::indestructible();

/// Nissa's emblem. The lands it protects are the ones her +1 keeps turning
/// into creatures, which is why an indestructible clause reads as removal
/// protection rather than as a board wipe answer.
pub(in crate::card::sets) static NISSA_WHO_SHAKES_THE_WORLD_EMBLEM: CardRecord = CardRecord::new(
    cards::NISSA_WHO_SHAKES_THE_WORLD_EMBLEM,
    "Nissa, Who Shakes the World emblem",
    CardArt::new("", ""),
    CardSet::Token,
    CardRules::new_emblem().with_ability(AbilityDef::static_ability(
        "Lands you control have indestructible.",
        EffectDef::StaticApply {
            recipient: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::HasType(CardType::Land),
                &[ZoneKind::Battlefield],
                PlayerRelation::You,
            ),
            effect: AppliedEffectDef::add_ability(&NISSA_LANDS_ARE_INDESTRUCTIBLE),
        },
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &FACE_DOWN_CREATURE,
    &INCUBATOR_TOKEN,
    &GERM_TOKEN_0_0_BLACK,
    &ORC_ARMY_TOKEN_0_0_BLACK,
    &BEAST_TOKEN_3_3_GREEN,
    &KNIGHT_TOKEN_2_2_WHITE,
    &SOLDIER_TOKEN_1_1_RED_WHITE,
    &DEMON_TOKEN_5_5_BLACK,
    &ELEMENTAL_TOKEN_GREEN_WHITE,
    &SPIRIT_TOKEN_1_1_WHITE,
    &WOLF_TOKEN_2_2_GREEN,
    &WOLVES_OF_THE_HUNT_TOKEN_1_1_GREEN,
    &WOLF_TOKEN_1_1_BLACK,
    &CHANDRA_TORCH_OF_DEFIANCE_EMBLEM,
    &DACK_FAYDEN_EMBLEM,
    &DOMRI_RADE_EMBLEM,
    &NISSA_WHO_SHAKES_THE_WORLD_EMBLEM,
    &DJINN_TOKEN_5_5_COLORLESS,
    &RABBIT_TOKEN_1_1_WHITE,
    &SERVO_TOKEN_1_1_COLORLESS,
    &GOLEM_TOKEN_3_3_COLORLESS,
    &TETRAVITE_TOKEN,
    &ASSASSIN_TOKEN_1_1_BLACK,
    &BIRD_TOKEN_4_4_RED,
    &CITIZEN_TOKEN_1_1_WHITE,
    &CAT_TOKEN_2_2_WHITE,
    &CAT_TOKEN_1_1_WHITE,
    &CAT_WARRIOR_TOKEN_2_1_WHITE,
    &THRULL_TOKEN_0_1_BLACK,
    &WASP_TOKEN_1_1_COLORLESS,
    &MINOR_DEMON_TOKEN_1_1_BLACK_RED,
    &OTTER_TOKEN_1_1_BLUE_RED,
    &WURM_TOKEN_5_5_GREEN,
    &CENTAUR_TOKEN_3_3_GREEN,
    &RHINO_TOKEN_4_4_GREEN,
    &ZOMBIE_TOKEN_2_2_BLACK,
    &HUMAN_TOKEN_1_1_WHITE,
    &ANGEL_TOKEN_4_4_WHITE,
    &SPIRIT_TOKEN_1_1_BLUE,
    &HOMUNCULUS_TOKEN_2_2_BLUE,
    &SPIDER_TOKEN_1_2_GREEN,
    &SOLDIER_TOKEN_1_1_WHITE,
    &DRAKE_TOKEN_2_2_BLUE,
    &GOBLIN_TOKEN_1_1_RED,
    &GOBLIN_TOKEN_1_1_RED_HASTE,
    &TREASURE_TOKEN,
    &ELEMENTAL_TOKEN_5_3_GREEN,
    &MAP_TOKEN,
    &WARRIOR_TOKEN_1_1_RED,
    &SPIRIT_TOKEN_1_1_WHITE_BLACK,
    &SLIVER_TOKEN_1_1_COLORLESS,
    &DRAGON_TOKEN_2_2_RED,
    &ELEMENTAL_TOKEN_1_1_RED,
    &SAPROLING_TOKEN_1_1_GREEN,
    &BIRD_TOKEN_1_1_WHITE,
    &DRAGON_TOKEN_6_6_RED,
    &SNAKE_TOKEN_1_1_POISONOUS,
    &WURM_TOKEN_6_6_GREEN,
    &RAT_TOKEN_1_1_BLACK,
    &FOOD_TOKEN,
    &CLUE_TOKEN,
    &SKELETON_TOKEN_4_1_BLACK,
    &BLOOD_TOKEN,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
