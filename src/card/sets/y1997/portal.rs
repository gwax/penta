//! Portal card records.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::AbilityDef;
use crate::AbilityTargetDef;
use crate::AbilityTargetPredicate;
use crate::AppliedEffectDef;
use crate::AppliedRuleDef;
use crate::CardArt;
use crate::CardRules;
use crate::CardSet;
use crate::CardType;
use crate::DiscardSelectionDef;
use crate::EffectDef;
use crate::EffectRecipientDef;
use crate::ObjectPredicateDef;
use crate::PlayerRelation;
use crate::TargetIndex;
use crate::ValueDef;
use crate::ZoneKind;
use crate::ZonePlacement;
use crate::card::abilities;
use crate::{BasicLandType, ObjectRefDef, PlayerRefDef, ResolvedEffectDurationDef};

use crate::mana_cost;

// POR 1 — Alabaster Dragon
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static ALABASTER_DRAGON: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("1edc6ec1-3b34-45e0-8573-39eba1d10efa"),
    "Alabaster Dragon",
    crate::card::CardArt::new("3a2fcc23-ac09-4ada-b194-424739c9c734", "Bob Eggleton"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 2 — Angelic Blessing
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static ANGELIC_BLESSING: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("31dda640-2a00-437e-855f-173c487e7395"),
    "Angelic Blessing",
    crate::card::CardArt::new("ed3c8bae-953f-4bb4-a78d-02e4e354e53c", "Mark Zug"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 4 — Ardent Militia
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static ARDENT_MILITIA: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("543f8c6a-bcf1-4400-82e5-83d36cb60464"),
    "Ardent Militia",
    crate::card::CardArt::new("bb212ca5-bbb5-4c83-9a7b-9d5ab451e032", "Zina Saunders"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 6 — Armored Pegasus
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static ARMORED_PEGASUS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("3f021a79-a182-4914-9ff4-d6fcba7c1d22"),
    "Armored Pegasus",
    crate::card::CardArt::new("012049f8-0936-49ed-948d-0d34af28550f", "Una Fricker"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 7 — Blessed Reversal
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static BLESSED_REVERSAL: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("899ecc19-8106-4e5a-bb25-aaea9684ba0e"),
    "Blessed Reversal",
    crate::card::CardArt::new("3fb6d738-f6a8-4626-8103-68e63874eda4", "Pete Venters"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 10 — Breath of Life
pub(in crate::card::sets) static BREATH_OF_LIFE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("bcea5e09-6385-41df-970b-ac26c9b46127"),
    "Breath of Life",
    crate::card::CardArt::new("a10f24f7-f82e-413e-824f-384607c7d858", "Lubov"),
    crate::card::CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{3}{W}")).with_ability(AbilityDef::spell_with_targets(
        "Return target creature card from your graveyard to the battlefield.",
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
            zone: ZoneKind::Battlefield,
            placement: ZonePlacement::Top,
        },
    )),
);

// POR 11 — Charging Paladin
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static CHARGING_PALADIN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("29db1bbf-a6cf-460c-bec8-dbd682157af4"),
    "Charging Paladin",
    crate::card::CardArt::new("851f3f72-2923-4432-898a-02679a8b320f", "Ciruelo"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 20 — Knight Errant
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static KNIGHT_ERRANT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("9c31b4b4-18fc-4a6e-8d74-fd5340964320"),
    "Knight Errant",
    crate::card::CardArt::new("413f10fe-0e53-46ca-bd64-0d66dee8882d", "Matthew D. Wilson"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 21 — Path of Peace
pub(in crate::card::sets) static PATH_OF_PEACE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("a1f3e1c9-bfad-49a1-b171-6fa344ef2eef"),
    "Path of Peace",
    crate::card::CardArt::new("cb14d3f4-09f3-4113-bdc3-0fd753137f7c", "David A. Cherry"),
    crate::card::CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{3}{W}")).with_ability(AbilityDef::spell_with_targets(
        "Destroy target creature. Its owner gains 4 life.",
        &[AbilityTargetDef::exactly_one_permanent(
            ObjectPredicateDef::HasType(CardType::Creature),
        )],
        EffectDef::Sequence(&[
            EffectDef::Destroy {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                can_regenerate: true,
                then: None,
            },
            EffectDef::GainLife {
                recipient: EffectRecipientDef::player(PlayerRefDef::OwnerOf(ObjectRefDef::Target(
                    TargetIndex::PRIMARY,
                ))),
                amount: ValueDef::Constant(4),
            },
        ]),
    )),
);

