//! Nemesis cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef,
    BasicLandType, CardArt, CardRules, CardSet, CardSupertype, CardType, ComparisonDef,
    DamageEventMatcherDef, DamagePreventionDef, EffectDef, EffectRecipientDef, ManaColor,
    ObjectPredicateDef, ObjectQueryDef, ObjectRefDef, PlayerRelation, ResolvedEffectDurationDef,
    SpellAdditionalCostDef, SpendModeDef, TriggerConditionDef, ValueDef, ZoneKind, abilities,
    cards,
};
use crate::{TargetIndex, mana_cost};

// NEM 18 — Seal of Cleansing
pub(in crate::card::sets) static SEAL_OF_CLEANSING: CardRecord = CardRecord::new(
    cards::SEAL_OF_CLEANSING,
    "Seal of Cleansing",
    CardArt::new(
        "af6c921e-1b82-412c-9979-adfdf83440f7",
        "Christopher Moeller",
    ),
    CardSet::Nemesis,
    CardRules::new_enchantment(mana_cost!("{1}{W}")).with_ability(
        AbilityDef::activated_with_targets(
            "Sacrifice this enchantment: Destroy target artifact or enchantment.",
            &[AbilityCostDef::SacrificeSource],
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::AnyOf(&[
                    ObjectPredicateDef::HasType(CardType::Artifact),
                    ObjectPredicateDef::HasType(CardType::Enchantment),
                ]),
            )],
            EffectDef::destroy_target(TargetIndex::PRIMARY, true),
        ),
    ),
);

/// One Island back to hand, which is what makes the card free on turn one and
/// a real cost on turn six.
static DAZE_COST: SpellAdditionalCostDef = SpellAdditionalCostDef::new(
    ObjectPredicateDef::HasAnyBasicLandType(&[BasicLandType::Island]),
    ZoneKind::Battlefield,
    1,
)
.spent(SpendModeDef::ReturnToHand);

static DAZE_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::Spell,
        zones: &[ZoneKind::Stack],
        controller: None,
        owner: None,
    },
)];

// NEM 30 — Daze
pub(in crate::card::sets) static DAZE: CardRecord = CardRecord::new(
    cards::DAZE,
    "Daze",
    CardArt::new("d03bff25-0d5e-4dcf-8d75-6df846afea3b", "Matthew D. Wilson"),
    CardSet::Nemesis,
    CardRules::new_instant(mana_cost!("{1}{U}")).with_abilities(&[
        AbilityDef::spell_with_targets(
            "Counter target spell unless its controller pays {1}.",
            &DAZE_TARGET,
            abilities::counter_target_unless_paid(ValueDef::Constant(1)),
        ),
        AbilityDef::alternative_cast(
            mana_cost!("{0}"),
            AlternativeCastKindDef::AlternativeCost,
            Some("You may return an Island you control to its owner's hand rather than pay this spell's mana cost."),
            EffectDef::None,
        )
        .with_alternative_additional_cost(&DAZE_COST),
    ]),
);

/// "If an opponent controls an Island and you control a Mountain" -- one
/// condition made of two, checked where the free cast is offered rather than
/// where it resolves.
static AN_OPPONENTS_ISLAND: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::HasAnyBasicLandType(&[BasicLandType::Island]),
    &[ZoneKind::Battlefield],
    PlayerRelation::Opponent,
);

static YOUR_MOUNTAIN: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::HasAnyBasicLandType(&[BasicLandType::Mountain]),
    &[ZoneKind::Battlefield],
    PlayerRelation::You,
);

static SALVAGE_WINDOW: TriggerConditionDef = TriggerConditionDef::All(&[
    TriggerConditionDef::ObjectCount {
        query: AN_OPPONENTS_ISLAND,
        comparison: ComparisonDef::GreaterOrEqual,
        amount: 1,
    },
    TriggerConditionDef::ObjectCount {
        query: YOUR_MOUNTAIN,
        comparison: ComparisonDef::GreaterOrEqual,
        amount: 1,
    },
]);

// NEM 94 — Mogg Salvage
pub(in crate::card::sets) static MOGG_SALVAGE: CardRecord = CardRecord::new(
    cards::MOGG_SALVAGE,
    "Mogg Salvage",
    CardArt::new("403aa48c-b684-4c54-8863-460958055a1f", "Paolo Parente"),
    CardSet::Nemesis,
    // Free only against the deck it was printed to beat, which is why it is a
    // sideboard card rather than a maindeck one.
    CardRules::new_instant(mana_cost!("{2}{R}")).with_abilities(&[
        AbilityDef::destroy_target(
            "Destroy target artifact.",
            &AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::HasType(
                CardType::Artifact,
            )),
            true,
        ),
        AbilityDef::alternative_cast(
            mana_cost!("{0}"),
            AlternativeCastKindDef::AlternativeCost,
            Some("If an opponent controls an Island and you control a Mountain, you may cast this spell without paying its mana cost."),
            EffectDef::None,
        )
        .with_alternative_condition(&SALVAGE_WINDOW),
    ]),
);

// NEM 98 — Seal of Fire
pub(in crate::card::sets) static SEAL_OF_FIRE: CardRecord = CardRecord::new(
    cards::SEAL_OF_FIRE,
    "Seal of Fire",
    CardArt::new(
        "37eaf1f6-4bdc-4669-9a15-50b65e016ccf",
        "Christopher Moeller",
    ),
    CardSet::Nemesis,
    CardRules::new_enchantment(mana_cost!("{R}")).with_ability(AbilityDef::activated_with_targets(
        "Sacrifice this enchantment: It deals 2 damage to any target.",
        &[AbilityCostDef::SacrificeSource],
        &[AbilityTargetDef::exactly_one(
            AbilityTargetPredicate::AnyTarget,
        )],
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(2),
        },
    )),
);

// NEM 141 — Kor Haven
pub(in crate::card::sets) static KOR_HAVEN: CardRecord = CardRecord::new(
    cards::KOR_HAVEN,
    "Kor Haven",
    CardArt::new("3d5529ca-5c20-4dfd-8595-96d6dfa6debe", "Darrell Riche"),
    CardSet::Nemesis,
    CardRules::new_land(&[])
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&[
            abilities::tap_for(ManaColor::Colorless),
            AbilityDef::activated_with_targets(
                "{1}{W}, {T}: Prevent all combat damage that would be dealt by target attacking creature this turn.",
                &[
                    AbilityCostDef::Mana(mana_cost!("{1}{W}")),
                    AbilityCostDef::TapSource,
                ],
                &[AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
                    object: ObjectPredicateDef::All(&[
                        ObjectPredicateDef::HasType(CardType::Creature),
                        ObjectPredicateDef::Attacking,
                    ]),
                    zones: &[ZoneKind::Battlefield],
                    controller: None,
                    owner: None,
                })],
                EffectDef::PreventDamage {
                    prevention: DamagePreventionDef::unlimited(
                        DamageEventMatcherDef::combat_from(ObjectRefDef::Target(
                            TargetIndex::PRIMARY,
                        )),
                    ),
                    duration: ResolvedEffectDurationDef::UntilEndOfTurn,
                },
            ),
        ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &SEAL_OF_CLEANSING,
    &DAZE,
    &MOGG_SALVAGE,
    &SEAL_OF_FIRE,
    &KOR_HAVEN,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
