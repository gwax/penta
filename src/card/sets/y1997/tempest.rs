//! Tempest cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AddManaEffectDef,
    CardArt, CardRules, CardSet, EffectDef, EffectRecipientDef, ObjectPredicateDef,
    TriggerEventDef, ValueDef, cards,
};
use crate::{TargetIndex, mana_cost};

// TMP 183 — Jackal Pup
pub(in crate::card::sets) static JACKAL_PUP: CardRecord = CardRecord::new(
    cards::JACKAL_PUP,
    "Jackal Pup",
    CardArt::new("3707ab74-9aec-4d30-86e0-ffa5f72d5b4f", "Susan Van Camp"),
    CardSet::Tempest,
    CardRules::new_creature(mana_cost!("{R}"), &["Jackal"], 2, 1).with_ability(
        AbilityDef::triggered(
            "Whenever this creature is dealt damage, it deals that much damage to you.",
            TriggerEventDef::DamageDealt {
                source: ObjectPredicateDef::Any,
                recipient: EffectRecipientDef::Source,
            },
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

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&JACKAL_PUP, &MOGG_FANATIC, &LOTUS_PETAL];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