// POR 22 — Regal Unicorn
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static REGAL_UNICORN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("daa1fb8c-12fa-4e9c-979f-55e89356acaf"),
    "Regal Unicorn",
    crate::card::CardArt::new("54ca9b1c-fead-4bb6-800f-8b762a82fda7", "Zina Saunders"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 25 — Sacred Nectar
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SACRED_NECTAR: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("484d1b31-5363-49ef-9b13-2005568636c1"),
    "Sacred Nectar",
    crate::card::CardArt::new("8d4b8de0-0bb5-40fb-8b73-d00d38a582d5", "Dana Knutson"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 26 — Seasoned Marshal
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SEASONED_MARSHAL: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("17db0060-3667-4c8c-ae9b-d62dceac64e3"),
    "Seasoned Marshal",
    crate::card::CardArt::new("9de20845-06b7-4542-8d61-4b97309669f9", "Matthew D. Wilson"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 29 — Starlight
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static STARLIGHT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f6992524-6921-473b-8301-cb63fe502600"),
    "Starlight",
    crate::card::CardArt::new("413c5a7e-e19d-4cbd-9279-88391b75c6c5", "Brian Despain"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 35 — Venerable Monk
pub(in crate::card::sets) static VENERABLE_MONK: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("72322032-c287-4a9e-9d61-a452f6c45bfb"),
    "Venerable Monk",
    crate::card::CardArt::new("704b8be3-4ed8-4e94-aa66-c7187a299088", "Terese Nielsen"),
    crate::card::CardSet::Portal,
    CardRules::new_creature(mana_cost!("{2}{W}"), &["Human", "Monk", "Cleric"], 2, 2).with_ability(
        abilities::enters_trigger(
            "When this creature enters, you gain 2 life.",
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(2),
            },
        ),
    ),
);

// POR 36 — Vengeance
pub(in crate::card::sets) static VENGEANCE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("c91c249b-157c-4f1d-8171-29d1e75b1c9f"),
    "Vengeance",
    crate::card::CardArt::new("3209ee48-4485-44fc-b71d-cd6241674e64", "Keith Parkinson"),
    crate::card::CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{3}{W}")).with_ability(AbilityDef::destroy_target(
        "Destroy target tapped creature.",
        &AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::All(&[
            ObjectPredicateDef::HasType(CardType::Creature),
            ObjectPredicateDef::Tapped,
        ])),
        true,
    )),
);

// POR 42 — Baleful Stare
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static BALEFUL_STARE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("49fb46c8-30ae-4457-a726-6fe1ddd183d5"),
    "Baleful Stare",
    crate::card::CardArt::new("7c53b808-c2c5-4941-bead-1cb94adc5a2f", "Eric Peterson"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 47 — Cloud Spirit
pub(in crate::card::sets) static CLOUD_SPIRIT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("cc7547aa-fcf7-4b6e-955d-cc5ebc40cd7d"),
    "Cloud Spirit",
    crate::card::CardArt::new("938d6c51-903b-4e0b-8702-291666581f2a", "Randy Gallegos"),
    crate::card::CardSet::Portal,
    CardRules::new_creature(mana_cost!("{2}{U}"), &["Spirit"], 3, 1).with_abilities(&[
        abilities::flying(),
        AbilityDef::static_ability(
            "This creature can block only creatures with flying.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::can_block_only(
                    ObjectPredicateDef::HasKeyword(crate::card::KeywordAbility::Flying),
                )),
            },
        ),
    ]),
);

// POR 54 — Exhaustion
pub(in crate::card::sets) static EXHAUSTION: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("9d6a5c33-cf74-4cec-a4f4-1aac9e7b8f79"),
    "Exhaustion",
    crate::card::CardArt::new("fcc103a6-7888-4e35-b35b-a796a48caf70", "Kaja Foglio"),
    crate::card::CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{2}{U}")).with_ability(AbilityDef::spell_with_targets(
        "Creatures and lands target opponent controls don't untap during their next untap step.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Player(PlayerRelation::Opponent),
        )],
        EffectDef::SkipNextUntapSteps {
            object: EffectRecipientDef::objects_controlled_by_target(
                ObjectPredicateDef::AnyOf(&[
                    ObjectPredicateDef::HasType(CardType::Creature),
                    ObjectPredicateDef::HasType(CardType::Land),
                ]),
                TargetIndex::PRIMARY,
            ),
            count: 1,
        },
    )),
);

