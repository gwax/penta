//! Theros Beyond Death cards cataloged for the Vintage Cube.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCoverageDef, AbilityDef, CardArt, CardRules, CardSet, ComparisonDef, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, PlayerRelation, TopCardSelectionDef,
    TriggerConditionDef, TriggerEventDef, ValueComparisonDef, ValueDef, ZoneKind, ZonePlacement,
    cards,
};
use crate::mana_cost;

/// The check that ends games. Both sides are read as the trigger resolves,
/// which is what makes an empty library and a single blue permanent enough.
static DEVOTION_REACHES_THE_LIBRARY: ValueComparisonDef = ValueComparisonDef {
    left: ValueDef::DevotionTo(ManaColor::Blue),
    comparison: ComparisonDef::GreaterOrEqual,
    right: ValueDef::LibrarySize(PlayerRelation::You),
};

static ORACLE_WINS: TriggerConditionDef =
    TriggerConditionDef::ValueComparison(&DEVOTION_REACHES_THE_LIBRARY);

static ORACLE_WINS_THE_GAME: EffectDef = EffectDef::WinTheGame {
    player: EffectRecipientDef::Controller,
};

/// Looking is the smaller half. The rest going to the bottom in a random
/// order is written as an ordinary bottom placement: nothing in the pool
/// looks at the bottom of a library, so the order there is unobservable.
static ORACLE_LOOK: TopCardSelectionDef = TopCardSelectionDef {
    count: ValueDef::DevotionTo(ManaColor::Blue),
    object: None,
    minimum: 0,
    maximum: 1,
    select_all_matching: false,
    reveal_selected: false,
    selected_zone: ZoneKind::Library,
    selected_placement: ZonePlacement::Top,
    rest_zone: ZoneKind::Library,
    rest_placement: ZonePlacement::Bottom,
    selected_order_follows_choice: false,
    then: None,
    selected_face_down: false,
};

/// The look happens first and moves nothing out of the library, so the
/// comparison reads the same number either way round.
static ORACLE_ENTERS: [EffectDef; 2] = [
    EffectDef::LookAtTopAndSelect {
        player: EffectRecipientDef::Controller,
        looker: EffectRecipientDef::Controller,
        selection: &ORACLE_LOOK,
    },
    EffectDef::IfCondition {
        condition: &ORACLE_WINS,
        then: &ORACLE_WINS_THE_GAME,
    },
];

// THB 73 — Thassa's Oracle
pub(in crate::card::sets) static THASSAS_ORACLE: CardRecord = CardRecord::new(
    cards::THASSAS_ORACLE,
    "Thassa's Oracle",
    CardArt::new("13d7e352-4d01-4947-a76f-f8a01dd876cc", "Jesper Ejsing"),
    CardSet::TherosBeyondDeath,
    // Two blue mana and an empty library is the whole card. The looking is
    // what it does when the library is not empty yet.
    CardRules::new_creature(mana_cost!("{U}{U}"), &["Merfolk", "Wizard"], 1, 3).with_ability(
        AbilityDef::triggered(
            "When this creature enters, look at the top X cards of your library, where X is your devotion to blue. Put up to one of them on top of your library and the rest on the bottom of your library in a random order. If X is greater than or equal to the number of cards in your library, you win the game.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                None,
                Some(ZoneKind::Battlefield),
            ),
            EffectDef::Sequence(&ORACLE_ENTERS),
        )
        .with_coverage(AbilityCoverageDef::partial(
            "The cards put on the bottom go there in their library order rather than a random \
             one. Nothing in the pool looks at the bottom of a library, so the difference is \
             not observable from inside a game.",
        )),
    ),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&THASSAS_ORACLE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
