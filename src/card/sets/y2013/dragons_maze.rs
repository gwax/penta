//! Dragon's Maze card records used by the built-in ISD–RTR Standard decks.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityImplementationDef, AbilityTargetDef, AbilityTargetPredicate,
    AppliedEffectDef, CardArt, CardBehavior, CardComposition, CardEffectStatus, CardPart,
    CardRules, CardSet, CardStructure, CardSupertype, CardType, EffectDef, EffectDurationDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, PlayOptionDef, PlayerRelation, SpellForm,
    TargetPredicate, TargetSlotDef, TriggerEventDef, TurnStepDef, ValueDef, ZoneKind, abilities,
    cards,
};
use crate::ids::{CardPartId, PlayOptionId, TargetSlotId};
use crate::mana_cost;

pub(in crate::card::sets) static AETHERLING: CardRecord = CardRecord::new(
    cards::AETHERLING,
    "Aetherling",
    CardArt::new("9c93313b-cf43-47e9-a911-717b4d14b0b5", "Tyler Jacobson"),
    CardSet::DragonsMaze,
    CardRules::new_creature(
        mana_cost!("{4}{U}{U}"),
        &["Shapeshifter"],
        4,
        5,
    )
    .with_abilities(&[
        AbilityDef::activated(
            "{U}: Exile this creature. Return it to the battlefield under its owner's control at the beginning of the next end step.",
            &[AbilityCostDef::Mana(mana_cost!("{U}"))],
            EffectDef::Sequence(&[
                EffectDef::ExileLinkedToSource {
                    object: EffectRecipientDef::Source,
                },
                // The next end step belongs to whoever's turn it is, which
                // may well be the opponent.
                EffectDef::AtNextStep {
                    step: TurnStepDef::End,
                    player: PlayerRelation::Any,
                    effect: &EffectDef::ReturnLinkedExiles {
                        zone: ZoneKind::Battlefield,
                        grant: None,
                    },
                },
            ]),
        ),
        AbilityDef::activated(
            "{U}: This creature can't be blocked this turn.",
            &[AbilityCostDef::Mana(mana_cost!("{U}"))],
            EffectDef::MakeUnblockableThisTurn {
                object: EffectRecipientDef::Source,
            },
        ),
        AbilityDef::activated(
            "{1}: This creature gets +1/-1 until end of turn.",
            &[AbilityCostDef::Mana(mana_cost!("{1}"))],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::ModifyPowerToughness {
                    power: ValueDef::Constant(1),
                    toughness: ValueDef::Constant(-1),
                },
                duration: EffectDurationDef::UntilEndOfTurn,
            },
        ),
        AbilityDef::activated(
            "{1}: This creature gets -1/+1 until end of turn.",
            &[AbilityCostDef::Mana(mana_cost!("{1}"))],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::ModifyPowerToughness {
                    power: ValueDef::Constant(-1),
                    toughness: ValueDef::Constant(1),
                },
                duration: EffectDurationDef::UntilEndOfTurn,
            },
        ),
    ]),
);

pub(in crate::card::sets) static BLOOD_BARON_OF_VIZKOPA: CardRecord = CardRecord::new(
    cards::BLOOD_BARON_OF_VIZKOPA,
    "Blood Baron of Vizkopa",
    CardArt::new("e4edad09-bf7b-40e9-ac2a-100da8a43274", "Anthony Palumbo"),
    CardSet::DragonsMaze,
    CardRules::new_creature(
        mana_cost!("{3}{W}{B}"),
        &["Vampire"],
        4,
        4,
    )
    .with_abilities(&[
        abilities::lifelink(),
        abilities::protection_from(ManaColor::White),
        abilities::protection_from(ManaColor::Black),
        AbilityDef::custom_full(
            "As long as you have 30 or more life and an opponent has 10 or less life, this creature gets +6/+6 and has flying.",
            CardBehavior::BloodBaronOfVizkopa,
            "The conditional power, toughness, and flying effect is implemented by the card-local static-effect hook.",
        ),
    ]),
);

pub(in crate::card::sets) static GAZE_OF_GRANITE: CardRecord = CardRecord::new(
    cards::GAZE_OF_GRANITE,
    "Gaze of Granite",
    CardArt::new("96c9ac10-d114-4aa5-87ac-f1069cde8e40", "Nils Hamm"),
    CardSet::DragonsMaze,
    CardRules::new_sorcery(mana_cost!("{X}{B}{B}{G}")).with_ability(AbilityDef::spell(
        "Destroy each nonland permanent with mana value X or less.",
        EffectDef::Destroy {
            object: EffectRecipientDef::MatchingObjects {
                object: ObjectPredicateDef::All(&[
                    ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Land)),
                    ObjectPredicateDef::ManaValueAtMostValue(ValueDef::ChosenX),
                ]),
                zones: &[ZoneKind::Battlefield],
                controller: PlayerRelation::Any,
            },
            can_regenerate: true,
        },
    )),
);

