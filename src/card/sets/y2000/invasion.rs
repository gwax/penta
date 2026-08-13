//! Invasion cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AddManaEffectDef, CardArt, CardRules, CardSet, EffectDef,
    ManaColor, ValueDef, ZoneKind, abilities, cards,
};
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

// INV 321 — Coastal Tower
pub(in crate::card::sets) static COASTAL_TOWER: CardRecord = CardRecord::new(
    cards::COASTAL_TOWER,
    "Coastal Tower",
    CardArt::new("d115dbff-e35b-495f-a1e3-19651895927e", "Don Hazeltine"),
    CardSet::Invasion,
    CardRules::new_land(&[]).with_abilities(&[
        abilities::enters_tapped("This land enters tapped."),
        AbilityDef::activated_mana(
            "{T}: Add {W} or {U}.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::choice(&[
                ManaColor::White,
                ManaColor::Blue,
            ])),
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&FACT_OR_FICTION, &COASTAL_TOWER];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
