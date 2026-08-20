//! Fifth Dawn cards cataloged for the Vintage Cube.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, AppliedRuleDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, PlayActionMatcherDef, PlayRestrictionDef, cards,
};
use crate::mana_cost;

/// A permission rather than a prohibition, in the same vocabulary: which
/// action it opens, and which cards it opens it for.
static CRUCIBLE_PERMISSION: PlayRestrictionDef = PlayRestrictionDef::new(
    PlayActionMatcherDef::PlayLand,
    ObjectPredicateDef::HasType(CardType::Land),
);

// 5DN 114 — Crucible of Worlds
pub(in crate::card::sets) static CRUCIBLE_OF_WORLDS: CardRecord = CardRecord::new(
    cards::CRUCIBLE_OF_WORLDS,
    "Crucible of Worlds",
    CardArt::new("312a6058-de08-487d-95bd-b3c56807fdd6", "Ron Spencer"),
    CardSet::FifthDawn,
    // One line, and it turns every fetchland, every Wasteland, and every
    // land anything made you discard back into a land drop.
    CardRules::new_artifact(mana_cost!("{3}")).with_ability(AbilityDef::static_ability(
        "You may play lands from your graveyard.",
        EffectDef::StaticApply {
            recipient: EffectRecipientDef::Controller,
            effect: AppliedEffectDef::Rule(AppliedRuleDef::MayPlayFromGraveyard(
                CRUCIBLE_PERMISSION,
            )),
        },
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&CRUCIBLE_OF_WORLDS];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
