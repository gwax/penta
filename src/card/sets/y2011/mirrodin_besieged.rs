//! Mirrodin Besieged cards cataloged as cross-format rules-engine test cases.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef,
    CardArt, CardRules, CardSet, EffectDef, EffectRecipientDef, SpellResolutionDestinationDef,
    ValueDef, abilities, cards,
};
use crate::{TargetIndex, mana_cost};

// MBS 19 — White Sun's Zenith
pub(in crate::card::sets) static WHITE_SUNS_ZENITH: CardRecord = CardRecord::new(
    cards::WHITE_SUNS_ZENITH,
    "White Sun's Zenith",
    CardArt::new("a879940e-6632-47c5-a30e-d29a82d16e9d", "Mike Bierek"),
    CardSet::MirrodinBesieged,
    CardRules::new_instant(mana_cost!("{X}{W}{W}")).with_ability(
        AbilityDef::spell(
            "Create X 2/2 white Cat creature tokens. Shuffle White Sun's Zenith into its owner's library.",
            EffectDef::CreateToken {
                token: cards::CAT_TOKEN_2_2_WHITE,
                count: ValueDef::ChosenX,
                tapped: false,
            },
        )
        .with_resolution_destination(SpellResolutionDestinationDef::LibraryShuffled),
    ),
);

// MBS 115 — Mortarpod
pub(in crate::card::sets) static MORTARPOD: CardRecord = CardRecord::new(
    cards::MORTARPOD,
    "Mortarpod",
    CardArt::new("fbd23da5-421f-41d0-bb60-59560da7dece", "Eric Deschamps"),
    CardSet::MirrodinBesieged,
    CardRules::new_artifact(mana_cost!("{2}"))
        .with_subtypes(&["Equipment"])
        .with_abilities(&[
            abilities::living_weapon(cards::GERM_TOKEN_0_0_BLACK),
            AbilityDef::static_ability(
                "Equipped creature gets +0/+1 and has \"Sacrifice this creature: This creature deals 1 damage to any target.\"",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::Composite(&[
                        AppliedEffectDef::modify_power_toughness(
                            ValueDef::Constant(0),
                            ValueDef::Constant(1),
                        ),
                        AppliedEffectDef::add_ability(&AbilityDef::activated_with_targets(
                            "Sacrifice this creature: This creature deals 1 damage to any target.",
                            &[AbilityCostDef::SacrificeSource],
                            &[AbilityTargetDef::exactly_one(AbilityTargetPredicate::AnyTarget)],
                            EffectDef::DealDamage {
                                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                                amount: ValueDef::Constant(1),
                            },
                        )),
                    ]),
                },
            ),
            abilities::equip(mana_cost!("{2}"), "Equip {2}"),
        ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&WHITE_SUNS_ZENITH, &MORTARPOD];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
