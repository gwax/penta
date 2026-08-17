//! Tempest cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AddManaEffectDef,
    BattlefieldEntryModificationDef, CardArt, CardRules, CardSet, CardSupertype, CardType,
    ChoiceVisibilityDef, ChooseDef, EffectDef, EffectRecipientDef, ManaColor,
    ObjectChoiceBindingDef, ObjectPredicateDef, ObjectQueryDef, ObjectSetDef, PlayerRefDef,
    PlayerRelation, PlayerSetDef, ReplacementEffectDef, ReplacementEventDef, TriggerConditionDef,
    TriggerEventDef, ValueDef, ZoneKind, ZonePlacement, abilities, cards,
};
use crate::ids::ObjectBindingIndex;
use crate::{TargetIndex, mana_cost};

// TMP 51 — Warmth
pub(in crate::card::sets) static WARMTH: CardRecord = CardRecord::new(
    cards::WARMTH,
    "Warmth",
    CardArt::new("d7dbeea8-06b0-4482-bdae-aa82b9db8856", "Drew Tucker"),
    CardSet::Tempest,
    CardRules::new_enchantment(mana_cost!("{1}{W}")).with_ability(AbilityDef::triggered(
        "Whenever an opponent casts a red spell, you gain 2 life.",
        TriggerEventDef::SpellCast(ObjectPredicateDef::All(&[
            ObjectPredicateDef::Color(ManaColor::Red),
            ObjectPredicateDef::ControlledBy(PlayerRelation::Opponent),
        ])),
        EffectDef::GainLife {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(2),
        },
    )),
);

// TMP 151 — Reanimate
pub(in crate::card::sets) static REANIMATE: CardRecord = CardRecord::new(
    cards::REANIMATE,
    "Reanimate",
    CardArt::new("fc00f897-988b-4602-969a-c510804ec12a", "Robert Bliss"),
    CardSet::Tempest,
    CardRules::new_sorcery(mana_cost!("{B}")).with_ability(AbilityDef::spell_with_targets(
        "Put target creature card from a graveyard onto the battlefield under your control. You lose life equal to that card's mana value.",
        &[AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::HasType(CardType::Creature),
            zones: &[ZoneKind::Graveyard],
            controller: None,
            owner: None,
        })],
        EffectDef::Sequence(&[
            EffectDef::MoveToZone {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                zone: ZoneKind::Battlefield,
                placement: ZonePlacement::Top,
                arrival_effect: None,
                controller: Some(PlayerRelation::You),
            },
            EffectDef::LoseLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::TargetManaValue(TargetIndex::PRIMARY),
            },
        ]),
    )),
);

// TMP 183 — Jackal Pup
pub(in crate::card::sets) static JACKAL_PUP: CardRecord = CardRecord::new(
    cards::JACKAL_PUP,
    "Jackal Pup",
    CardArt::new("3707ab74-9aec-4d30-86e0-ffa5f72d5b4f", "Susan Van Camp"),
    CardSet::Tempest,
    CardRules::new_creature(mana_cost!("{R}"), &["Jackal"], 2, 1).with_ability(
        AbilityDef::triggered(
            "Whenever this creature is dealt damage, it deals that much damage to you.",
            TriggerEventDef::damage_to_source(),
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::TriggerEventAmount,
            },
        ),
    ),
);

// TMP 190 — Mogg Fanatic
pub(in crate::card::sets) static MOGG_FANATIC: CardRecord = CardRecord::new(
    cards::MOGG_FANATIC,
    "Mogg Fanatic",
    CardArt::new("ca2ecfd4-c874-4468-8601-87aa110d5a00", "Brom"),
    CardSet::Tempest,
    CardRules::new_creature(mana_cost!("{R}"), &["Goblin"], 1, 1).with_ability(
        AbilityDef::activated_with_targets(
            "Sacrifice this creature: It deals 1 damage to any target.",
            &[AbilityCostDef::SacrificeSource],
            &[AbilityTargetDef::exactly_one(
                AbilityTargetPredicate::AnyTarget,
            )],
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(1),
            },
        ),
    ),
);

// TMP 250 — Root Maze
pub(in crate::card::sets) static ROOT_MAZE: CardRecord = CardRecord::new(
    cards::ROOT_MAZE,
    "Root Maze",
    CardArt::new("99a12b74-f191-4362-81ab-77590ae5e68f", "Rebecca Guay"),
    CardSet::Tempest,
    CardRules::new_enchantment(mana_cost!("{G}")).with_ability(AbilityDef::replacement_for(
        "Artifacts and lands enter tapped.",
        ReplacementEventDef::ObjectEntersBattlefield {
            object: ObjectPredicateDef::AnyOf(&[
                ObjectPredicateDef::HasType(CardType::Artifact),
                ObjectPredicateDef::HasType(CardType::Land),
            ]),
            controller: PlayerRelation::Any,
        },
        ReplacementEffectDef::ModifyBattlefieldEntry(BattlefieldEntryModificationDef::Tapped),
    )),
);

