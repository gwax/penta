//! `HarperPrism` Book Promo card records.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::AbilityCostDef;
use crate::AbilityDef;
use crate::AbilityTargetDef;
use crate::AbilityTargetPredicate;
use crate::AddManaEffectDef;
use crate::CardArt;
use crate::CardRules;
use crate::CardSet;
use crate::CardType;
use crate::EffectDef;
use crate::EffectRecipientDef;
use crate::LikelihoodDef;
use crate::ManaColor;
use crate::ObjectPredicateDef;
use crate::ObjectRefDef;
use crate::PlayerRelation;
use crate::TargetIndex;
use crate::TriggerEventDef;
use crate::TurnStepDef;
use crate::ValueDef;
use crate::ZoneKind;

use crate::mana_cost;

// PHPR 1 — Arena
pub(in crate::card::sets) static ARENA: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("2f989fda-2e54-427c-9154-4820c48abb02"),
    "Arena",
    CardArt::new("2f989fda-2e54-427c-9154-4820c48abb02", "Rob Alexander"),
    CardSet::HarperPrismBookPromos,
    CardRules::new_land(&[]).with_ability(AbilityDef::activated_with_targets(
        "{3}, {T}: Tap target creature you control and target creature of an opponent's choice they control. Those creatures fight each other.",
        &[
            AbilityCostDef::Mana(mana_cost!("{3}")),
            AbilityCostDef::TapSource,
        ],
        &[
            AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::HasType(CardType::Creature),
                zones: &[ZoneKind::Battlefield],
                controller: Some(PlayerRelation::You),
                owner: None,
            }),
            AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
                object: ObjectPredicateDef::HasType(CardType::Creature),
                zones: &[ZoneKind::Battlefield],
                controller: Some(PlayerRelation::You),
                owner: None,
            })
            .chosen_by_opponent(),
        ],
        EffectDef::Sequence(&[
            EffectDef::Tap {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            },
            EffectDef::Tap {
                object: EffectRecipientDef::Target(TargetIndex(1)),
            },
            EffectDef::Fight {
                first: ObjectRefDef::Target(TargetIndex::PRIMARY),
                second: ObjectRefDef::Target(TargetIndex(1)),
                excess: None,
            },
        ]),
    )),
);

// PHPR 2 — Sewers of Estark
// Audit: unsupported — Needs a duration-scoped replacement/prevention effect for “Choose target creature. If it's attacking, it can't be blocked this turn. If it's blocking, prevent all combat damage that would be dealt this combat by it and each creature it's blocking”.
pub(in crate::card::sets) static SEWERS_OF_ESTARK: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("b0da11d4-3603-4f59-8f61-7204bf04e165"),
    "Sewers of Estark",
    CardArt::new("b0da11d4-3603-4f59-8f61-7204bf04e165", "Melissa A. Benson"),
    CardSet::HarperPrismBookPromos,
    CardRules::unsupported(),
);

// PHPR 5 — Mana Crypt
pub(in crate::card::sets) static MANA_CRYPT: CardRecord = CardRecord::new_with_legacy_id(
    2142,
    "Mana Crypt",
    CardArt::new("0cb33b46-4d1b-4f97-bfdc-d815aee111da", "Matt Stewart"),
    CardSet::HarperPrismBookPromos,
    CardRules::new_artifact(mana_cost!("{0}")).with_abilities(&[
        AbilityDef::triggered(
            "At the beginning of your upkeep, flip a coin. If you lose the flip, this artifact deals 3 damage to you.",
            TriggerEventDef::StepBegins {
                step: TurnStepDef::Upkeep,
                player: PlayerRelation::You,
            },
            EffectDef::Randomized {
                likelihood: LikelihoodDef::new(0.5),
                on_success: &EffectDef::None,
                // Losing the flip is the whole cost of the card, and it is paid to the
                // artifact itself: three damage from a source its controller chose to keep
                // around.
                on_failure: &EffectDef::DealDamage {
                    recipient: EffectRecipientDef::Controller,
                    amount: ValueDef::Constant(3),
                },
            },
        ),
        AbilityDef::activated_mana(
            "{T}: Add {C}{C}.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::one(ManaColor::Colorless).with_amount(2)),
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&ARENA, &SEWERS_OF_ESTARK, &MANA_CRYPT];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
