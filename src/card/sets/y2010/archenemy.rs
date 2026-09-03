//! Archenemy card records.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::AbilityCostDef;
use crate::AbilityDef;
use crate::AbilityTargetDef;
use crate::BattlefieldEntryModificationDef;
use crate::CardArt;
use crate::CardRules;
use crate::CardSet;
use crate::CardType;
use crate::EffectDef;
use crate::EffectRecipientDef;
use crate::KeywordAbility;
use crate::ObjectPredicateDef;
use crate::ObjectRefDef;
use crate::TargetIndex;
use crate::ValueDef;
use crate::ZoneKind;
use crate::ZonePlacement;

use crate::mana_cost;

// ARC 22 — Reassembling Skeleton
pub(in crate::card::sets) static REASSEMBLING_SKELETON: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("655f983e-3b23-48ee-89d5-d01d469d5a6f"),
    "Reassembling Skeleton",
    crate::card::CardArt::new("75c219bc-a140-4ecd-953a-eef2cc552d58", "Austin Hsu"),
    crate::card::CardSet::Archenemy,
    CardRules::new_creature(mana_cost!("{1}{B}"), &["Skeleton", "Warrior"], 1, 1).with_ability(
        AbilityDef::activated(
            "{1}{B}: Return this card from your graveyard to the battlefield tapped.",
            &[AbilityCostDef::Mana(mana_cost!("{1}{B}"))],
            EffectDef::WithBattlefieldArrival {
                effect: &EffectDef::MoveToZone {
                    object: EffectRecipientDef::object(ObjectRefDef::Source),
                    zone: ZoneKind::Battlefield,
                    placement: ZonePlacement::Top,
                },
                arrival: crate::card::BattlefieldArrivalDef {
                    modifications: &[BattlefieldEntryModificationDef::Tapped],
                    ..crate::card::BattlefieldArrivalDef::DEFAULT
                },
            },
        )
        .with_source_zones(&[ZoneKind::Graveyard]),
    ),
);

// ARC 32 — Chandra's Outrage
pub(in crate::card::sets) static CHANDRAS_OUTRAGE: CardRecord = CardRecord::new_with_legacy_id(
    1199,
    "Chandra's Outrage",
    CardArt::new(
        "65d1b479-f6f6-4fec-a5a6-1a74d426fb13",
        "Christopher Moeller",
    ),
    CardSet::Archenemy,
    CardRules::new_instant(mana_cost!("{2}{R}{R}")).with_ability(
        AbilityDef::spell_with_targets(
            "Chandra's Outrage deals 4 damage to target creature and 2 damage to that creature's controller.",
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::HasType(CardType::Creature),
            )],
            EffectDef::Sequence(&[
                EffectDef::DealDamage {
                    recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    amount: ValueDef::Constant(4),
                },
                EffectDef::DealDamage {
                    recipient: EffectRecipientDef::ControllerOfTarget(TargetIndex::PRIMARY),
                    amount: ValueDef::Constant(2),
                },
            ]),
        ),
    ),
);

// ARC 65 — Plummet
pub(in crate::card::sets) static PLUMMET: CardRecord = CardRecord::new_with_legacy_id(
    1644,
    "Plummet",
    CardArt::new("a96d7d96-5a86-45ef-a30b-b11ece22f060", "Pete Venters"),
    CardSet::Archenemy,
    CardRules::new_instant(mana_cost!("{1}{G}")).with_ability(AbilityDef::spell_with_targets(
        "Destroy target creature with flying.",
        &[AbilityTargetDef::exactly_one_permanent(
            ObjectPredicateDef::All(&[
                ObjectPredicateDef::HasType(CardType::Creature),
                ObjectPredicateDef::HasKeyword(KeywordAbility::Flying),
            ]),
        )],
        EffectDef::Destroy {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            can_regenerate: true,
            then: None,
        },
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&REASSEMBLING_SKELETON, &CHANDRAS_OUTRAGE, &PLUMMET];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
