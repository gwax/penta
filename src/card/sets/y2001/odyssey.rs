//! Odyssey cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AddManaEffectDef,
    AppliedEffectDef, CardArt, CardRules, CardSet, ComparisonDef, DiscardSelectionDef, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, ObjectQueryDef, PlayerRelation,
    PlayerSetDef, ResolvedEffectDurationDef, SpellAdditionalCostDef, SpendModeDef,
    TriggerConditionDef, TriggerEventDef, ValueDef, ZoneKind, ZonePlacement, cards,
};
use crate::{TargetIndex, mana_cost};

/// Everyone who is not the caster draws, so casting into it is what makes it
/// resolve against you. In a two-player game that is the opponent alone.
static STANDSTILL_REFILL: EffectDef = EffectDef::Sequence(&[
    EffectDef::Sacrifice {
        object: EffectRecipientDef::Source,
    },
    EffectDef::DrawCards {
        recipient: EffectRecipientDef::players(PlayerSetDef::Related(PlayerRelation::Opponent)),
        amount: ValueDef::Constant(3),
    },
]);

// ODY 102 — Standstill
pub(in crate::card::sets) static STANDSTILL: CardRecord = CardRecord::new(
    cards::STANDSTILL,
    "Standstill",
    CardArt::new("3ede3f6f-e642-4fe4-aa37-0f01cdf4d149", "Heather Hudson"),
    CardSet::Odyssey,
    // A deck built to do nothing profits from the stalemate; whoever blinks
    // first hands over three cards.
    CardRules::new_enchantment(mana_cost!("{1}{U}")).with_ability(AbilityDef::triggered(
        "When a player casts a spell, sacrifice this enchantment. If you do, each of that player's opponents draws three cards.",
        TriggerEventDef::SpellCast(ObjectPredicateDef::Any),
        STANDSTILL_REFILL,
    )),
);

// ODY 113 — Upheaval
pub(in crate::card::sets) static UPHEAVAL: CardRecord = CardRecord::new(
    cards::UPHEAVAL,
    "Upheaval",
    CardArt::new("9e201229-34a6-48c8-a07c-d8aefcf5f8a7", "Kev Walker"),
    CardSet::Odyssey,
    CardRules::new_sorcery(mana_cost!("{4}{U}{U}")).with_ability(AbilityDef::spell(
        "Return all permanents to their owners' hands.",
        EffectDef::MoveToZone {
            object: EffectRecipientDef::matching_objects(
                ObjectPredicateDef::Any,
                &[ZoneKind::Battlefield],
                PlayerRelation::Any,
            ),
            zone: ZoneKind::Hand,
            placement: ZonePlacement::Top,
            arrival_effect: None,
            controller: None,
        },
    )),
);

/// Both halves pump the same amount, so they share the applied effect. The
/// Atog eats its own graveyard as readily as its hand, which is why it grows
/// so fast in a deck that has been drawing and discarding all game.
static ATOG_PUMP: EffectDef = EffectDef::Apply {
    recipient: EffectRecipientDef::Source,
    effect: AppliedEffectDef::modify_power_toughness(ValueDef::Constant(1), ValueDef::Constant(1)),
    duration: ResolvedEffectDurationDef::UntilEndOfTurn,
};

/// X cards from your own graveyard, exiled as the spell is cast. The count is
/// the X it is cast for, so a big Scrying costs the graveyard that fed it.
static EXILE_X_FROM_YOUR_GRAVEYARD: SpellAdditionalCostDef =
    SpellAdditionalCostDef::new(ObjectPredicateDef::Any, ZoneKind::Graveyard, 0)
        .counted_in_x()
        .spent(SpendModeDef::Exile);

// ODY 161 — Skeletal Scrying
pub(in crate::card::sets) static SKELETAL_SCRYING: CardRecord = CardRecord::new(
    cards::SKELETAL_SCRYING,
    "Skeletal Scrying",
    CardArt::new("ee49bae4-6b1a-4c3f-8b2e-1d5a7c9e3f2b", "Bob Petillo"),
    CardSet::Odyssey,
    // Cards for life, paid for with the graveyard: a control deck that has
    // already spent its removal has the fuel and can afford the life.
    CardRules::new_instant(mana_cost!("{X}{B}")).with_ability(
        AbilityDef::spell_with_additional_cost(
            "As an additional cost to cast this spell, exile X cards from your graveyard.\nYou draw X cards and you lose X life.",
            &[],
            EXILE_X_FROM_YOUR_GRAVEYARD,
            EffectDef::Sequence(&[
                EffectDef::DrawCards {
                    recipient: EffectRecipientDef::Controller,
                    amount: ValueDef::ChosenX,
                },
                EffectDef::LoseLife {
                    recipient: EffectRecipientDef::Controller,
                    amount: ValueDef::ChosenX,
                },
            ]),
        ),
    ),
);