pub(in crate::card::sets) static PUTREFY: CardRecord = CardRecord::new(
    cards::PUTREFY,
    "Putrefy",
    CardArt::new("0d43a0b6-2a5c-4959-96ee-6e570949dfed", "Igor Kieryluk"),
    CardSet::DragonsMaze,
    CardRules::new_instant(mana_cost!("{1}{B}{G}")).with_ability(AbilityDef::custom_full(
        "Destroy target artifact or creature. It can't be regenerated.",
        CardBehavior::Putrefy,
        "Implemented by the named card-local special behavior.",
    )),
);

pub(in crate::card::sets) static RURIC_THAR_THE_UNBOWED: CardRecord = CardRecord::new(
    cards::RURIC_THAR_THE_UNBOWED,
    "Ruric Thar, the Unbowed",
    CardArt::new("84dd3586-7c3b-4f9c-a1eb-7745b75339b0", "Tyler Jacobson"),
    CardSet::DragonsMaze,
    CardRules::new_creature(
        mana_cost!("{4}{R}{G}"),
        &["Ogre", "Warrior"],
        6,
        6,
    )
    .with_supertype(CardSupertype::Legendary)
    .with_abilities(&[
        abilities::vigilance(),
        abilities::reach(),
        AbilityDef::not_implemented(
            "Ruric Thar attacks each combat if able.",
            "The attack requirement is not enforced.",
        ),
        AbilityDef::triggered(
            "Whenever a player casts a noncreature spell, Ruric Thar deals 6 damage to that player.",
            TriggerEventDef::SpellCast(ObjectPredicateDef::NoncreatureSpell),
            EffectDef::DealDamage {
                // Whoever cast it, which is what the event names; this hits
                // its own controller too.
                recipient: EffectRecipientDef::EventPlayer,
                amount: ValueDef::Constant(6),
            },
        ),
    ]),
);

pub(in crate::card::sets) static SIN_COLLECTOR: CardRecord = CardRecord::new(
    cards::SIN_COLLECTOR,
    "Sin Collector",
    CardArt::new("305a3feb-df49-486c-a3b4-ff2721d60019", "Mike Bierek"),
    CardSet::DragonsMaze,
    CardRules::new_creature(
        mana_cost!("{1}{W}{B}"),
        &["Human", "Cleric"],
        2,
        1,
    )
    .with_abilities(&[AbilityDef::triggered(
            "When this creature enters, target opponent reveals their hand. You choose an instant or sorcery card from it and exile that card.",
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
            behavior: Some(CardBehavior::SinCollector),
            explanation: "The targeted trigger uses the shared stack and a card-local hand-reveal and exile resolver.",
        }),
    ]),
);

const fn turn_rules() -> CardRules {
    CardRules::new_instant(mana_cost!("{2}{U}")).with_ability(
        AbilityDef::not_implemented(
            "Until end of turn, target creature loses all abilities and becomes a red Weird with base power and toughness 0/1.\nFuse (You may cast one or both halves of this card from your hand.)",
            "Printed rules are cataloged but are not executed by the engine.",
        ),
    )
}

fn turn_burn_composition() -> CardComposition {
    let turn = turn_rules();
    let burn = CardRules::new_instant(mana_cost!("{1}{R}")).with_ability(
        AbilityDef::not_implemented(
            "Burn deals 2 damage to any target.\nFuse (You may cast one or both halves of this card from your hand.)",
            "Printed rules are cataloged but are not executed by the engine.",
        ),
    );
    let turn_target = || {
        TargetSlotDef::exactly_one(
            TargetSlotId(0),
            "creature for Turn",
            TargetPredicate::CreaturePermanent,
        )
    };
    let burn_target = || {
        TargetSlotDef::exactly_one(
            TargetSlotId(1),
            "target for Burn",
            TargetPredicate::AnyTarget,
        )
    };
    CardComposition {
        parts: vec![
            CardPart::new(CardPartId::PRIMARY, "Turn", turn),
            CardPart::new(CardPartId(1), "Burn", burn),
        ],
        structure: CardStructure::Split {
            parts: vec![CardPartId::PRIMARY, CardPartId(1)],
            fused: Some(PlayOptionId(2)),
        },
        play_options: vec![
            PlayOptionDef::cast(
                PlayOptionId::DEFAULT,
                "Turn",
                SpellForm::Part(CardPartId::PRIMARY),
                turn.mana_cost().expect("Turn has a printed mana cost"),
                CardEffectStatus::MetadataOnly,
            )
            .with_targets(vec![turn_target()]),
            PlayOptionDef::cast(
                PlayOptionId(1),
                "Burn",
                SpellForm::Part(CardPartId(1)),
                burn.mana_cost().expect("Burn has a printed mana cost"),
                CardEffectStatus::MetadataOnly,
            )
            .with_targets(vec![burn_target()]),
            PlayOptionDef::cast(
                PlayOptionId(2),
                "Turn // Burn",
                SpellForm::Combined(vec![CardPartId::PRIMARY, CardPartId(1)]),
                mana_cost!("{3}{U}{R}"),
                CardEffectStatus::MetadataOnly,
            )
            .with_targets(vec![turn_target(), burn_target()])
            .restricted_to_hand(),
        ],
    }
}

