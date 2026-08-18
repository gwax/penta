//! Alliances cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef,
    CardArt, CardRules, CardSet, CardSupertype, CardType, DividedTotal, EffectDef,
    EffectRecipientDef, InstalledTriggerDef, ManaColor, ObjectPredicateDef, PlayerRelation,
    SpellAdditionalCostDef, SpendModeDef, TriggerEventDef, TurnStepDef, ValueDef, ZoneKind,
    ZonePlacement, abilities, cards,
};
use crate::{TargetIndex, mana_cost};

/// Four damage split however the caster likes. There is no printed ceiling on
/// the number of creatures, but the division supplies one anyway: every target
/// must be assigned at least one damage, so four is the most it can ever
/// reach.
static PYROKINESIS_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef {
    predicate: AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::HasType(CardType::Creature),
        zones: &[crate::card::ZoneKind::Battlefield],
        controller: None,
        owner: None,
    },
    minimum: 1,
    maximum: AbilityTargetDef::UNLIMITED,
    divided_total: Some(DividedTotal::Fixed(4)),
}];

/// Exiled from hand rather than discarded: the card is spent without ever
/// becoming a graveyard card, which is what "exile a red card" means.
static EXILE_A_RED_CARD: SpellAdditionalCostDef =
    SpellAdditionalCostDef::new(ObjectPredicateDef::Color(ManaColor::Red), ZoneKind::Hand, 1)
        .spent(SpendModeDef::Exile);

// ALL 78 — Pyrokinesis
pub(in crate::card::sets) static PYROKINESIS: CardRecord = CardRecord::new(
    cards::PYROKINESIS,
    "Pyrokinesis",
    CardArt::new("db2a5e85-6cbc-43c1-9362-4056ad017ef0", "Ron Spencer"),
    CardSet::Alliances,
    // The free cast is what the card is played for -- a blowout from an empty
    // board -- so the printed cost alone understates it considerably.
    CardRules::new_instant(mana_cost!("{4}{R}{R}")).with_abilities(&[
        AbilityDef::alternative_cast(
            mana_cost!("{0}"),
            AlternativeCastKindDef::AlternativeCost,
            Some("You may exile a red card from your hand rather than pay this spell's mana cost."),
            EffectDef::None,
        )
        .with_alternative_additional_cost(&EXILE_A_RED_CARD),
        AbilityDef::spell_with_targets(
            "Pyrokinesis deals 4 damage divided as you choose among any number of target creatures.",
            &PYROKINESIS_TARGETS,
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::DividedAmongTargets,
            },
        ),
    ]),
);

/// The land fetches, and then leaves: the return is a delayed trigger so
/// that the land is available to tap again next turn rather than staying to
/// be tapped twice in one.
static GLACIERS_RETURN: EffectDef =
    EffectDef::InstallTrigger(InstalledTriggerDef::once(&AbilityDef::triggered(
        "At the beginning of the next cleanup step, return this land to its owner's hand.",
        TriggerEventDef::StepBegins {
            step: TurnStepDef::Cleanup,
            player: PlayerRelation::Any,
        },
        EffectDef::MoveToZone {
            object: EffectRecipientDef::Source,
            zone: ZoneKind::Hand,
            controller: None,
            placement: ZonePlacement::Top,
            arrival_effect: None,
        },
    )));

static GLACIERS_FETCH: EffectDef = EffectDef::Sequence(&[
    EffectDef::SearchZone {
        player: EffectRecipientDef::Controller,
        source: ZoneKind::Library,
        object: ObjectPredicateDef::All(&[
            ObjectPredicateDef::HasType(CardType::Land),
            ObjectPredicateDef::Supertype(CardSupertype::Basic),
        ]),
        minimum: 0,
        maximum: ValueDef::Constant(1),
        reveal: false,
        destination: ZoneKind::Battlefield,
        placement: ZonePlacement::Top,
        shuffle: true,
        enters_tapped: true,
    },
    GLACIERS_RETURN,
]);

// ALL 144 — Thawing Glaciers
pub(in crate::card::sets) static THAWING_GLACIERS: CardRecord = CardRecord::new(
    cards::THAWING_GLACIERS,
    "Thawing Glaciers",
    CardArt::new("6411a8c6-010f-4863-a0fa-bbebe09d5c34", "Jeff A. Menges"),
    CardSet::Alliances,
    // One basic a turn, forever: slow enough that only a deck with nothing
    // better to do at end of turn wants it, which is exactly Landstill.
    CardRules::new_land(&[]).with_abilities(&[
        abilities::enters_tapped("This land enters tapped."),
        AbilityDef::activated(
            "{1}, {T}: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle. Return this land to its owner's hand at the beginning of the next cleanup step.",
            &[
                AbilityCostDef::Mana(mana_cost!("{1}")),
                AbilityCostDef::TapSource,
            ],
            GLACIERS_FETCH,
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&PYROKINESIS, &THAWING_GLACIERS];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
