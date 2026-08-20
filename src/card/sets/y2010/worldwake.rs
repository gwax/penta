//! Worldwake cards cataloged for the Vintage Cube.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AddManaEffectDef, AppliedEffectDef, CardArt, CardChoiceSourceDef,
    CardRules, CardSet, CardType, CardTypeSet, ColorSet, CreatureTypeSetDef, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, ResolvedEffectDurationDef, TriggerEventDef,
    ValueDef, ZoneKind, ZonePlacement, abilities, cards,
};
use crate::mana_cost;

static AN_EQUIPMENT_IN_HAND: [CardChoiceSourceDef; 1] = [CardChoiceSourceDef::Zone(ZoneKind::Hand)];

/// The second half of the card, and the reason the first half is worth
/// finding: a minimum of zero is the printed "you may", and with no
/// Equipment in hand the choice is never offered at all.
static MYSTIC_PUT_EQUIPMENT_DOWN: EffectDef = EffectDef::ChooseCards {
    player: EffectRecipientDef::Controller,
    sources: &AN_EQUIPMENT_IN_HAND,
    object: ObjectPredicateDef::Subtype("Equipment"),
    minimum: 0,
    maximum: 1,
    reveal: false,
    destination: ZoneKind::Battlefield,
    placement: ZonePlacement::Top,
    // It arrives as itself: nothing about the Equipment changes on the way
    // down, and it is not attached to anything.
    arrival_effect: None,
};

// WWK 20 — Stoneforge Mystic
pub(in crate::card::sets) static STONEFORGE_MYSTIC: CardRecord = CardRecord::new(
    cards::STONEFORGE_MYSTIC,
    "Stoneforge Mystic",
    CardArt::new("19557351-b65f-4b04-b971-66abdc07000a", "Mike Bierek"),
    CardSet::Worldwake,
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Kor", "Artificer"], 1, 2)
        .with_abilities(&[
            AbilityDef::triggered(
                "When this creature enters, you may search your library for an Equipment card, reveal it, put it into your hand, then shuffle.",
                TriggerEventDef::zone_changed(
                    ObjectPredicateDef::Source,
                    None,
                    Some(ZoneKind::Battlefield),
                ),
                EffectDef::May {
                    player: EffectRecipientDef::Controller,
                    effect: &EffectDef::SearchZone {
                        player: EffectRecipientDef::Controller,
                        source: ZoneKind::Library,
                        object: ObjectPredicateDef::Subtype("Equipment"),
                        minimum: 0,
                        maximum: ValueDef::Constant(1),
                        reveal: true,
                        destination: ZoneKind::Hand,
                        placement: ZonePlacement::Top,
                        shuffle: true,
                        enters_tapped: false,
                        binding: None,
                        then: None,
                    },
                },
            ),
            AbilityDef::activated(
                "{1}{W}, {T}: You may put an Equipment card from your hand onto the battlefield.",
                &[
                    AbilityCostDef::Mana(mana_cost!("{1}{W}")),
                    AbilityCostDef::TapSource,
                ],
                MYSTIC_PUT_EQUIPMENT_DOWN,
            ),
        ]),
);

static COLONNADE_FLYING: AbilityDef = abilities::flying();

static COLONNADE_VIGILANCE: AbilityDef = abilities::vigilance();

/// "It's still a land" is the type being added rather than set: everything
/// else about the animation replaces, and the land stays a land.
static COLONNADE_ANIMATION: [AppliedEffectDef; 6] = [
    AppliedEffectDef::add_card_types(CardTypeSet::single(CardType::Creature)),
    AppliedEffectDef::set_creature_types(CreatureTypeSetDef::named(&["Elemental"])),
    AppliedEffectDef::set_colors(ColorSet::from_colors(&[ManaColor::White, ManaColor::Blue])),
    AppliedEffectDef::set_base_power_toughness(ValueDef::Constant(4), ValueDef::Constant(4)),
    AppliedEffectDef::add_ability(&COLONNADE_FLYING),
    AppliedEffectDef::add_ability(&COLONNADE_VIGILANCE),
];

static COLONNADE_COLORS: [ManaColor; 2] = [ManaColor::White, ManaColor::Blue];

// WWK 133 — Celestial Colonnade
pub(in crate::card::sets) static CELESTIAL_COLONNADE: CardRecord = CardRecord::new(
    cards::CELESTIAL_COLONNADE,
    "Celestial Colonnade",
    CardArt::new("f6929259-2903-4f6f-9b06-42048fd55c6a", "Eric Deschamps"),
    CardSet::Worldwake,
    // A land that costs you a turn and then wins the game on its own, which
    // is the trade every control deck in the format is happy to make.
    CardRules::new_land(&[]).with_abilities(&[
        abilities::enters_tapped("This land enters tapped."),
        AbilityDef::activated_mana(
            "{T}: Add {W} or {U}.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::choice(&COLONNADE_COLORS)),
        ),
        AbilityDef::activated(
            "{3}{W}{U}: Until end of turn, this land becomes a 4/4 white and blue Elemental \
             creature with flying and vigilance. It's still a land.",
            &[AbilityCostDef::Mana(mana_cost!("{3}{W}{U}"))],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::Composite(&COLONNADE_ANIMATION),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&STONEFORGE_MYSTIC, &CELESTIAL_COLONNADE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