// POR 55 — Flux
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static FLUX: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("3c26bf66-8fa8-4f69-9556-c9fcc56a7f33"),
    "Flux",
    crate::card::CardArt::new(
        "368b28e4-a367-4a38-866d-c3768bd9b7ad",
        "Richard Kane Ferguson",
    ),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 56 — Giant Octopus
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static GIANT_OCTOPUS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("4528edca-cc36-4f63-9615-24ca315d672c"),
    "Giant Octopus",
    crate::card::CardArt::new("5b707b2d-63e1-4c2c-ba42-9e027f02b1ff", "Heather Hudson"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 57 — Horned Turtle
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static HORNED_TURTLE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("a7d25497-36b4-48b9-ba01-f24f6222d6be"),
    "Horned Turtle",
    crate::card::CardArt::new("b2348ce1-6305-42a7-8061-64275f6dc5c6", "DiTerlizzi"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 65 — Phantom Warrior
pub(in crate::card::sets) static PHANTOM_WARRIOR: CardRecord = CardRecord::new_with_legacy_id(
    1169,
    "Phantom Warrior",
    CardArt::new("e12a1a64-5b32-4b85-8fae-c407d7926547", "Greg Staples"),
    CardSet::Portal,
    CardRules::new_creature(mana_cost!("{1}{U}{U}"), &["Illusion", "Warrior"], 2, 2).with_ability(
        AbilityDef::static_ability(
            "This creature can't be blocked.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::cannot_be_blocked_by(
                    ObjectPredicateDef::Any,
                )),
            },
        ),
    ),
);

// POR 72 — Theft of Dreams
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static THEFT_OF_DREAMS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("29019e28-4ef8-4732-9972-0a47305fe303"),
    "Theft of Dreams",
    crate::card::CardArt::new(
        "099da8aa-16b1-4395-8467-1636feb14a8a",
        "Richard Kane Ferguson",
    ),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 74 — Tidal Surge
pub(in crate::card::sets) static TIDAL_SURGE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("a027c31d-c662-4ce1-a0d1-a32e62f6a724"),
    "Tidal Surge",
    crate::card::CardArt::new("8737440b-0bf0-483f-895b-aa24da2b9cfe", "Doug Chaffee"),
    crate::card::CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{1}{U}")).with_ability(AbilityDef::spell_with_targets(
        "Tap up to three target creatures without flying.",
        &[AbilityTargetDef::up_to(
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Creature),
                    ObjectPredicateDef::Not(&ObjectPredicateDef::HasKeyword(
                        crate::card::KeywordAbility::Flying,
                    )),
                ]),
                zones: &[ZoneKind::Battlefield],
                controller: None,
                owner: None,
            },
            3,
        )],
        EffectDef::Tap {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        },
    )),
);

// POR 75 — Time Ebb
pub(in crate::card::sets) static TIME_EBB: CardRecord = CardRecord::new_with_legacy_id(
    1171,
    "Time Ebb",
    CardArt::new("bf0c48f6-8b2e-4eff-aa1e-10e6ccae426a", "Alan Rabinowitz"),
    CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{2}{U}")).with_ability(AbilityDef::spell_with_targets(
        "Put target creature on top of its owner's library.",
        &[AbilityTargetDef::exactly_one_permanent(
            ObjectPredicateDef::HasType(CardType::Creature),
        )],
        EffectDef::MoveToZone {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            zone: ZoneKind::Library,
            placement: ZonePlacement::Top,
        },
    )),
);

// POR 77 — Wind Drake
pub(in crate::card::sets) static WIND_DRAKE: CardRecord = CardRecord::new_with_legacy_id(
    618,
    "Wind Drake",
    CardArt::new("c9dcb8d2-0da9-40fc-b0c0-2c76b3d277bc", "Steve Prescott"),
    CardSet::Portal,
    CardRules::new_creature(mana_cost!("{2}{U}"), &["Drake"], 2, 2)
        .with_abilities(&[abilities::flying()]),
);

