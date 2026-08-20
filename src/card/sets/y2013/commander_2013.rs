//! Commander 2013 cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, PlayerRelation, ResolvedEffectDurationDef,
    SpellLifeCostDef, ValueDef, ZoneKind, cards,
};
use crate::mana_cost;

/// Every creature, whoever controls it, and the amount is the life its caster
/// was willing to spend. Held behind a reference because a negated value is
/// one word wider than the value it negates.
static TOXIC_DELUGE_AMOUNT: ValueDef = ValueDef::Negate(&ValueDef::ChosenX);

// C13 96 — Toxic Deluge
pub(in crate::card::sets) static TOXIC_DELUGE: CardRecord = CardRecord::new(
    cards::TOXIC_DELUGE,
    "Toxic Deluge",
    CardArt::new("564caf57-4ba5-4993-a35e-945699c94eb7", "Svetlin Velinov"),
    CardSet::Commander2013,
    CardRules::new_sorcery(mana_cost!("{2}{B}")).with_ability(
        AbilityDef::spell(
            "As an additional cost to cast this spell, pay X life.\nAll creatures get -X/-X until end of turn.",
            EffectDef::Apply {
                recipient: EffectRecipientDef::matching_objects(
                    ObjectPredicateDef::HasType(CardType::Creature),
                    &[ZoneKind::Battlefield],
                    PlayerRelation::Any,
                ),
                effect: AppliedEffectDef::modify_power_toughness(
                    TOXIC_DELUGE_AMOUNT,
                    TOXIC_DELUGE_AMOUNT,
                ),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        )
        .with_spell_life_cost(SpellLifeCostDef::variable()),
    ),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&TOXIC_DELUGE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
