//! Urza's Legacy cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, BattlefieldEntryChoiceDestinationDef,
    BattlefieldEntryScalarChoiceDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, PlayerRelation, ReplacementChoiceDef,
    ReplacementEffectDef, ValueDef, ZoneKind, cards,
};
use crate::mana_cost;

/// Creatures of whatever type the Plague named. The chosen type lives on the
/// enchantment, so the predicate reads it from the ability's source rather
/// than naming a tribe the way a printed lord does.
static CREATURES_OF_THE_CHOSEN_TYPE: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::HasSourcesChosenScalar(BattlefieldEntryChoiceDestinationDef::CreatureType),
]);

// ULG 51 — Engineered Plague
pub(in crate::card::sets) static ENGINEERED_PLAGUE: CardRecord = CardRecord::new(
    cards::ENGINEERED_PLAGUE,
    "Engineered Plague",
    CardArt::new("27e158d5-efb2-4f90-8898-60ede98f7d29", "Michael Sutfin"),
    CardSet::UrzasLegacy,
    CardRules::new_enchantment(mana_cost!("{2}{B}")).with_abilities(&[
        AbilityDef::replacement(
            "As this enchantment enters, choose a creature type.",
            ReplacementEffectDef::Choose(ReplacementChoiceDef::Scalar(
                BattlefieldEntryScalarChoiceDef::CREATURE_TYPE,
            )),
        ),
        // Both players' creatures, which is what makes it a sideboard card
        // rather than a lord: it shrinks the mirror as readily as the matchup
        // it was brought in for.
        AbilityDef::static_ability(
            "All creatures of the chosen type get -1/-1.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::matching_objects(
                    CREATURES_OF_THE_CHOSEN_TYPE,
                    &[ZoneKind::Battlefield],
                    PlayerRelation::Any,
                ),
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(-1),
                    ValueDef::Constant(-1),
                ),
            },
        ),
    ]),
);

// ULG 125 — Defense Grid
pub(in crate::card::sets) static DEFENSE_GRID: CardRecord = CardRecord::new(
    cards::DEFENSE_GRID,
    "Defense Grid",
    CardArt::new("5c2592c9-3f8c-4b7e-9e0a-4a6f2c1d8b3e", "Mark Tedin"),
    CardSet::UrzasLegacy,
    // "Except during its controller's turn" is the nonactive player: the tax
    // lands on the instant held up and not on the sorcery cast on time.
    CardRules::new_artifact(mana_cost!("{2}")).with_ability(AbilityDef::static_ability(
        "Each spell costs {3} more to cast except during its controller's turn.",
        EffectDef::IncreaseMatchingSpellCostBy {
            spell: ObjectPredicateDef::Any,
            caster: PlayerRelation::NonactivePlayer,
            amount: mana_cost!("{3}"),
        },
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&ENGINEERED_PLAGUE, &DEFENSE_GRID];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
