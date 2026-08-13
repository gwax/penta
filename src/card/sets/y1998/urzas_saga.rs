//! Urza's Saga cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    ObjectPredicateDef, TriggerEventDef, ZoneKind, cards,
};
use crate::{TargetIndex, mana_cost};

// USG 21 — Monk Realist
pub(in crate::card::sets) static MONK_REALIST: CardRecord = CardRecord::new(
    cards::MONK_REALIST,
    "Monk Realist",
    CardArt::new("7a7fe9f1-f3c0-43e4-aa30-d0bdab4ae94d", "Daren Bader"),
    CardSet::UrzasSaga,
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Human", "Monk", "Cleric"], 1, 1).with_ability(
        AbilityDef::triggered_with_targets(
            "When this creature enters, destroy target enchantment.",
            TriggerEventDef::ZoneChanged {
                object: ObjectPredicateDef::Source,
                from: None,
                to: Some(ZoneKind::Battlefield),
            },
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::HasType(CardType::Enchantment),
            )],
            EffectDef::destroy_target(TargetIndex::PRIMARY, true),
        ),
    ),
);

// USG 59 — Annul
pub(in crate::card::sets) static ANNUL: CardRecord = CardRecord::new(
    cards::ANNUL,
    "Annul",
    CardArt::new("3f8c73ff-be92-41ca-93a7-76f9823adb38", "Greg Simanson"),
    CardSet::UrzasSaga,
    CardRules::new_instant(mana_cost!("{U}")).with_ability(AbilityDef::counter_target(
        "Counter target artifact or enchantment spell.",
        &AbilityTargetDef::exactly_one_spell(ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Artifact),
            ObjectPredicateDef::HasType(CardType::Enchantment),
        ])),
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&MONK_REALIST, &ANNUL];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
