//! Edge of Eternities cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityDef, AlternativeCastKindDef, CardArt, CardRules, CardSet, CardType, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, ObjectQueryDef, PlayerRelation, TopCardSelectionDef,
    TriggerConditionDef, ValueDef, ZoneKind, ZonePlacement,
};
use crate::mana_cost;

/// "Where X is the number of lands you control", which is the whole reason
/// the card is playable: it looks at more the longer the game goes.
static LANDS_YOU_CONTROL: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::HasType(CardType::Land),
    &[ZoneKind::Battlefield],
    PlayerRelation::You,
);

/// One selection differs from the other only in how many it keeps, so the
/// two are the same shape twice rather than a count the spell could carry:
/// how many cards a look takes is printed on it, and this card prints two
/// numbers.
const fn consult_selection(cards: u8) -> TopCardSelectionDef {
    TopCardSelectionDef {
        count: ValueDef::CountMatchingObjects(&LANDS_YOU_CONTROL),
        object: None,
        minimum: cards,
        maximum: cards,
        select_all_matching: false,
        reveal_selected: false,
        selected_zone: ZoneKind::Hand,
        selected_placement: ZonePlacement::Top,
        selected_face_down: None,
        rest_zone: ZoneKind::Library,
        rest_placement: ZonePlacement::Bottom,
        rest_random_order: true,
        selected_order_follows_choice: false,
        then: None,
    }
}

static CONSULT_ONE: TopCardSelectionDef = consult_selection(1);
static CONSULT_TWO: TopCardSelectionDef = consult_selection(2);

static CONSULT_WAS_KICKED: TriggerConditionDef =
    TriggerConditionDef::SourceCastWith(AlternativeCastKindDef::Kicked);

static CONSULT_NOT_KICKED: TriggerConditionDef = TriggerConditionDef::Not(&CONSULT_WAS_KICKED);

static CONSULT_LOOK_ONE: EffectDef = EffectDef::LookAtTopAndSelect {
    player: EffectRecipientDef::Controller,
    looker: EffectRecipientDef::Controller,
    selection: &CONSULT_ONE,
};

static CONSULT_LOOK_TWO: EffectDef = EffectDef::LookAtTopAndSelect {
    player: EffectRecipientDef::Controller,
    looker: EffectRecipientDef::Controller,
    selection: &CONSULT_TWO,
};

/// The two halves are complementary conditions on one fact rather than an
/// effect with a branch, so each reads the way its own printed clause does.
static CONSULT_EFFECT: [EffectDef; 2] = [
    EffectDef::IfCondition {
        condition: &CONSULT_NOT_KICKED,
        then: &CONSULT_LOOK_ONE,
    },
    EffectDef::IfCondition {
        condition: &CONSULT_WAS_KICKED,
        then: &CONSULT_LOOK_TWO,
    },
];

// EOE 51 — Consult the Star Charts
pub(in crate::card::sets) static CONSULT_THE_STAR_CHARTS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("a16a6555-2e3a-4587-aacd-0307d696b26c"),
    "Consult the Star Charts",
    CardArt::new(
        "a16a6555-2e3a-4587-aacd-0307d696b26c",
        "Antonio José Manzanedo",
    ),
    CardSet::EdgeOfEternities,
    // Two mana to dig as deep as your mana base, and four to keep twice as
    // much of what it finds.
    CardRules::new_instant(mana_cost!("{1}{U}")).with_abilities(&[
        AbilityDef::alternative_cast(
            mana_cost!("{2}{U}{U}"),
            AlternativeCastKindDef::Kicked,
            Some("Kicker {1}{U} (You may pay an additional {1}{U} as you cast this spell.)"),
            EffectDef::None,
        ),
        AbilityDef::spell(
            "Look at the top X cards of your library, where X is the number of lands you \
             control. Put one of those cards into your hand. If this spell was kicked, put two \
             of those cards into your hand instead. Put the rest on the bottom of your library \
             in a random order.",
            EffectDef::Sequence(&CONSULT_EFFECT),
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&CONSULT_THE_STAR_CHARTS];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
