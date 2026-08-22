//! Through the Omenpaths cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, BattlefieldEntryModificationDef, BattlefieldEntryScalarChoiceDef,
    CardArt, CardRules, CardSet, EffectDef, EffectPaymentDef, EffectRecipientDef, PlayerRelation,
    PlayerSetDef, ReplacementChoiceDef, ReplacementEffectDef,
};

/// Declining is what makes it a tapped land, so the branch that pays does
/// nothing at all and the branch that does not is the whole cost.
static PASSAGE_ENTERS_TAPPED: [ReplacementEffectDef; 1] =
    [ReplacementEffectDef::ModifyBattlefieldEntry(
        BattlefieldEntryModificationDef::Tapped,
    )];

static PASSAGE_PAID: [ReplacementEffectDef; 0] = [];

static PASSAGE_ENTRY: [ReplacementEffectDef; 2] = [
    ReplacementEffectDef::Choose(ReplacementChoiceDef::Scalar(
        BattlefieldEntryScalarChoiceDef::BASIC_LAND_TYPE,
    )),
    ReplacementEffectDef::PayOr {
        payment: EffectPaymentDef::life(PlayerSetDef::Related(PlayerRelation::You), 2),
        if_paid: &PASSAGE_PAID,
        if_declined: &PASSAGE_ENTERS_TAPPED,
    },
];

// OM1 181 — Multiversal Passage
pub(in crate::card::sets) static MULTIVERSAL_PASSAGE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("21502958-a8e3-494a-9be9-bebbbb1dd9dc"),
    "Multiversal Passage",
    CardArt::new("21502958-a8e3-494a-9be9-bebbbb1dd9dc", "Daren Bader"),
    CardSet::ThroughTheOmenpaths,
    // A shock land that is whichever basic type the hand actually wants,
    // which is a different card in a deck with two colours and in one with
    // five. The mana ability comes from the type rather than a printed
    // clause, so choosing is all there is to it.
    CardRules::new_land(&[]).with_abilities(&[
        AbilityDef::replacement(
            "As this land enters, choose a basic land type. Then you may pay 2 life. If you \
             don't, it enters tapped.",
            ReplacementEffectDef::Sequence(&PASSAGE_ENTRY),
        ),
        AbilityDef::static_ability(
            "This land is the chosen type.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::set_chosen_basic_land_type(),
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&MULTIVERSAL_PASSAGE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
