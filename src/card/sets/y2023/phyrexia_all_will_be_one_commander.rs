//! Phyrexia: All Will Be One Commander cards cataloged for the Vintage Cube
//! pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, CardArt, CardRules, CardSet, EffectDef, EffectRecipientDef,
    ObjectPredicateDef, TriggerEventDef, ValueDef, abilities,
};
use crate::mana_cost;

// ONC 6 — Glimmer Lens
pub(in crate::card::sets) static GLIMMER_LENS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("c9262000-e6f3-4da1-ad1c-038f65d3bef6"),
    "Glimmer Lens",
    CardArt::new(
        "c9262000-e6f3-4da1-ad1c-038f65d3bef6",
        "Sidharth Chaturvedi",
    ),
    CardSet::PhyrexiaAllWillBeOneCommander,
    CardRules::new_artifact(mana_cost!("{1}{W}"))
        .with_subtypes(&["Equipment"])
        .with_abilities(&[
            abilities::for_mirrodin(),
            // "And at least one other creature" is the whole declaration
            // being two or more: the Rebel it brought is one attacker, so
            // the card asks for a second body before it draws.
            AbilityDef::triggered(
                "Whenever equipped creature and at least one other creature attack, draw a card.",
                TriggerEventDef::attacks_in_declaration(
                    ObjectPredicateDef::AttachedToSource,
                    2,
                    None,
                ),
                EffectDef::DrawCards {
                    recipient: EffectRecipientDef::Controller,
                    amount: ValueDef::Constant(1),
                },
            ),
            abilities::equip(
                &[AbilityCostDef::Mana(mana_cost!("{1}{W}"))],
                "Equip {1}{W}",
            ),
        ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&GLIMMER_LENS];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