// POR 82 — Bog Raiders
pub(in crate::card::sets) static BOG_RAIDERS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("eb7bbb7a-b59a-4a01-b1cb-66eef881ffcd"),
    "Bog Raiders",
    crate::card::CardArt::new("3739188b-f2b3-4ab0-8e5c-b3a1d2a1ad09", "Carl Critchlow"),
    crate::card::CardSet::Portal,
    CardRules::new_creature(mana_cost!("{2}{B}"), &["Zombie"], 2, 2)
        .with_ability(abilities::landwalk(BasicLandType::Swamp)),
);

// POR 95 — Gravedigger
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static GRAVEDIGGER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("b979d70e-d514-420f-886c-f60e2bb1861f"),
    "Gravedigger",
    crate::card::CardArt::new("11055d4e-3efe-493c-8c18-9e2642267511", "Dermot Power"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 101 — Mind Rot
pub(in crate::card::sets) static MIND_ROT: CardRecord = CardRecord::new_with_legacy_id(
    1004,
    "Mind Rot",
    CardArt::new("ab454fb8-347f-4d4d-84bb-195c9d51b06b", "Steve Luke"),
    CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{2}{B}")).with_ability(AbilityDef::spell_with_targets(
        "Target player discards two cards.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::Player(PlayerRelation::Any),
        )],
        EffectDef::Discard {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(2),
            selection: DiscardSelectionDef::RecipientChooses,
            then: None,
        },
    )),
);

// POR 106 — Rain of Tears
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static RAIN_OF_TEARS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("803ba4ef-24ed-4f45-aed8-f9442322e31e"),
    "Rain of Tears",
    crate::card::CardArt::new("cad93919-273f-4a26-8ebd-13503dd6b220", "Charles Gillespie"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 109 — Serpent Warrior
pub(in crate::card::sets) static SERPENT_WARRIOR: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("c364fd06-64c5-45f6-8ed5-64f44a1e8bda"),
    "Serpent Warrior",
    crate::card::CardArt::new("ab726e7d-171f-48b2-9652-545e17913330", "Ron Spencer"),
    crate::card::CardSet::Portal,
    CardRules::new_creature(mana_cost!("{2}{B}"), &["Snake", "Warrior"], 3, 3).with_ability(
        abilities::enters_trigger(
            "When this creature enters, you lose 3 life.",
            EffectDef::LoseLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(3),
            },
        ),
    ),
);

// POR 118 — Blaze
pub(in crate::card::sets) static BLAZE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("04095ad2-7308-4e26-b9ef-070a5755d066"),
    "Blaze",
    crate::card::CardArt::new("3940d0ca-0ca2-4446-9330-a554c3e89824", "David A. Cherry"),
    crate::card::CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{X}{R}")).with_ability(AbilityDef::spell_with_targets(
        "Blaze deals X damage to any target.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::AnyTarget,
        )],
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::ChosenX,
        },
    )),
);

// POR 121 — Craven Giant
pub(in crate::card::sets) static CRAVEN_GIANT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("4a2e1c12-f848-43b4-9505-851c66a509f1"),
    "Craven Giant",
    crate::card::CardArt::new("ea3cf964-88f6-4e62-97ce-cf0e179a53fb", "Brian Snõddy"),
    crate::card::CardSet::Portal,
    CardRules::new_creature(mana_cost!("{2}{R}"), &["Giant"], 4, 1).with_ability(
        AbilityDef::static_ability(
            "This creature can't block.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::CANNOT_BLOCK),
            },
        ),
    ),
);

// POR 137 — Lava Axe
pub(in crate::card::sets) static LAVA_AXE: CardRecord = CardRecord::new_with_legacy_id(
    1203,
    "Lava Axe",
    CardArt::new("1c4f1041-8bbe-46fa-bbe4-40cd993f53a2", "Brian Snõddy"),
    CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{4}{R}")).with_ability(AbilityDef::spell_with_targets(
        "Lava Axe deals 5 damage to target player or planeswalker.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::PlayerOrPlaneswalker(PlayerRelation::Any),
        )],
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(5),
        },
    )),
);

