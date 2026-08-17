//! Weatherlight cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::cards;
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef, AppliedRuleDef,
    CardArt, CardRules, CardSet, CardType, EffectDef, EffectPaymentDef, EffectRecipientDef,
    ObjectPredicateDef, PayOrDef, PlayerRelation, PlayerSetDef, ResolvedEffectDurationDef,
    TriggerEventDef, ZoneKind,
};
use crate::{TargetIndex, mana_cost};

/// The artifact has to belong to the player being attacked, which in a
/// two-player game is the only opponent there is.
static DEFENDERS_ARTIFACT: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::HasType(CardType::Artifact),
        zones: &[ZoneKind::Battlefield],
        controller: Some(PlayerRelation::Opponent),
        owner: None,
    },
)];

/// Paying trades the hit for the artifact: the Vandal connects, and then
/// deals nothing because it spent the swing breaking something instead.
static VANDAL_TRADE: EffectDef = EffectDef::Sequence(&[
    EffectDef::Destroy {
        object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        can_regenerate: true,
    },
    EffectDef::Apply {
        recipient: EffectRecipientDef::Source,
        effect: AppliedEffectDef::Rule(AppliedRuleDef::AssignsNoCombatDamage),
        duration: ResolvedEffectDurationDef::UntilEndOfTurn,
    },
]);

// WTH 105 — Goblin Vandal
pub(in crate::card::sets) static GOBLIN_VANDAL: CardRecord = CardRecord::new(
    cards::GOBLIN_VANDAL,
    "Goblin Vandal",
    CardArt::new("b7ad3b81-f706-4b33-b1ec-7600182a5232", "Franz Vohwinkel"),
    CardSet::Weatherlight,
    CardRules::new_creature(mana_cost!("{R}"), &["Goblin", "Rogue"], 1, 1).with_ability(
        AbilityDef::triggered_with_targets(
            "Whenever this creature attacks and isn't blocked, you may pay {R}. If you do, destroy target artifact defending player controls and this creature assigns no combat damage this turn.",
            TriggerEventDef::AttacksAndIsNotBlocked {
                attacker: ObjectPredicateDef::Source,
            },
            &DEFENDERS_ARTIFACT,
            EffectDef::PayOr(PayOrDef::optional(
                EffectPaymentDef::mana(
                    PlayerSetDef::Related(PlayerRelation::You),
                    mana_cost!("{R}"),
                ),
                &VANDAL_TRADE,
            )),
        ),
    ),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&GOBLIN_VANDAL];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
