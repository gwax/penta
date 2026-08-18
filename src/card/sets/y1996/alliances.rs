//! Alliances cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef, CardArt,
    CardRules, CardSet, CardType, DividedTotal, EffectDef, EffectRecipientDef, ManaColor,
    ObjectPredicateDef, SpellAdditionalCostDef, SpendModeDef, ValueDef, ZoneKind, cards,
};
use crate::{TargetIndex, mana_cost};

/// Four damage split however the caster likes. There is no printed ceiling on
/// the number of creatures, but the division supplies one anyway: every target
/// must be assigned at least one damage, so four is the most it can ever
/// reach.
static PYROKINESIS_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef {
    predicate: AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::HasType(CardType::Creature),
        zones: &[crate::card::ZoneKind::Battlefield],
        controller: None,
        owner: None,
    },
    minimum: 1,
    maximum: AbilityTargetDef::UNLIMITED,
    divided_total: Some(DividedTotal::Fixed(4)),
}];

/// Exiled from hand rather than discarded: the card is spent without ever
/// becoming a graveyard card, which is what "exile a red card" means.
static EXILE_A_RED_CARD: SpellAdditionalCostDef =
    SpellAdditionalCostDef::new(ObjectPredicateDef::Color(ManaColor::Red), ZoneKind::Hand, 1)
        .spent(SpendModeDef::Exile);

// ALL 78 — Pyrokinesis
pub(in crate::card::sets) static PYROKINESIS: CardRecord = CardRecord::new(
    cards::PYROKINESIS,
    "Pyrokinesis",
    CardArt::new("db2a5e85-6cbc-43c1-9362-4056ad017ef0", "Ron Spencer"),
    CardSet::Alliances,
    // The free cast is what the card is played for -- a blowout from an empty
    // board -- so the printed cost alone understates it considerably.
    CardRules::new_instant(mana_cost!("{4}{R}{R}")).with_abilities(&[
        AbilityDef::alternative_cast(
            mana_cost!("{0}"),
            AlternativeCastKindDef::AlternativeCost,
            Some("You may exile a red card from your hand rather than pay this spell's mana cost."),
            EffectDef::None,
        )
        .with_alternative_additional_cost(&EXILE_A_RED_CARD),
        AbilityDef::spell_with_targets(
            "Pyrokinesis deals 4 damage divided as you choose among any number of target creatures.",
            &PYROKINESIS_TARGETS,
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::DividedAmongTargets,
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&PYROKINESIS];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
