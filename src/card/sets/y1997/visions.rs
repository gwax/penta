//! Visions cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef, BasicLandType,
    CardArt, CardRules, CardSet, EffectDef, EffectRecipientDef, ObjectPredicateDef,
    SpellAdditionalCostDef, SpendModeDef, TopCardSelectionDef, ValueDef, ZoneKind, ZonePlacement,
    cards,
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
    spend: SpendModeDef::ByZone,
};

static FIREBLAST_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::AnyTarget,
)];

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

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&IMPULSE, &FIREBLAST];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
