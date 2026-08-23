//! Innistrad: Crimson Vow cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AppliedEffectDef, CardArt, CardComposition, CardEffectStatus,
    CardPart, CardRules, CardSet, CardStructure, CardType, DoubleFacedKind, EffectDef,
    EffectRecipientDef, ObjectPredicateDef, PlayOptionDef, PlayerRelation, SpellForm, ValueDef,
    ZoneKind, abilities,
};
use crate::ids::{CardPartId, PlayOptionId};
use crate::mana_cost;

static ODDITY_TRAMPLE: AbilityDef = abilities::trample();
static ODDITY_HASTE: AbilityDef = abilities::haste();

/// What the back face hands the rest of the board. The keywords are the ones
/// it already has, which is the joke: the 8/8 makes everything else look
/// like a smaller version of itself.
static BEHEMOTH_GRANT: [AppliedEffectDef; 3] = [
    AppliedEffectDef::modify_power_toughness(ValueDef::Constant(1), ValueDef::Constant(1)),
    AppliedEffectDef::add_ability(&ODDITY_TRAMPLE),
    AppliedEffectDef::add_ability(&ODDITY_HASTE),
];

/// "Other creatures you control", which excludes the Behemoth itself: it
/// already has both keywords and does not need the counters.
static OTHER_CREATURES_YOU_CONTROL: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
]);

static ODDITY_ABILITIES: [AbilityDef; 3] = [
    abilities::trample(),
    abilities::haste(),
    AbilityDef::activated(
        "{5}{G}{G}: Transform this creature.",
        &[AbilityCostDef::Mana(mana_cost!("{5}{G}{G}"))],
        EffectDef::Transform {
            object: EffectRecipientDef::Source,
        },
    ),
];

static BEHEMOTH_ABILITIES: [AbilityDef; 3] = [
    abilities::trample(),
    abilities::haste(),
    AbilityDef::static_ability(
        "Other creatures you control get +1/+1 and have trample and haste.",
        EffectDef::StaticApply {
            recipient: EffectRecipientDef::matching_objects(
                OTHER_CREATURES_YOU_CONTROL,
                &[ZoneKind::Battlefield],
                PlayerRelation::You,
            ),
            effect: AppliedEffectDef::Composite(&BEHEMOTH_GRANT),
        },
    ),
];

const fn ulvenwald_oddity_rules() -> CardRules {
    CardRules::new_creature(mana_cost!("{2}{G}{G}"), &["Beast"], 4, 4)
        .with_abilities(&ODDITY_ABILITIES)
}

const fn ulvenwald_behemoth_rules() -> CardRules {
    CardRules::new_creature_without_mana_cost(&["Beast", "Horror"], 8, 8)
        .with_abilities(&BEHEMOTH_ABILITIES)
}

fn ulvenwald_composition() -> CardComposition {
    CardComposition {
        parts: vec![
            CardPart::new(
                CardPartId::PRIMARY,
                "Ulvenwald Oddity",
                ulvenwald_oddity_rules(),
            ),
            CardPart::new(
                CardPartId(1),
                "Ulvenwald Behemoth",
                ulvenwald_behemoth_rules(),
            ),
        ],
        structure: CardStructure::DoubleFaced {
            front: CardPartId::PRIMARY,
            back: CardPartId(1),
            kind: DoubleFacedKind::Transforming,
        },
        play_options: vec![PlayOptionDef::cast(
            PlayOptionId::DEFAULT,
            "Ulvenwald Oddity",
            SpellForm::Part(CardPartId::PRIMARY),
            mana_cost!("{2}{G}{G}"),
            CardEffectStatus::Implemented,
        )],
    }
}

// VOW 225 — Ulvenwald Oddity
pub(in crate::card::sets) static ULVENWALD_ODDITY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("5fdf5fc4-69c8-4a59-9095-c2feefb64371"),
    "Ulvenwald Oddity",
    CardArt::new("5fdf5fc4-69c8-4a59-9095-c2feefb64371", "Brent Hollowell"),
    CardSet::InnistradCrimsonVow,
    // Four mana for a hasty trampling 4/4, and a mana sink that turns it
    // into an 8/8 that makes every other creature bigger and hasty too.
    // Nothing about it asks for anything but lands.
    ulvenwald_oddity_rules(),
)
.with_composition(ulvenwald_composition);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&ULVENWALD_ODDITY];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
