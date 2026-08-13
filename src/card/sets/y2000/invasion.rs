//! Invasion cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{AbilityDef, CardArt, CardRules, CardSet, EffectDef, ValueDef, ZoneKind, cards};
use crate::{ZonePlacement, mana_cost};

// INV 57 — Fact or Fiction
pub(in crate::card::sets) static FACT_OR_FICTION: CardRecord = CardRecord::new(
    cards::FACT_OR_FICTION,
    "Fact or Fiction",
    CardArt::new(
        "7fd4d018-dcf3-4439-8445-02d66e44f7d3",
        "Terese Nielsen",
    ),
    CardSet::Invasion,
    CardRules::new_instant(mana_cost!("{3}{U}")).with_ability(AbilityDef::spell(
        "Reveal the top five cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard.",
        EffectDef::RevealAndSplitIntoPiles {
            count: ValueDef::Constant(5),
            rest: ZoneKind::Graveyard,
            placement: ZonePlacement::Top,
        },
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&FACT_OR_FICTION];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
