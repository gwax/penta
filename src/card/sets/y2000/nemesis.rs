//! Nemesis cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, CardArt, CardRules,
    CardSet, CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef, ValueDef, cards,
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

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&SEAL_OF_CLEANSING, &SEAL_OF_FIRE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
