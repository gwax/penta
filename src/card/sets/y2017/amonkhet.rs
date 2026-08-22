//! Amonkhet cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, CardArt, CardRules, CardSet, CardType,
    EffectDef, EffectRecipientDef, ObjectPredicateDef, PlayerRelation, TriggerEventDef, ValueDef,
    ZoneKind, abilities,
};
use crate::{TargetIndex, mana_cost};

/// "Target non-Dragon creature an opponent controls." The exclusion is why
/// the card does not simply answer another Glorybringer, which is the whole
/// reason it is printed that way.
static A_NON_DRAGON_THEY_CONTROL: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::All(&[
            ObjectPredicateDef::HasType(CardType::Creature),
            ObjectPredicateDef::Not(&ObjectPredicateDef::Subtype("Dragon")),
        ]),
        zones: &[ZoneKind::Battlefield],
        controller: Some(PlayerRelation::Opponent),
        owner: None,
    },
)];

// AKH 134 — Glorybringer
pub(in crate::card::sets) static GLORYBRINGER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("3277ad99-5682-4baa-b106-de15721876a6"),
    "Glorybringer",
    CardArt::new("3277ad99-5682-4baa-b106-de15721876a6", "Sam Burley"),
    CardSet::Amonkhet,
    // Five mana that attacks the turn it lands for four in the air and kills
    // something on the way in. What exerting costs is the next attack, which
    // is the only thing keeping it honest.
    CardRules::new_creature(mana_cost!("{3}{R}{R}"), &["Dragon"], 4, 4).with_abilities(&[
        abilities::flying(),
        abilities::haste(),
        AbilityDef::triggered_with_targets(
            "You may exert this creature as it attacks. When you do, it deals 4 damage to target \
             non-Dragon creature an opponent controls.",
            TriggerEventDef::Exerted(ObjectPredicateDef::Source),
            &A_NON_DRAGON_THEY_CONTROL,
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(4),
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&GLORYBRINGER];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
