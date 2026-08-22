//! The Brothers' War cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, CardArt, CardRules,
    CardSet, CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef, TriggerEventDef,
    ValueDef, ZoneKind, ZonePlacement,
};
use crate::{TargetIndex, mana_cost};

/// "Noncreature artifact or noncreature enchantment." The two types are
/// alternatives and the exclusion applies to both, so it sits outside the
/// choice rather than inside it -- which is what leaves a creature that
/// happens to be an artifact alone.
static A_NONCREATURE_ARTIFACT_OR_ENCHANTMENT: [AbilityTargetDef; 1] =
    [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::All(&[
                ObjectPredicateDef::AnyOf(&[
                    ObjectPredicateDef::HasType(CardType::Artifact),
                    ObjectPredicateDef::HasType(CardType::Enchantment),
                ]),
                ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Creature)),
            ]),
            zones: &[ZoneKind::Battlefield],
            controller: None,
            owner: None,
        },
    )];

// BRO 199 — Haywire Mite
pub(in crate::card::sets) static HAYWIRE_MITE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("847a175e-ead1-4596-baf3-5f7f57859e0b"),
    "Haywire Mite",
    CardArt::new("847a175e-ead1-4596-baf3-5f7f57859e0b", "Izzy"),
    CardSet::TheBrothersWar,
    // One mana for a body that is never dead: it answers whichever artifact
    // or enchantment the format is afraid of this week, and every deck can
    // cast it whether or not it can pay the green.
    CardRules::new_artifact_creature(mana_cost!("{1}"), &["Insect"], 1, 1).with_abilities(&[
        AbilityDef::triggered(
            "When this creature dies, you gain 2 life.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                Some(ZoneKind::Battlefield),
                Some(ZoneKind::Graveyard),
            ),
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(2),
            },
        ),
        AbilityDef::activated_with_targets(
            "{G}, Sacrifice this creature: Exile target noncreature artifact or noncreature \
             enchantment.",
            &[
                AbilityCostDef::Mana(mana_cost!("{G}")),
                AbilityCostDef::SacrificeSource,
            ],
            &A_NONCREATURE_ARTIFACT_OR_ENCHANTMENT,
            EffectDef::MoveToZone {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                zone: ZoneKind::Exile,
                controller: None,
                placement: ZonePlacement::Top,
                arrival_effect: None,
                attachment: None,
                counters: None,
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&HAYWIRE_MITE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
