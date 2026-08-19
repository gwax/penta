//! The Lord of the Rings: Tales of Middle-earth cards cataloged for the
//! Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    ObjectPredicateDef, TriggerEventDef, ValueDef, ZoneKind, abilities, cards,
};
use crate::mana_cost;

/// "Power or toughness 2 or less" is a disjunction, not a pair of bounds: a
/// 5/1 is small enough and a 1/5 is too. Written as "less than 3" because
/// that is the comparison the predicate offers.
static STERN_SCOLDING_TARGET: AbilityTargetDef =
    AbilityTargetDef::exactly_one_spell(ObjectPredicateDef::All(&[
        ObjectPredicateDef::HasType(CardType::Creature),
        ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::PowerLessThan(ValueDef::Constant(3)),
            ObjectPredicateDef::ToughnessLessThan(ValueDef::Constant(3)),
        ]),
    ]));

// LTR 71 — Stern Scolding
pub(in crate::card::sets) static STERN_SCOLDING: CardRecord = CardRecord::new(
    cards::STERN_SCOLDING,
    "Stern Scolding",
    CardArt::new("3ca1e1de-b916-445f-b3b2-0f4d0cc7ceeb", "Valera Lutfullina"),
    CardSet::LordOfTheRings,
    CardRules::new_instant(mana_cost!("{U}")).with_ability(AbilityDef::counter_target(
        "Counter target creature spell with power or toughness 2 or less.",
        &STERN_SCOLDING_TARGET,
    )),
);

// LTR 103 — Orcish Bowmasters
// Audit: blocked — Needs two things. A trigger on an opponent drawing a card, which no event here raises, and which this card qualifies further: every draw except the first one in each of that player's draw steps, so the count has to be kept per player per turn. And amass, which is a conditional token creation followed by a chosen Army taking counters and gaining a creature type.

// LTR 169 — Generous Ent
pub(in crate::card::sets) static GENEROUS_ENT: CardRecord = CardRecord::new(
    cards::GENEROUS_ENT,
    "Generous Ent",
    CardArt::new("85d22d5d-3875-42ff-b51e-c6e21db201f5", "Simon Dominic"),
    CardSet::LordOfTheRings,
    CardRules::new_creature(mana_cost!("{5}{G}"), &["Treefolk"], 5, 7).with_abilities(&[
        abilities::reach(),
        AbilityDef::triggered(
            "When this creature enters, create a Food token.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                None,
                Some(ZoneKind::Battlefield),
            ),
            EffectDef::CreateToken {
                token: cards::FOOD_TOKEN,
                count: ValueDef::Constant(1),
                tapped: false,
            },
        ),
        // Six mana is not what this card is for. Forestcycling is: one mana
        // from hand, and the Ent becomes the land the draw did not give you.
        abilities::typecycling(
            "Forestcycling {1} ({1}, Discard this card: Search your library for a Forest card, reveal it, put it into your hand, then shuffle.)",
            mana_cost!("{1}"),
            ObjectPredicateDef::Subtype("Forest"),
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&STERN_SCOLDING, &GENEROUS_ENT];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
