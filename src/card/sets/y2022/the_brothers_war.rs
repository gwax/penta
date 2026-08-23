//! The Brothers' War cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef,
    CardArt, CardRules, CardSet, CardType, CharacteristicOperationDef, CreatureTypeSetDef,
    EffectDef, EffectRecipientDef, ObjectPredicateDef, PlayerRelation, SetOperationDef,
    TriggerEventDef, TurnStepDef, ValueDef, ZoneKind, ZonePlacement,
};
use crate::{TargetIndex, mana_cost};

/// "Noncreature artifact or noncreature enchantment." The two types are
/// alternatives and the exclusion applies to both, so it sits outside the
/// choice rather than inside it -- which is what leaves a creature that
/// happens to be an artifact alone.
static A_NONCREATURE_ARTIFACT_OR_ENCHANTMENT: [AbilityTargetDef; 1] =
    [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::All(&[
                ObjectPredicateDef::AnyOf(&[
                    ObjectPredicateDef::HasType(CardType::Artifact),
                    ObjectPredicateDef::HasType(CardType::Enchantment),
                ]),
                ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Creature)),
            ]),
            zones: &[ZoneKind::Battlefield],
            controller: None,
            owner: None,
        },
    )];

// BRO 199 — Haywire Mite
pub(in crate::card::sets) static HAYWIRE_MITE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("847a175e-ead1-4596-baf3-5f7f57859e0b"),
    "Haywire Mite",
    CardArt::new("847a175e-ead1-4596-baf3-5f7f57859e0b", "Izzy"),
    CardSet::TheBrothersWar,
    // One mana for a body that is never dead: it answers whichever artifact
    // or enchantment the format is afraid of this week, and every deck can
    // cast it whether or not it can pay the green.
    CardRules::new_artifact_creature(mana_cost!("{1}"), &["Insect"], 1, 1).with_abilities(&[
        AbilityDef::triggered(
            "When this creature dies, you gain 2 life.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                Some(ZoneKind::Battlefield),
                Some(ZoneKind::Graveyard),
            ),
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(2),
            },
        ),
        AbilityDef::activated_with_targets(
            "{G}, Sacrifice this creature: Exile target noncreature artifact or noncreature \
             enchantment.",
            &[
                AbilityCostDef::Mana(mana_cost!("{G}")),
                AbilityCostDef::SacrificeSource,
            ],
            &A_NONCREATURE_ARTIFACT_OR_ENCHANTMENT,
            EffectDef::MoveToZone {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                zone: ZoneKind::Exile,
                controller: None,
                placement: ZonePlacement::Top,
                arrival_effect: None,
                attachment: None,
                counters: None,
            },
        ),
    ]),
);

/// "It's a Phyrexian in addition to its other types." Added rather than set:
/// what comes back through the Portal keeps whatever it already was, and is
/// a Phyrexian as well.
static AS_A_PHYREXIAN: AppliedEffectDef =
    AppliedEffectDef::Characteristic(CharacteristicOperationDef::CreatureTypes(
        SetOperationDef::Add(CreatureTypeSetDef::named(&["Phyrexian"])),
    ));

/// Any graveyard, not only yours: the Portal is as happy to take back what
/// it made an opponent sacrifice as anything of your own.
static A_CREATURE_CARD_IN_A_GRAVEYARD: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::HasType(CardType::Creature),
        zones: &[ZoneKind::Graveyard],
        controller: None,
        owner: None,
    },
)];

// BRO 240 — Portal to Phyrexia
pub(in crate::card::sets) static PORTAL_TO_PHYREXIA: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("5f608efc-0dbc-4cc3-aadd-ed473bfc29ab"),
    "Portal to Phyrexia",
    CardArt::new("5f608efc-0dbc-4cc3-aadd-ed473bfc29ab", "Svetlin Velinov"),
    CardSet::TheBrothersWar,
    // Nine mana, and the game is over: three of their creatures die on the
    // way in and one comes back for you every upkeep afterwards.
    CardRules::new_artifact(mana_cost!("{9}")).with_abilities(&[
        AbilityDef::triggered(
            "When this artifact enters, each opponent sacrifices three creatures of their choice.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                None,
                Some(ZoneKind::Battlefield),
            ),
            EffectDef::SacrificeOfChoice {
                player: EffectRecipientDef::Opponent,
                object: ObjectPredicateDef::HasType(CardType::Creature),
                count: ValueDef::Constant(3),
                then: None,
                amount: crate::card::SacrificedAmountDef::Power,
                otherwise: None,
                optional: false,
            },
        ),
        AbilityDef::triggered_with_targets(
            "At the beginning of your upkeep, put target creature card from a graveyard onto the \
             battlefield under your control. It's a Phyrexian in addition to its other types.",
            TriggerEventDef::StepBegins {
                step: TurnStepDef::Upkeep,
                player: PlayerRelation::You,
            },
            &A_CREATURE_CARD_IN_A_GRAVEYARD,
            EffectDef::MoveToZone {
                counters: None,
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                zone: ZoneKind::Battlefield,
                placement: ZonePlacement::Top,
                arrival_effect: Some(&AS_A_PHYREXIAN),
                attachment: None,
                controller: Some(PlayerRelation::You),
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&HAYWIRE_MITE, &PORTAL_TO_PHYREXIA];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
