//! Stronghold cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AddManaEffectDef, CardArt, CardRules, CardSet,
    CardType, EffectDef, EffectPaymentCostDef, EffectPaymentDef, ObjectPredicateDef,
    PlayerRelation, PlayerSetDef, ReplacementEffectDef, ValueDef, ZoneKind, abilities, cards,
};
use crate::mana_cost;

// STH 36 — Mana Leak
pub(in crate::card::sets) static MANA_LEAK: CardRecord = CardRecord::new(
    cards::MANA_LEAK,
    "Mana Leak",
    CardArt::new("abcaf16d-aa02-43e2-aa38-bb1835d47a05", "Christopher Rush"),
    CardSet::Stronghold,
    CardRules::new_instant(mana_cost!("{1}{U}")).with_ability(AbilityDef::spell_with_targets(
        "Counter target spell unless its controller pays {3}.",
        &[AbilityTargetDef::exactly_one_spell(ObjectPredicateDef::Any)],
        abilities::counter_target_unless_paid(ValueDef::Constant(3)),
    )),
);

/// A land card from hand, which is the whole cost. A hand with none cannot
/// pay at all, and the Mox goes straight to the graveyard.
static A_LAND_CARD: ObjectPredicateDef = ObjectPredicateDef::HasType(CardType::Land);

static MOX_DIAMOND_ENTRY: ReplacementEffectDef = ReplacementEffectDef::PayOr {
    payment: EffectPaymentDef {
        payer: PlayerSetDef::Related(PlayerRelation::You),
        cost: EffectPaymentCostDef::DiscardMatching(A_LAND_CARD),
    },
    // Paying changes nothing about the entry: the Mox arrives as it was
    // going to. Declining is what redirects it.
    if_paid: &[],
    if_declined: &[ReplacementEffectDef::MoveToZone(ZoneKind::Graveyard)],
};

// STH 138 — Mox Diamond
pub(in crate::card::sets) static MOX_DIAMOND: CardRecord = CardRecord::new(
    cards::MOX_DIAMOND,
    "Mox Diamond",
    CardArt::new("28028830-83ed-45e2-b495-3b9ad9d3e988", "Dan Frazier"),
    CardSet::Stronghold,
    // Free mana that costs a land: the deck playing one is trading a card for
    // the turn it comes down.
    CardRules::new_artifact(mana_cost!("{0}")).with_abilities(&[
        AbilityDef::replacement(
            "If this artifact would enter, you may discard a land card instead. If you do, put this artifact onto the battlefield. If you don't, put it into its owner's graveyard.",
            MOX_DIAMOND_ENTRY,
        ),
        AbilityDef::activated_mana(
            "{T}: Add one mana of any color.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::any_color()),
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&MANA_LEAK, &MOX_DIAMOND];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
