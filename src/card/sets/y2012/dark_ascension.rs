//! Dark Ascension card records used by the built-in ISD–RTR Standard deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef,
    CardArt, CardComposition, CardEffectStatus, CardPart, CardRules, CardSet, CardStructure,
    CardType, DoubleFacedKind, EffectDef, EffectDurationDef, EffectRecipientDef, LandEntry,
    ManaColor, ObjectPredicateDef, PlayOptionDef, PlayerRelation, SpellForm, ValueDef, ZoneKind,
    abilities, cards,
};
use crate::ids::{CardPartId, PlayOptionId, TargetSlotId};
use crate::mana_cost;

pub(in crate::card::sets) static HELLRIDER: CardRecord = CardRecord::new(
    cards::HELLRIDER,
    "Hellrider",
    CardArt::new("0ec8d800-7f06-44e0-b22d-cdff0a9b153d", "Svetlin Velinov"),
    CardSet::DarkAscension,
    CardRules::new_creature(
        mana_cost!("{2}{R}{R}"),
        &["Devil"],
        3,
        3,
    )
    .with_abilities(&[
        abilities::haste(),
        AbilityDef::not_implemented(
            "Whenever a creature you control attacks, this creature deals 1 damage to the player or planeswalker it's attacking.",
            "The attack trigger is not executed.",
        ),
    ]),
);

const fn huntmaster_front_rules() -> CardRules {
    CardRules::new_creature(
        mana_cost!("{2}{R}{G}"),
        &["Human", "Werewolf"],
        2,
        2,
    )
    .with_ability(AbilityDef::not_implemented(
        "Whenever this creature enters or transforms into Huntmaster of the Fells, create a 2/2 green Wolf creature token and you gain 2 life.\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.",
        "Printed rules are cataloged but are not executed by the engine.",
    ))
}

static HUNTMASTER_BACK_ABILITIES: [AbilityDef; 2] = [
    abilities::trample(),
    AbilityDef::not_implemented(
        "Whenever this creature transforms into Ravager of the Fells, it deals 2 damage to target opponent or planeswalker and 2 damage to up to one target creature that player or that planeswalker's controller controls.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.",
        "The transform trigger, damage trigger, and transformation procedure are not executed.",
    ),
];

const fn huntmaster_back_rules() -> CardRules {
    CardRules::new_creature_without_mana_cost(&["Werewolf"], 4, 4)
        .printed_colors(&[ManaColor::Red, ManaColor::Green])
        .with_abilities(&HUNTMASTER_BACK_ABILITIES)
}

fn huntmaster_composition() -> CardComposition {
    let front = huntmaster_front_rules();
    let back = huntmaster_back_rules();
    CardComposition {
        parts: vec![
            CardPart::new(CardPartId::PRIMARY, "Huntmaster of the Fells", front),
            CardPart::new(CardPartId(1), "Ravager of the Fells", back),
        ],
        structure: CardStructure::DoubleFaced {
            front: CardPartId::PRIMARY,
            back: CardPartId(1),
            kind: DoubleFacedKind::Transforming,
        },
        play_options: vec![PlayOptionDef::cast(
            PlayOptionId::DEFAULT,
            "Huntmaster of the Fells",
            SpellForm::Part(CardPartId::PRIMARY),
            front
                .mana_cost()
                .expect("Huntmaster of the Fells has a printed mana cost"),
            CardEffectStatus::MetadataOnly,
        )],
    }
}

pub(in crate::card::sets) static HUNTMASTER_OF_THE_FELLS: CardRecord = CardRecord::new(
    cards::HUNTMASTER_OF_THE_FELLS,
    "Huntmaster of the Fells",
    CardArt::new("aae6fb12-b252-453b-bca7-1ea2a0d6c8dc", "Chris Rahn"),
    CardSet::DarkAscension,
    huntmaster_front_rules(),
)
.with_composition(huntmaster_composition);

pub(in crate::card::sets) static RAY_OF_REVELATION: CardRecord = CardRecord::new(
    cards::RAY_OF_REVELATION,
    "Ray of Revelation",
    CardArt::new("d7e2c5a4-cf92-46bd-9033-8036436488cb", "Cliff Childs"),
    CardSet::DarkAscension,
    CardRules::new_instant(mana_cost!("{1}{W}")).with_abilities(&[
        AbilityDef::spell(
            "Destroy target enchantment.",
            EffectDef::Destroy {
                object: EffectRecipientDef::Target(TargetSlotId(0)),
                can_regenerate: true,
            },
        )
        .with_targets(&[AbilityTargetDef::exactly_one(
            TargetSlotId(0),
            "enchantment",
            AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::HasType(CardType::Enchantment),
                zones: &[ZoneKind::Battlefield],
                controller: None,
                owner: None,
            },
        )]),
        AbilityDef::not_implemented(
            "Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
            "Casting from the graveyard for a flashback cost is not implemented.",
        ),
    ]),
);