// POR 145 — Raging Goblin
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static RAGING_GOBLIN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("6c0fa444-5534-4476-8bfa-78b2364f2dd3"),
    "Raging Goblin",
    crate::card::CardArt::new("1f0a166c-f7c0-45b4-aa90-053ce545cfb2", "Brian Snõddy"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 147 — Rain of Salt
pub(in crate::card::sets) static RAIN_OF_SALT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("661ffab2-9cf5-492d-874f-de73d7a13e2b"),
    "Rain of Salt",
    crate::card::CardArt::new("4792293a-e11d-4c5e-bbd9-6f09e69ee617", "Adam Rex"),
    crate::card::CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{4}{R}{R}")).with_ability(AbilityDef::spell_with_targets(
        "Destroy two target lands.",
        &[AbilityTargetDef::exactly_value(
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::HasType(CardType::Land),
                zones: &[ZoneKind::Battlefield],
                controller: None,
                owner: None,
            },
            ValueDef::Constant(2),
        )],
        EffectDef::Destroy {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            can_regenerate: true,
            then: None,
        },
    )),
);

// POR 152 — Thundermare
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static THUNDERMARE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("59a9f3f5-c80f-47a4-bf84-b7262437017f"),
    "Thundermare",
    crate::card::CardArt::new("e936e5cb-0a8e-4348-afea-e5f96b19fe23", "Bob Eggleton"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 154 — Volcanic Hammer
pub(in crate::card::sets) static VOLCANIC_HAMMER: CardRecord = CardRecord::new_with_legacy_id(
    273,
    "Volcanic Hammer",
    CardArt::new(
        "58c0489d-b073-4ad4-b044-447fcc865b6c",
        "Edward P. Beard, Jr.",
    ),
    CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{1}{R}")).with_ability(AbilityDef::spell_with_targets(
        "Volcanic Hammer deals 3 damage to any target.",
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::AnyTarget,
        )],
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(3),
        },
    )),
);

// POR 158 — Anaconda
pub(in crate::card::sets) static ANACONDA: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("0a2012ad-6425-4935-83af-fc7309ec2ece"),
    "Anaconda",
    crate::card::CardArt::new("1be798fd-18c9-45b0-8207-7e5e01c83f49", "Stephen Daniele"),
    crate::card::CardSet::Portal,
    CardRules::new_creature(mana_cost!("{3}{G}"), &["Snake"], 3, 3)
        .with_ability(abilities::landwalk(BasicLandType::Swamp)),
);

// POR 160 — Bull Hippo
pub(in crate::card::sets) static BULL_HIPPO: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("1fbe115b-ded7-4749-95e2-b69bff26fc74"),
    "Bull Hippo",
    crate::card::CardArt::new("1d1f8259-1825-4a46-8026-75adc4480322", "Daren Bader"),
    crate::card::CardSet::Portal,
    CardRules::new_creature(mana_cost!("{3}{G}"), &["Hippo"], 3, 3)
        .with_ability(abilities::landwalk(BasicLandType::Island)),
);

// POR 161 — Charging Rhino
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static CHARGING_RHINO: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("49e47248-051c-4ee6-aad2-352ebd1f38ca"),
    "Charging Rhino",
    crate::card::CardArt::new("651f89e5-9ce2-4713-aca9-6581005f6ca2", "Daren Bader"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 168 — Gorilla Warrior
pub(in crate::card::sets) static GORILLA_WARRIOR: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("38f9c3f3-0d4d-4eec-bd14-9be3233178dc"),
    "Gorilla Warrior",
    crate::card::CardArt::new("76c7e2b0-2df0-4cde-8565-762c93e6c14f", "Steve White"),
    crate::card::CardSet::Portal,
    CardRules::new_creature(mana_cost!("{2}{G}"), &["Ape", "Warrior"], 3, 2),
);

// POR 173 — Monstrous Growth
pub(in crate::card::sets) static MONSTROUS_GROWTH: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("0523c816-dddf-4b63-8db8-5e41dc673e5f"),
    "Monstrous Growth",
    crate::card::CardArt::new("3816da20-4434-4bf7-a9dd-3eb3bb735f08", "Una Fricker"),
    crate::card::CardSet::Portal,
    CardRules::new_sorcery(mana_cost!("{1}{G}")).with_ability(AbilityDef::spell_with_targets(
        "Target creature gets +4/+4 until end of turn.",
        &[AbilityTargetDef::exactly_one_permanent(
            ObjectPredicateDef::HasType(CardType::Creature),
        )],
        EffectDef::Apply {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            effect: AppliedEffectDef::modify_power_toughness(
                ValueDef::Constant(4),
                ValueDef::Constant(4),
            ),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    )),
);