/// Naming a card is modelled as picking one of the cards in hand. Every name
/// worth choosing is one of those -- naming something you do not hold can
/// only fail -- and the choice is public either way, so nothing is hidden and
/// nothing achievable is lost.
static NAMED_CARD: ObjectBindingIndex = ObjectBindingIndex::PRIMARY;
static REVEALED_CARD: ObjectBindingIndex = ObjectBindingIndex::new(1);

static SCROLL_NAMES_MATCH: TriggerConditionDef = TriggerConditionDef::BoundObjectsShareName {
    first: NAMED_CARD,
    second: REVEALED_CARD,
};

static SCROLL_SHOT: EffectDef = EffectDef::DealDamage {
    recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    amount: ValueDef::Constant(2),
};

static SCROLL_IF_MATCHED: EffectDef = EffectDef::IfCondition {
    condition: &SCROLL_NAMES_MATCH,
    then: &SCROLL_SHOT,
};

static SCROLL_REVEAL: EffectDef = EffectDef::RevealAtRandomFromHand {
    player: EffectRecipientDef::Controller,
    binding: REVEALED_CARD,
    then: &SCROLL_IF_MATCHED,
};

static CARDS_IN_YOUR_HAND: ObjectQueryDef = ObjectQueryDef::owned_by(
    ObjectPredicateDef::Any,
    &[ZoneKind::Hand],
    PlayerSetDef::Related(PlayerRelation::You),
);

// TMP 281 — Cursed Scroll
pub(in crate::card::sets) static CURSED_SCROLL: CardRecord = CardRecord::new(
    cards::CURSED_SCROLL,
    "Cursed Scroll",
    CardArt::new(
        "31415b9b-fb30-4132-a9a3-795b4573a901",
        "D. Alexander Gregory",
    ),
    CardSet::Tempest,
    // An empty hand makes it a certainty, which is why the card belongs in a
    // deck that has already spent everything.
    CardRules::new_artifact(mana_cost!("{1}")).with_ability(AbilityDef::activated_with_targets(
        "{3}, {T}: Choose a card name, then reveal a card at random from your hand. If that card has the chosen name, this artifact deals 2 damage to any target.",
        &[
            AbilityCostDef::Mana(mana_cost!("{3}")),
            AbilityCostDef::TapSource,
        ],
        &SCROLL_TARGET,
        EffectDef::Choose(ChooseDef {
            binding: ObjectChoiceBindingDef::Object(NAMED_CARD),
            chooser: PlayerRefDef::EffectController,
            candidates: ObjectSetDef::Query(CARDS_IN_YOUR_HAND),
            exclude: None,
            minimum: 1,
            maximum: 1,
            visibility: ChoiceVisibilityDef::Public,
            then: &SCROLL_REVEAL,
        }),
    )),
);

static SCROLL_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::AnyTarget,
)];

// TMP 294 — Lotus Petal
pub(in crate::card::sets) static LOTUS_PETAL: CardRecord = CardRecord::new(
    cards::LOTUS_PETAL,
    "Lotus Petal",
    CardArt::new("6c877da3-68fa-41d0-8a24-8c79fcd8ecc1", "April Lee"),
    CardSet::Tempest,
    CardRules::new_artifact(mana_cost!("{0}")).with_ability(AbilityDef::activated_mana(
        "{T}, Sacrifice this artifact: Add one mana of any color.",
        &[AbilityCostDef::TapSource, AbilityCostDef::SacrificeSource],
        EffectDef::AddMana(AddManaEffectDef::any_color()),
    )),
);

// TMP 315 — Ancient Tomb
pub(in crate::card::sets) static ANCIENT_TOMB: CardRecord = CardRecord::new(
    cards::ANCIENT_TOMB,
    "Ancient Tomb",
    CardArt::new("30e401e3-282b-4524-87e1-c6cd50cd6d00", "Colin MacNeil"),
    CardSet::Tempest,
    CardRules::new_land(&[]).with_ability(AbilityDef::activated_mana(
        "{T}: Add {C}{C}. This land deals 2 damage to you.",
        &[AbilityCostDef::TapSource],
        EffectDef::AddMana(
            AddManaEffectDef::one(ManaColor::Colorless)
                .with_amount(2)
                .with_damage_to_controller(2),
        ),
    )),
);

// TMP 330 — Wasteland
pub(in crate::card::sets) static WASTELAND: CardRecord = CardRecord::new(
    cards::WASTELAND,
    "Wasteland",
    CardArt::new("99ff731b-8399-40c8-b539-ba6ba5783771", "Una Fricker"),
    CardSet::Tempest,
    CardRules::new_land(&[]).with_abilities(&[
        abilities::tap_for(ManaColor::Colorless),
        AbilityDef::activated_with_targets(
            "{T}, Sacrifice this land: Destroy target nonbasic land.",
            &[AbilityCostDef::TapSource, AbilityCostDef::SacrificeSource],
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Land),
                    ObjectPredicateDef::Not(&ObjectPredicateDef::Supertype(CardSupertype::Basic)),
                ]),
            )],
            EffectDef::destroy_target(TargetIndex::PRIMARY, true),
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &WARMTH,
    &REANIMATE,
    &JACKAL_PUP,
    &MOGG_FANATIC,
    &ROOT_MAZE,
    &CURSED_SCROLL,
    &LOTUS_PETAL,
    &ANCIENT_TOMB,
    &WASTELAND,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
