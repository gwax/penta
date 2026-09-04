//! ARB card records required by supported formats.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AppliedEffectDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, PlayerRelation, ResolvedEffectDurationDef,
    ValueDef, abilities,
};
use crate::mana_cost;

// ARB 29 — Soul Manipulation
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SOUL_MANIPULATION: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("bcd3cb05-c6f9-435a-a0e7-1f85da4a36eb"),
    "Soul Manipulation",
    crate::card::CardArt::new("bcd3cb05-c6f9-435a-a0e7-1f85da4a36eb", "Carl Critchlow"),
    crate::card::CardSet::AlaraReborn,
    crate::card::CardRules::unsupported(),
);

// ARB 95 — Putrid Leech
pub(in crate::card::sets) static PUTRID_LEECH: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("aaa47568-5668-4a9f-ad1c-9a13010ffc2b"),
    "Putrid Leech",
    CardArt::new("aaa47568-5668-4a9f-ad1c-9a13010ffc2b", "Dave Allsop"),
    CardSet::AlaraReborn,
    // A two-mana 4/4 that costs two life a turn to be one, and the life is
    // paid before blockers rather than after.
    CardRules::new_creature(mana_cost!("{B}{G}"), &["Zombie", "Leech"], 2, 2).with_ability(
        AbilityDef::activated(
            "Pay 2 life: This creature gets +2/+2 until end of turn. Activate only once each turn.",
            &[AbilityCostDef::PayLife(2)],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(2),
                    ValueDef::Constant(2),
                ),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        )
        .once_each_turn(),
    ),
);

// ARB 133 — Thopter Foundry
pub(in crate::card::sets) static THOPTER_FOUNDRY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("42b8d797-b01d-49cf-9818-d84bba17029d"),
    "Thopter Foundry",
    CardArt::new("42b8d797-b01d-49cf-9818-d84bba17029d", "Ralph Horsley"),
    CardSet::AlaraReborn,
    // Two mana for a machine that turns every spent artifact into a flier
    // and a life, which is why it is played beside the artifacts that come
    // back on their own.
    CardRules::new_artifact(mana_cost!("{W/B}{U}")).with_ability(AbilityDef::activated(
        "{1}, Sacrifice a nontoken artifact: Create a 1/1 blue Thopter artifact creature token \
         with flying. You gain 1 life.",
        &[
            AbilityCostDef::Mana(mana_cost!("{1}")),
            AbilityCostDef::SacrificePermanent {
                // "A nontoken artifact": the Thopters it makes are artifacts too, so
                // without that word the Foundry would eat its own output forever.
                object: ObjectPredicateDef::All(&[
                    ObjectPredicateDef::HasType(CardType::Artifact),
                    ObjectPredicateDef::Not(&ObjectPredicateDef::Token),
                ]),
                controller: PlayerRelation::You,
            },
        ],
        EffectDef::Sequence(&[
            EffectDef::create_artifact_creature_token(&["Thopter"], &[ManaColor::Blue], 1, 1)
                .with_abilities(&[abilities::flying()]),
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(1),
            },
        ]),
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&SOUL_MANIPULATION, &PUTRID_LEECH, &THOPTER_FOUNDRY];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
