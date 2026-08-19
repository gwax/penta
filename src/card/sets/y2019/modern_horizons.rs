//! Modern Horizons cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef, CardArt,
    CardRules, CardSet, CardType, EffectDef, EffectRecipientDef, ManaColor, ObjectPredicateDef,
    PlayerRelation, SpellAdditionalCostDef, SpendModeDef, TriggerConditionDef, ZoneKind, cards,
};
use crate::{TargetIndex, mana_cost};

/// Exiled rather than discarded: the card is spent without ever becoming a
/// graveyard card, which is what "exile a green card" means.
static EXILE_A_GREEN_CARD: SpellAdditionalCostDef = SpellAdditionalCostDef::new(
    ObjectPredicateDef::Color(ManaColor::Green),
    ZoneKind::Hand,
    1,
)
.spent(SpendModeDef::Exile);

/// "If it's not your turn" gates only the free cast. The printed cost is
/// always available, which is why this is a condition on the alternative
/// rather than a restriction on the card.
static NOT_YOUR_TURN: TriggerConditionDef =
    TriggerConditionDef::ActivePlayer(PlayerRelation::Opponent);

static FORCE_OF_VIGOR_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::up_to(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Artifact),
            ObjectPredicateDef::HasType(CardType::Enchantment),
        ]),
        zones: &[ZoneKind::Battlefield],
        controller: None,
        owner: None,
    },
    2,
)];

// MH1 164 — Force of Vigor
pub(in crate::card::sets) static FORCE_OF_VIGOR: CardRecord = CardRecord::new(
    cards::FORCE_OF_VIGOR,
    "Force of Vigor",
    CardArt::new("017c415b-d635-43c6-92b8-8c95d1c4ff8d", "Randy Vargas"),
    CardSet::ModernHorizons1,
    CardRules::new_instant(mana_cost!("{2}{G}{G}")).with_abilities(&[
        AbilityDef::alternative_cast(
            mana_cost!("{0}"),
            AlternativeCastKindDef::AlternativeCost,
            Some(
                "If it's not your turn, you may exile a green card from your hand rather than pay this spell's mana cost.",
            ),
            EffectDef::None,
        )
        .with_alternative_additional_cost(&EXILE_A_GREEN_CARD)
        .with_alternative_condition(&NOT_YOUR_TURN),
        AbilityDef::spell_with_targets(
            "Destroy up to two target artifacts and/or enchantments.",
            &FORCE_OF_VIGOR_TARGETS,
            EffectDef::Destroy {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                can_regenerate: true,
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&FORCE_OF_VIGOR];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
