//! Innistrad: Midnight Hunt cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    ObjectPredicateDef, abilities, cards,
};
use crate::{TargetIndex, mana_cost};

static AN_ARTIFACT_OR_ENCHANTMENT: [AbilityTargetDef; 1] =
    [AbilityTargetDef::exactly_one_permanent(
        ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Artifact),
            ObjectPredicateDef::HasType(CardType::Enchantment),
        ]),
    )];

// MID 10 — Cathar Commando
pub(in crate::card::sets) static CATHAR_COMMANDO: CardRecord = CardRecord::new(
    cards::CATHAR_COMMANDO,
    "Cathar Commando",
    CardArt::new("98cbc1c2-b76e-4da3-aa43-00e10b2ce532", "Evyn Fong"),
    CardSet::InnistradMidnightHunt,
    // Flash is what makes the two halves one card: it can be held up as
    // removal and cashed in as a 3/1 when nothing needs killing.
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Human", "Soldier"], 3, 1).with_abilities(&[
        abilities::flash(),
        AbilityDef::activated_with_targets(
            "{1}, Sacrifice this creature: Destroy target artifact or enchantment.",
            &[
                AbilityCostDef::Mana(mana_cost!("{1}")),
                AbilityCostDef::SacrificeSource,
            ],
            &AN_ARTIFACT_OR_ENCHANTMENT,
            EffectDef::destroy_target(TargetIndex::PRIMARY, true),
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&CATHAR_COMMANDO];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
