//! Mirrodin cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AppliedEffectDef, CardArt, CardRules, CardSet,
    CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef, ValueDef, ZoneKind, ZonePlacement,
    abilities,
};
use crate::{TargetIndex, mana_cost};

static SPELLBOMB_BOUNCE_COST: [AbilityCostDef; 2] = [
    AbilityCostDef::Mana(mana_cost!("{U}")),
    AbilityCostDef::SacrificeSource,
];

static SPELLBOMB_DRAW_COST: [AbilityCostDef; 2] = [
    AbilityCostDef::Mana(mana_cost!("{1}")),
    AbilityCostDef::SacrificeSource,
];

static A_CREATURE: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one_permanent(
    ObjectPredicateDef::HasType(CardType::Creature),
)];

// MRD 141 — Aether Spellbomb
pub(in crate::card::sets) static AETHER_SPELLBOMB: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f3792e8b-4ad7-4e2d-994c-c4eaac0fa55f"),
    "Aether Spellbomb",
    CardArt::new("f3792e8b-4ad7-4e2d-994c-c4eaac0fa55f", "Jim Nelson"),
    CardSet::Mirrodin,
    // One mana that answers a creature for a turn if it has to and replaces
    // itself if it does not, which is why it costs a deck nothing to play.
    CardRules::new_artifact(mana_cost!("{1}")).with_abilities(&[
        AbilityDef::activated_with_targets(
            "{U}, Sacrifice this artifact: Return target creature to its owner's hand.",
            &SPELLBOMB_BOUNCE_COST,
            &A_CREATURE,
            EffectDef::MoveToZone {
                counters: None,
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                zone: ZoneKind::Hand,
                placement: ZonePlacement::Top,
                controller: None,
                arrival_effect: None,
                attachment: None,
            },
        ),
        AbilityDef::activated(
            "{1}, Sacrifice this artifact: Draw a card.",
            &SPELLBOMB_DRAW_COST,
            EffectDef::DrawCards {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(1),
            },
        ),
    ]),
);

static GREAVES_HASTE: AbilityDef = abilities::haste();

static GREAVES_SHROUD: AbilityDef = abilities::shroud();

/// The two halves are why the card is played: haste makes the creature useful
/// the turn it arrives, and shroud makes it hard to answer -- including by
/// its own controller, who cannot target it either.
static GREAVES_GRANTS: [AppliedEffectDef; 2] = [
    AppliedEffectDef::add_ability(&GREAVES_HASTE),
    AppliedEffectDef::add_ability(&GREAVES_SHROUD),
];

// MRD 146 — Bonesplitter
// Audit: metadata-only — Card rules have not been implemented.
pub(in crate::card::sets) static BONESPLITTER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("ae31d513-7412-4467-b497-a7183ff29a42"),
    "Bonesplitter",
    crate::card::CardArt::new("465a7990-c9f9-4716-a833-fd41458b9cee", "Darrell Riche"),
    crate::card::CardSet::Mirrodin,
    crate::card::CardRules::unsupported(),
);

// MRD 199 — Lightning Greaves
pub(in crate::card::sets) static LIGHTNING_GREAVES: CardRecord = CardRecord::new_with_legacy_id(
    2170,
    "Lightning Greaves",
    CardArt::new("61a28870-cf78-4323-9d82-cee764067764", "Jeremy Jarvis"),
    CardSet::Mirrodin,
    // Equipping for nothing is the whole card: the Greaves move to whatever
    // just arrived, every turn, for as long as they are on the battlefield.
    CardRules::new_artifact(mana_cost!("{2}"))
        .with_subtypes(&["Equipment"])
        .with_abilities(&[
            AbilityDef::static_ability(
                "Equipped creature has haste and shroud.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::Composite(&GREAVES_GRANTS),
                },
            ),
            abilities::equip(&[AbilityCostDef::Mana(mana_cost!("{0}"))], "Equip {0}"),
        ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&AETHER_SPELLBOMB, &BONESPLITTER, &LIGHTNING_GREAVES];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