// ODY 292 — Psychatog
pub(in crate::card::sets) static PSYCHATOG: CardRecord = CardRecord::new(
    cards::PSYCHATOG,
    "Psychatog",
    CardArt::new(
        "6757bf0e-489f-4be2-9e41-463b59f00dd1",
        "Edward P. Beard, Jr.",
    ),
    CardSet::Odyssey,
    CardRules::new_creature(mana_cost!("{1}{U}{B}"), &["Atog"], 1, 2).with_abilities(&[
        AbilityDef::activated(
            "Discard a card: This creature gets +1/+1 until end of turn.",
            &[AbilityCostDef::DiscardCardMatching(ObjectPredicateDef::Any)],
            ATOG_PUMP,
        ),
        AbilityDef::activated(
            "Exile two cards from your graveyard: This creature gets +1/+1 until end of turn.",
            &[AbilityCostDef::ExileCardsFromGraveyard {
                object: ObjectPredicateDef::Any,
                count: 2,
            }],
            ATOG_PUMP,
        ),
    ]),
);

/// Threshold: seven or more cards in your own graveyard. The count is of
/// cards you own, not of every graveyard on the table.
static YOUR_GRAVEYARD: ObjectQueryDef = ObjectQueryDef::owned_by(
    ObjectPredicateDef::Any,
    &[ZoneKind::Graveyard],
    crate::card::PlayerSetDef::Related(PlayerRelation::You),
);

static THRESHOLD: TriggerConditionDef = TriggerConditionDef::ObjectCount {
    query: YOUR_GRAVEYARD,
    comparison: ComparisonDef::GreaterOrEqual,
    amount: 7,
};

static BARBARIAN_RING_SHOT: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::AnyTarget,
)];

// ODY 313 — Barbarian Ring
pub(in crate::card::sets) static BARBARIAN_RING: CardRecord = CardRecord::new(
    cards::BARBARIAN_RING,
    "Barbarian Ring",
    CardArt::new("1809361e-ae1a-4c47-8464-e6496e94d962", "John Avon"),
    CardSet::Odyssey,
    // The land costs a life every time it makes mana, and pays that back once
    // the graveyard is deep enough to turn it into a burn spell.
    CardRules::new_land(&[]).with_abilities(&[
        AbilityDef::activated_mana(
            "{T}: Add {R}. This land deals 1 damage to you.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::one(ManaColor::Red).with_damage_to_controller(1)),
        ),
        AbilityDef::activated_with_targets(
            "Threshold — {R}, {T}, Sacrifice this land: It deals 2 damage to any target. Activate only if there are seven or more cards in your graveyard.",
            &[
                AbilityCostDef::Mana(mana_cost!("{R}")),
                AbilityCostDef::TapSource,
                AbilityCostDef::SacrificeSource,
            ],
            &BARBARIAN_RING_SHOT,
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(2),
            },
        )
        .with_activation_condition(&THRESHOLD),
    ]),
);

static COLISEUM_DIG: EffectDef = EffectDef::Sequence(&[
    EffectDef::DrawCards {
        recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        amount: ValueDef::Constant(3),
    },
    EffectDef::Discard {
        recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        amount: ValueDef::Constant(3),
        selection: DiscardSelectionDef::RecipientChooses,
    },
]);

static COLISEUM_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Player(PlayerRelation::Any),
)];

// ODY 317 — Cephalid Coliseum
pub(in crate::card::sets) static CEPHALID_COLISEUM: CardRecord = CardRecord::new(
    cards::CEPHALID_COLISEUM,
    "Cephalid Coliseum",
    CardArt::new("d5d74112-7244-4c3f-a5eb-b6be671aefe8", "John Avon"),
    CardSet::Odyssey,
    // The blue Barbarian Ring: a life every time it makes mana, and once the
    // graveyard is deep enough it cashes itself in for three fresh cards.
    CardRules::new_land(&[]).with_abilities(&[
        AbilityDef::activated_mana(
            "{T}: Add {U}. This land deals 1 damage to you.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::one(ManaColor::Blue).with_damage_to_controller(1)),
        ),
        AbilityDef::activated_with_targets(
            "Threshold — {U}, {T}, Sacrifice this land: Target player draws three cards, then discards three cards. Activate only if there are seven or more cards in your graveyard.",
            &[
                AbilityCostDef::Mana(mana_cost!("{U}")),
                AbilityCostDef::TapSource,
                AbilityCostDef::SacrificeSource,
            ],
            &COLISEUM_TARGET,
            COLISEUM_DIG,
        )
        .with_activation_condition(&THRESHOLD),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &STANDSTILL,
    &UPHEAVAL,
    &SKELETAL_SCRYING,
    &PSYCHATOG,
    &BARBARIAN_RING,
    &CEPHALID_COLISEUM,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[
    PrintingRecord::reprint(&crate::card::sets::y2012::return_to_ravnica::SYNCOPATE), // ODY 103
];
