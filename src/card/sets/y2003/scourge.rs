//! Scourge cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef,
    CardArt, CardRules, CardSet, EffectDef, EffectRecipientDef, ObjectPredicateDef, PlayerRelation,
    TriggerEventDef, ValueDef, ZoneKind, abilities, cards,
};
use crate::ids::TargetIndex;
use crate::mana_cost;

static GOBLIN_SPELLS: ObjectPredicateDef = ObjectPredicateDef::Subtype("Goblin");

// SCG 97 — Goblin Warchief
pub(in crate::card::sets) static GOBLIN_WARCHIEF: CardRecord = CardRecord::new(
    cards::GOBLIN_WARCHIEF,
    "Goblin Warchief",
    CardArt::new(
        "66864a4b-8924-40ef-a337-15b12413a158",
        "Greg Hildebrandt & Tim Hildebrandt",
    ),
    CardSet::Scourge,
    // The haste is what makes the discount matter: a Goblin cast for one
    // less that also attacks the turn it lands.
    CardRules::new_creature(mana_cost!("{1}{R}{R}"), &["Goblin", "Warrior"], 2, 2).with_abilities(
        &[
            AbilityDef::static_ability(
                "Goblin spells you cast cost {1} less to cast.",
                EffectDef::ReduceMatchingSpellCostBy {
                    spell: GOBLIN_SPELLS,
                    caster: PlayerRelation::You,
                    amount: ValueDef::Constant(1),
                },
            ),
            AbilityDef::static_ability(
                "Goblins you control have haste.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::matching_objects(
                        ObjectPredicateDef::Subtype("Goblin"),
                        &[ZoneKind::Battlefield],
                        PlayerRelation::You,
                    ),
                    effect: AppliedEffectDef::add_ability(&abilities::haste()),
                },
            ),
        ],
    ),
);

// SCG 103 — Siege-Gang Commander
pub(in crate::card::sets) static SIEGE_GANG_COMMANDER: CardRecord = CardRecord::new(
    cards::SIEGE_GANG_COMMANDER,
    "Siege-Gang Commander",
    CardArt::new(
        "92e78cec-aaf9-4fe8-887b-b7e356d63315",
        "Christopher Moeller",
    ),
    CardSet::Scourge,
    // Four bodies for five mana, and the ability turns any of them --
    // including itself -- into two damage anywhere.
    CardRules::new_creature(mana_cost!("{3}{R}{R}"), &["Goblin"], 2, 2).with_abilities(&[
        AbilityDef::triggered(
            "When this creature enters, create three 1/1 red Goblin creature tokens.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                None,
                Some(ZoneKind::Battlefield),
            ),
            EffectDef::CreateToken {
                token: cards::GOBLIN_TOKEN_1_1_RED,
                count: ValueDef::Constant(3),
                tapped: false,
            },
        ),
        AbilityDef::activated_with_targets(
            "{1}{R}, Sacrifice a Goblin: This creature deals 2 damage to any target.",
            &[
                AbilityCostDef::Mana(mana_cost!("{1}{R}")),
                AbilityCostDef::SacrificePermanent {
                    object: ObjectPredicateDef::Subtype("Goblin"),
                    controller: PlayerRelation::You,
                },
            ],
            &[AbilityTargetDef::exactly_one(
                AbilityTargetPredicate::AnyTarget,
            )],
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(2),
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&GOBLIN_WARCHIEF, &SIEGE_GANG_COMMANDER];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
