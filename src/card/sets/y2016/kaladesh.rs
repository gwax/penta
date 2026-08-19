//! Kaladesh cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AddManaEffectDef, CardArt, CardRules, CardSet, EffectDef,
    ManaColor, abilities, cards,
};

/// The fastland cycle: untapped while the board is still small, an expensive
/// tapped land after that. Every one of the ten prints this same clause, and
/// only the colour pair below it differs.
static FAST_LAND_ENTERS: AbilityDef = abilities::fast_land_enters(
    "This land enters tapped unless you control two or fewer other lands.",
);

static BLOOMING_MARSH_ABILITIES: [AbilityDef; 2] = [
    FAST_LAND_ENTERS,
    AbilityDef::activated_mana(
        "{T}: Add {B} or {G}.",
        &[AbilityCostDef::TapSource],
        EffectDef::AddMana(AddManaEffectDef::choice(&[
            ManaColor::Black,
            ManaColor::Green,
        ])),
    ),
];

// KLD 243 — Blooming Marsh
pub(in crate::card::sets) static BLOOMING_MARSH: CardRecord = CardRecord::new(
    cards::BLOOMING_MARSH,
    "Blooming Marsh",
    CardArt::new("90da33d4-fe9c-42fe-b326-2fe337dc3ecd", "Adam Paquette"),
    CardSet::Kaladesh,
    CardRules::new_land(&[]).with_abilities(&BLOOMING_MARSH_ABILITIES),
);

static BOTANICAL_SANCTUM_ABILITIES: [AbilityDef; 2] = [
    FAST_LAND_ENTERS,
    AbilityDef::activated_mana(
        "{T}: Add {G} or {U}.",
        &[AbilityCostDef::TapSource],
        EffectDef::AddMana(AddManaEffectDef::choice(&[
            ManaColor::Green,
            ManaColor::Blue,
        ])),
    ),
];

// KLD 244 — Botanical Sanctum
pub(in crate::card::sets) static BOTANICAL_SANCTUM: CardRecord = CardRecord::new(
    cards::BOTANICAL_SANCTUM,
    "Botanical Sanctum",
    CardArt::new("8744471b-a528-47d9-84d0-4526273f55e9", "Christine Choi"),
    CardSet::Kaladesh,
    CardRules::new_land(&[]).with_abilities(&BOTANICAL_SANCTUM_ABILITIES),
);

static CONCEALED_COURTYARD_ABILITIES: [AbilityDef; 2] = [
    FAST_LAND_ENTERS,
    AbilityDef::activated_mana(
        "{T}: Add {W} or {B}.",
        &[AbilityCostDef::TapSource],
        EffectDef::AddMana(AddManaEffectDef::choice(&[
            ManaColor::White,
            ManaColor::Black,
        ])),
    ),
];

// KLD 245 — Concealed Courtyard
pub(in crate::card::sets) static CONCEALED_COURTYARD: CardRecord = CardRecord::new(
    cards::CONCEALED_COURTYARD,
    "Concealed Courtyard",
    CardArt::new("c8769e97-aee8-4466-a9d7-0f4245ae4a97", "Jung Park"),
    CardSet::Kaladesh,
    CardRules::new_land(&[]).with_abilities(&CONCEALED_COURTYARD_ABILITIES),
);

static INSPIRING_VANTAGE_ABILITIES: [AbilityDef; 2] = [
    FAST_LAND_ENTERS,
    AbilityDef::activated_mana(
        "{T}: Add {R} or {W}.",
        &[AbilityCostDef::TapSource],
        EffectDef::AddMana(AddManaEffectDef::choice(&[
            ManaColor::Red,
            ManaColor::White,
        ])),
    ),
];

// KLD 246 — Inspiring Vantage
pub(in crate::card::sets) static INSPIRING_VANTAGE: CardRecord = CardRecord::new(
    cards::INSPIRING_VANTAGE,
    "Inspiring Vantage",
    CardArt::new("160ac412-005f-48ca-a204-10207307c6c2", "Jonas De Ro"),
    CardSet::Kaladesh,
    CardRules::new_land(&[]).with_abilities(&INSPIRING_VANTAGE_ABILITIES),
);

static SPIREBLUFF_CANAL_ABILITIES: [AbilityDef; 2] = [
    FAST_LAND_ENTERS,
    AbilityDef::activated_mana(
        "{T}: Add {U} or {R}.",
        &[AbilityCostDef::TapSource],
        EffectDef::AddMana(AddManaEffectDef::choice(&[ManaColor::Blue, ManaColor::Red])),
    ),
];

// KLD 249 — Spirebluff Canal
pub(in crate::card::sets) static SPIREBLUFF_CANAL: CardRecord = CardRecord::new(
    cards::SPIREBLUFF_CANAL,
    "Spirebluff Canal",
    CardArt::new("4e587ea7-0632-4789-ba75-3c410da2bb96", "Adam Paquette"),
    CardSet::Kaladesh,
    CardRules::new_land(&[]).with_abilities(&SPIREBLUFF_CANAL_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &BLOOMING_MARSH,
    &BOTANICAL_SANCTUM,
    &CONCEALED_COURTYARD,
    &INSPIRING_VANTAGE,
    &SPIREBLUFF_CANAL,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