pub(in crate::card::sets) static STRANGLEROOT_GEIST: CardRecord = CardRecord::new(
    cards::STRANGLEROOT_GEIST,
    "Strangleroot Geist",
    CardArt::new("bf1fb137-205c-480f-b6dc-dfa137793ae3", "Jason Chan"),
    CardSet::DarkAscension,
    CardRules::new_creature(
        mana_cost!("{G}{G}"),
        &["Spirit"],
        2,
        1,
    )
    .with_abilities(&[
        abilities::haste(),
        abilities::undying().with_text(
            "Undying (When this creature dies, if it had no +1/+1 counters on it, return it to the battlefield under its owner's control with a +1/+1 counter on it.)",
        ),
    ]),
);

pub(in crate::card::sets) static TRAGIC_SLIP: CardRecord = CardRecord::new(
    cards::TRAGIC_SLIP,
    "Tragic Slip",
    CardArt::new("09666671-601e-4fca-bdfb-fb288bf2672c", "Christopher Moeller"),
    CardSet::DarkAscension,
    CardRules::new_instant(mana_cost!("{B}")).with_abilities(&[
        AbilityDef::spell(
            "Target creature gets -1/-1 until end of turn.",
            EffectDef::Apply {
                recipient: EffectRecipientDef::Target(TargetSlotId(0)),
                effect: AppliedEffectDef::ModifyPowerToughness {
                    power: ValueDef::Constant(-1),
                    toughness: ValueDef::Constant(-1),
                },
                duration: EffectDurationDef::UntilEndOfTurn,
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
        AbilityDef::not_implemented(
            "Morbid — That creature gets -13/-13 until end of turn instead if a creature died this turn.",
            "Morbid conditions that depend on a creature having died this turn are not implemented, so the spell always applies -1/-1.",
        ),
    ]),
);

pub(in crate::card::sets) static VAULT_OF_THE_ARCHANGEL: CardRecord = CardRecord::new(
    cards::VAULT_OF_THE_ARCHANGEL,
    "Vault of the Archangel",
    CardArt::new("35a65437-430a-42ef-854f-6e66f8e1a04a", "John Avon"),
    CardSet::DarkAscension,
    CardRules::new_land(&[])
    .land_entry(LandEntry::Untapped)
    .with_abilities(&[
        abilities::tap_for(ManaColor::Colorless),
        AbilityDef::activated(
            "{2}{W}{B}, {T}: Creatures you control gain deathtouch and lifelink until end of turn.",
            &[
                AbilityCostDef::Mana(mana_cost!("{2}{W}{B}")),
                AbilityCostDef::TapSource,
            ],
            EffectDef::Sequence(&[
                EffectDef::Apply {
                    recipient: EffectRecipientDef::MatchingObjects {
                    object: ObjectPredicateDef::All(&[
                        ObjectPredicateDef::HasType(CardType::Creature),
                        ObjectPredicateDef::ControlledBy(PlayerRelation::You),
                    ]),
                    zones: &[ZoneKind::Battlefield],
                    controller: PlayerRelation::You,
                },
                    effect: AppliedEffectDef::GrantAbility(&abilities::deathtouch()),
                    duration: EffectDurationDef::UntilEndOfTurn,
                },
                EffectDef::Apply {
                    recipient: EffectRecipientDef::MatchingObjects {
                    object: ObjectPredicateDef::All(&[
                        ObjectPredicateDef::HasType(CardType::Creature),
                        ObjectPredicateDef::ControlledBy(PlayerRelation::You),
                    ]),
                    zones: &[ZoneKind::Battlefield],
                    controller: PlayerRelation::You,
                },
                    effect: AppliedEffectDef::GrantAbility(&abilities::lifelink()),
                    duration: EffectDurationDef::UntilEndOfTurn,
                },
            ]),
        ),
    ]),
);
pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &HELLRIDER,
    &HUNTMASTER_OF_THE_FELLS,
    &RAY_OF_REVELATION,
    &STRANGLEROOT_GEIST,
    &TRAGIC_SLIP,
    &VAULT_OF_THE_ARCHANGEL,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
