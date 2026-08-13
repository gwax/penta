//! Ice Age cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCoverageDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, CardArt, CardRules,
    CardSet, EffectDef, EffectRecipientDef, ManaColor, ObjectPredicateDef, ValueDef, cards,
};
use crate::{TargetIndex, mana_cost};

// ICE 72 — Hydroblast
pub(in crate::card::sets) static HYDROBLAST: CardRecord = CardRecord::new(
    cards::HYDROBLAST,
    "Hydroblast",
    CardArt::new("f62716f0-fde2-49ef-b8a4-c1b03f451194", "Kaja Foglio"),
    CardSet::IceAge,
    CardRules::new_instant(mana_cost!("{U}")).with_ability(AbilityDef::choose_one_spell(
        "Choose one —\n• Counter target spell if it's red.\n• Destroy target permanent if it's red.",
        &[
            AbilityDef::counter_target(
                "Counter target spell if it's red",
                &AbilityTargetDef::exactly_one_spell(ObjectPredicateDef::Color(ManaColor::Red)),
            ),
            AbilityDef::destroy_target(
                "Destroy target permanent if it's red",
                &AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::Color(
                    ManaColor::Red,
                )),
                true,
            ),
        ],
    )),
);

// ICE 194 — Incinerate
pub(in crate::card::sets) static INCINERATE: CardRecord = CardRecord::new(
    cards::INCINERATE,
    "Incinerate",
    CardArt::new("9c3f00af-010d-4485-b8b7-47400d99c496", "Mark Poole"),
    CardSet::IceAge,
    CardRules::new_instant(mana_cost!("{1}{R}")).with_ability(
        AbilityDef::spell_with_targets(
            "Incinerate deals 3 damage to any target. A creature dealt damage this way can't be regenerated this turn.",
            &[AbilityTargetDef::exactly_one(
                AbilityTargetPredicate::AnyTarget,
            )],
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(3),
            },
        )
        .with_coverage(AbilityCoverageDef::partial(
            "The damage is implemented; preventing regeneration for the rest of the turn is not yet modeled.",
        )),
    ),
);

// ICE 213 — Pyroblast
pub(in crate::card::sets) static PYROBLAST: CardRecord = CardRecord::new(
    cards::PYROBLAST,
    "Pyroblast",
    CardArt::new("c342cac5-08ae-4428-9c2c-f6c5904e54d2", "Kaja Foglio"),
    CardSet::IceAge,
    CardRules::new_instant(mana_cost!("{R}")).with_ability(AbilityDef::choose_one_spell(
        "Choose one —\n• Counter target spell if it's blue.\n• Destroy target permanent if it's blue.",
        &[
            AbilityDef::counter_target(
                "Counter target spell if it's blue",
                &AbilityTargetDef::exactly_one_spell(ObjectPredicateDef::Color(ManaColor::Blue)),
            ),
            AbilityDef::destroy_target(
                "Destroy target permanent if it's blue",
                &AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::Color(
                    ManaColor::Blue,
                )),
                true,
            ),
        ],
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&HYDROBLAST, &INCINERATE, &PYROBLAST];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
