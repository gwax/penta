//! Mirage cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, CardArt, CardRules, CardSet, CardType, EffectDef, EffectRecipientDef,
    ObjectPredicateDef, PlayerRelation, ZoneKind, cards,
};
use crate::mana_cost;

// MIR 245 — Tranquil Domain
pub(in crate::card::sets) static TRANQUIL_DOMAIN: CardRecord = CardRecord::new(
    cards::TRANQUIL_DOMAIN,
    "Tranquil Domain",
    CardArt::new(
        "801f34a6-9f22-43c2-b1e5-194395cc7da1",
        "D. Alexander Gregory",
    ),
    CardSet::Mirage,
    CardRules::new_instant(mana_cost!("{1}{G}")).with_ability(AbilityDef::spell(
        "Destroy all non-Aura enchantments.",
        EffectDef::Destroy {
            object: EffectRecipientDef::MatchingObjects {
                object: ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Enchantment),
                    ObjectPredicateDef::Not(&ObjectPredicateDef::Subtype("Aura")),
                ]),
                zones: &[ZoneKind::Battlefield],
                controller: PlayerRelation::Any,
            },
            can_regenerate: true,
        },
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&TRANQUIL_DOMAIN];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