// POR 176 — Natural Spring
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static NATURAL_SPRING: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("8ddfc1cc-5c13-443c-a0ae-0bcc931923e7"),
    "Natural Spring",
    crate::card::CardArt::new("1ff5d12a-8634-468b-86ca-4ba0f7c013ca", "Susan Van Camp"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 179 — Needle Storm
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static NEEDLE_STORM: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("29a44e44-94b1-4bd2-8e00-6bd2ec07ee4c"),
    "Needle Storm",
    crate::card::CardArt::new("be80dd2d-f595-4d80-84ae-66d3d18e7399", "Val Mayerik"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 183 — Redwood Treefolk
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static REDWOOD_TREEFOLK: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("e9399667-ae2a-4b64-84dd-8f97f3e5fe79"),
    "Redwood Treefolk",
    crate::card::CardArt::new("0274e162-33e4-4604-a6ea-51fc1a5c6a04", "Phil Foglio"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 185 — Spined Wurm
pub(in crate::card::sets) static SPINED_WURM: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("0053bd00-90fd-48c2-8f79-952d5d1e3e74"),
    "Spined Wurm",
    crate::card::CardArt::new("113fad70-36bc-4ab7-962a-cda3bddd02fc", "Keith Parkinson"),
    crate::card::CardSet::Portal,
    CardRules::new_creature(mana_cost!("{4}{G}"), &["Wurm"], 5, 4),
);

// POR 194 — Winter's Grasp
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static WINTER_S_GRASP: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("b2215de4-da49-4270-aec7-5e16a938bae4"),
    "Winter's Grasp",
    crate::card::CardArt::new("7af28a5d-45dc-4e31-9009-5c0bd25a9032", "Tom Wänerstrand"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

// POR 195 — Wood Elves
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static WOOD_ELVES: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("b7f1fb90-5c85-46a5-802d-248cc0250921"),
    "Wood Elves",
    crate::card::CardArt::new("4716bb55-0821-4809-9bc0-04e299b09549", "Rebecca Guay"),
    crate::card::CardSet::Portal,
    crate::card::CardRules::unsupported(),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &ALABASTER_DRAGON,
    &ANGELIC_BLESSING,
    &ARDENT_MILITIA,
    &ARMORED_PEGASUS,
    &BLESSED_REVERSAL,
    &BREATH_OF_LIFE,
    &CHARGING_PALADIN,
    &KNIGHT_ERRANT,
    &PATH_OF_PEACE,
    &REGAL_UNICORN,
    &SACRED_NECTAR,
    &SEASONED_MARSHAL,
    &STARLIGHT,
    &VENERABLE_MONK,
    &VENGEANCE,
    &BALEFUL_STARE,
    &CLOUD_SPIRIT,
    &EXHAUSTION,
    &FLUX,
    &GIANT_OCTOPUS,
    &HORNED_TURTLE,
    &PHANTOM_WARRIOR,
    &THEFT_OF_DREAMS,
    &TIDAL_SURGE,
    &TIME_EBB,
    &WIND_DRAKE,
    &BOG_RAIDERS,
    &GRAVEDIGGER,
    &MIND_ROT,
    &RAIN_OF_TEARS,
    &SERPENT_WARRIOR,
    &BLAZE,
    &CRAVEN_GIANT,
    &LAVA_AXE,
    &RAGING_GOBLIN,
    &RAIN_OF_SALT,
    &THUNDERMARE,
    &VOLCANIC_HAMMER,
    &ANACONDA,
    &BULL_HIPPO,
    &CHARGING_RHINO,
    &GORILLA_WARRIOR,
    &MONSTROUS_GROWTH,
    &NATURAL_SPRING,
    &NEEDLE_STORM,
    &REDWOOD_TREEFOLK,
    &SPINED_WURM,
    &WINTER_S_GRASP,
    &WOOD_ELVES,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
