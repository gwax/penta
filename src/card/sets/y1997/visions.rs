//! Visions cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef, BasicLandType,
    CardArt, CardRules, CardSet, CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef,
    PlayerRefDef, PlayerRelation, SpellAdditionalCostDef, SpendModeDef, TopCardSelectionDef,
    ValueDef, ZoneKind, ZonePlacement, cards,
};
use crate::{TargetIndex, mana_cost};

static IMPULSE_SELECTION: TopCardSelectionDef = TopCardSelectionDef {
    count: ValueDef::Constant(4),
    object: None,
    minimum: 1,
    maximum: 1,
    select_all_matching: false,
    reveal_selected: false,
    selected_zone: ZoneKind::Hand,
    selected_placement: ZonePlacement::Top,
    rest_zone: ZoneKind::Library,
    rest_placement: ZonePlacement::Bottom,
    selected_order_follows_choice: false,
    then: None,
};

// VIS 34 — Impulse
pub(in crate::card::sets) static IMPULSE: CardRecord = CardRecord::new(
    cards::IMPULSE,
    "Impulse",
    CardArt::new("9d710a97-062f-4773-b6c6-8aeddeb3b6e8", "Bryan Talbot"),
    CardSet::Visions,
    CardRules::new_instant(mana_cost!("{1}{U}")).with_ability(AbilityDef::spell(
        "Look at the top four cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order.",
        EffectDef::LookAtTopAndSelect {
            player: EffectRecipientDef::Controller,
            looker: EffectRecipientDef::Controller,
            selection: &IMPULSE_SELECTION,
        },
    )),
);

/// Two Mountains off the battlefield, which is why the card is a finisher
/// rather than a burn spell: it is cast from an empty board on the turn the
/// lands stop mattering.
static SACRIFICE_TWO_MOUNTAINS: SpellAdditionalCostDef = SpellAdditionalCostDef {
    object: ObjectPredicateDef::HasAnyBasicLandType(&[BasicLandType::Mountain]),
    zone: ZoneKind::Battlefield,
    count: 2,
    count_is_x: false,
    spend: SpendModeDef::ByZone,
};

static FIREBLAST_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::AnyTarget,
)];

static VISION_CHARM_PLAYER: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Player(PlayerRelation::Any),
)];

static VISION_CHARM_ARTIFACT: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one_permanent(
    ObjectPredicateDef::HasType(CardType::Artifact),
)];

/// The printed first choice is "a land type", which includes the nonbasic
/// ones. Nothing in this card pool carries a nonbasic land subtype, so the
/// choice offered is over the basic types alone.
static VISION_CHARM_MODES: [AbilityDef; 3] = [
    AbilityDef::spell_with_targets(
        "Target player mills four cards.",
        &VISION_CHARM_PLAYER,
        EffectDef::Mill {
            player: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(4),
        },
    ),
    AbilityDef::spell(
        "Choose a land type and a basic land type. Each land of the first chosen type becomes the second chosen type until end of turn.",
        EffectDef::SubstituteBasicLandTypeUntilEndOfTurn {
            chooser: PlayerRefDef::EffectController,
        },
    ),
    AbilityDef::spell_with_targets(
        "Target artifact phases out.",
        &VISION_CHARM_ARTIFACT,
        EffectDef::PhaseOut {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        },
    ),
];

// VIS 49 — Vision Charm
pub(in crate::card::sets) static VISION_CHARM: CardRecord = CardRecord::new(
    cards::VISION_CHARM,
    "Vision Charm",
    CardArt::new("0efaa72c-8f65-4488-ad66-80dc877166cc", "Greg Spalenka"),
    CardSet::Visions,
    // One blue for whichever of three the turn calls for. The deck wants the
    // land mode to strand an opponent's colours, and the phase-out to answer
    // an artifact at instant speed.
    CardRules::new_instant(mana_cost!("{U}")).with_ability(AbilityDef::modal_spell(
        "Choose one —\n• Target player mills four cards.\n• Choose a land type and a basic land type. Each land of the first chosen type becomes the second chosen type until end of turn.\n• Target artifact phases out.",
        &VISION_CHARM_MODES,
        1,
        1,
        false,
    )),
);

// VIS 72 — Vampiric Tutor
pub(in crate::card::sets) static VAMPIRIC_TUTOR: CardRecord = CardRecord::new(
    cards::VAMPIRIC_TUTOR,
    "Vampiric Tutor",
    CardArt::new("0a07cba3-2e8d-48ec-a6f8-4d2edfcd833d", "Gary Leach"),
    CardSet::Visions,
    CardRules::new_instant(mana_cost!("{B}")).with_ability(AbilityDef::spell(
        "Search your library for a card, then shuffle and put that card on top. You lose 2 life.",
        EffectDef::Sequence(&VAMPIRIC_TUTOR_EFFECT),
    )),
);

static VAMPIRIC_TUTOR_EFFECT: [EffectDef; 2] = [
    EffectDef::SearchZone {
        player: EffectRecipientDef::Controller,
        source: ZoneKind::Library,
        object: ObjectPredicateDef::Any,
        minimum: 0,
        maximum: ValueDef::Constant(1),
        reveal: false,
        destination: ZoneKind::Library,
        placement: ZonePlacement::Top,
        shuffle: true,
        enters_tapped: false,
        binding: None,
        then: None,
    },
    EffectDef::LoseLife {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::Constant(2),
    },
];

// VIS 79 — Fireblast
pub(in crate::card::sets) static FIREBLAST: CardRecord = CardRecord::new(
    cards::FIREBLAST,
    "Fireblast",
    CardArt::new("b1eb5b2c-1f02-48a6-a287-88eb189d6780", "Michael Danza"),
    CardSet::Visions,
    CardRules::new_instant(mana_cost!("{4}{R}{R}")).with_abilities(&[
        AbilityDef::spell_with_targets(
            "Fireblast deals 4 damage to any target.",
            &FIREBLAST_TARGET,
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(4),
            },
        ),
        AbilityDef::alternative_cast(
            crate::mana_cost!("{0}"),
            AlternativeCastKindDef::AlternativeCost,
            Some("You may sacrifice two Mountains rather than pay this spell's mana cost."),
            EffectDef::None,
        )
        .with_alternative_additional_cost(&SACRIFICE_TWO_MOUNTAINS),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&IMPULSE, &VISION_CHARM, &VAMPIRIC_TUTOR, &FIREBLAST];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