pub(in crate::card::sets) static TURN_BURN: CardRecord = CardRecord::new(
    cards::TURN_BURN,
    "Turn // Burn",
    CardArt::new("8d7fdd59-6d76-4a0c-ac75-816345ef4a39", "Ryan Barger"),
    CardSet::DragonsMaze,
    turn_rules(),
)
.with_composition(turn_burn_composition);

pub(in crate::card::sets) static UNFLINCHING_COURAGE: CardRecord = CardRecord::new(
    cards::UNFLINCHING_COURAGE,
    "Unflinching Courage",
    CardArt::new("35952c24-d728-4ec6-b0d1-b8183a18554a", "Mike Bierek"),
    CardSet::DragonsMaze,
    CardRules::new_enchantment(mana_cost!("{1}{G}{W}"))
        .with_subtypes(&["Aura"])
        .with_abilities(&[
        AbilityDef::spell(
            "Enchant creature",
            EffectDef::Attach {
                object: EffectRecipientDef::Target(TargetSlotId(0)),
            },
        )
        .with_targets(&[AbilityTargetDef::exactly_one(
            TargetSlotId(0),
            "creature",
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::HasType(CardType::Creature),
                zones: &[ZoneKind::Battlefield],
                controller: None,
                owner: None,
            },
        )]),
        AbilityDef::static_ability(
            "Enchanted creature gets +2/+2 and has trample and lifelink. (Damage dealt by the creature also causes its controller to gain that much life.)",
            EffectDef::Sequence(&[
                EffectDef::Apply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::ModifyPowerToughness {
                        power: ValueDef::Constant(2),
                        toughness: ValueDef::Constant(2),
                    },
                    duration: EffectDurationDef::WhileSourceRemainsInZone,
                },
                EffectDef::Apply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::GrantAbility(&abilities::trample()),
                    duration: EffectDurationDef::WhileSourceRemainsInZone,
                },
                EffectDef::Apply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::GrantAbility(&abilities::lifelink()),
                    duration: EffectDurationDef::WhileSourceRemainsInZone,
                },
            ]),
        ),
    ]),
);

pub(in crate::card::sets) static VOICE_OF_RESURGENCE: CardRecord = CardRecord::new(
    cards::VOICE_OF_RESURGENCE,
    "Voice of Resurgence",
    CardArt::new("07246783-d475-4f61-99ac-e2b574072349", "Winona Nelson"),
    CardSet::DragonsMaze,
    CardRules::new_creature(
        mana_cost!("{G}{W}"),
        &["Elemental"],
        2,
        2,
    )
    .with_ability(AbilityDef::not_implemented(
        "Whenever an opponent casts a spell during your turn and when this creature dies, create a green and white Elemental creature token with \"This token's power and toughness are each equal to the number of creatures you control.\"",
        "Printed rules are cataloged but are not executed by the engine.",
    )),
);

pub(in crate::card::sets) static WARLEADERS_HELIX: CardRecord = CardRecord::new(
    cards::WARLEADERS_HELIX,
    "Warleader's Helix",
    CardArt::new("81e474ac-54f7-43f9-8af9-2f1adf258b15", "Greg Staples"),
    CardSet::DragonsMaze,
    CardRules::new_instant(mana_cost!("{2}{R}{W}")).with_ability(AbilityDef::custom_full(
        "Warleader's Helix deals 4 damage to any target and you gain 4 life.",
        CardBehavior::WarleadersHelix,
        "Implemented by the named card-local special behavior.",
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &AETHERLING,
    &BLOOD_BARON_OF_VIZKOPA,
    &GAZE_OF_GRANITE,
    &PUTREFY,
    &RURIC_THAR_THE_UNBOWED,
    &SIN_COLLECTOR,
    &TURN_BURN,
    &UNFLINCHING_COURAGE,
    &VOICE_OF_RESURGENCE,
    &WARLEADERS_HELIX,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
