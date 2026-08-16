//! Urza's Saga cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, CardArt, CardChoiceSourceDef, CardRules, CardSet,
    CardType, EffectDef, EffectRecipientDef, ObjectPredicateDef, PlayerRelation, TriggerEventDef,
    ValueDef, ZoneKind, ZonePlacement, cards,
};
use crate::{TargetIndex, mana_cost};

// USG 21 — Monk Realist
pub(in crate::card::sets) static MONK_REALIST: CardRecord = CardRecord::new(
    cards::MONK_REALIST,
    "Monk Realist",
    CardArt::new("7a7fe9f1-f3c0-43e4-aa30-d0bdab4ae94d", "Daren Bader"),
    CardSet::UrzasSaga,
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Human", "Monk", "Cleric"], 1, 1).with_ability(
        AbilityDef::triggered_with_targets(
            "When this creature enters, destroy target enchantment.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                None,
                Some(ZoneKind::Battlefield),
            ),
            &[AbilityTargetDef::exactly_one_permanent(
                ObjectPredicateDef::HasType(CardType::Enchantment),
            )],
            EffectDef::destroy_target(TargetIndex::PRIMARY, true),
        ),
    ),
);

// USG 59 — Annul
pub(in crate::card::sets) static ANNUL: CardRecord = CardRecord::new(
    cards::ANNUL,
    "Annul",
    CardArt::new("3f8c73ff-be92-41ca-93a7-76f9823adb38", "Greg Simanson"),
    CardSet::UrzasSaga,
    CardRules::new_instant(mana_cost!("{U}")).with_ability(AbilityDef::counter_target(
        "Counter target artifact or enchantment spell.",
        &AbilityTargetDef::exactly_one_spell(ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Artifact),
            ObjectPredicateDef::HasType(CardType::Enchantment),
        ])),
    )),
);

/// "A Goblin permanent card": Gempalm Incinerator is a Goblin card that is
/// also a creature, and nothing in the pool is a Goblin instant, but the
/// clause names permanents rather than creatures and so does this.
static A_GOBLIN_PERMANENT_IN_HAND: [CardChoiceSourceDef; 1] =
    [CardChoiceSourceDef::Zone(ZoneKind::Hand)];

/// A minimum of zero is the "you may": the offer may be answered with
/// nothing, and with no Goblin in hand it is never made at all.
static GOBLIN_LACKEY_TRIGGER: EffectDef = EffectDef::ChooseCards {
    player: EffectRecipientDef::Controller,
    sources: &A_GOBLIN_PERMANENT_IN_HAND,
    object: ObjectPredicateDef::All(&[
        ObjectPredicateDef::Subtype("Goblin"),
        ObjectPredicateDef::Not(&ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Instant),
            ObjectPredicateDef::HasType(CardType::Sorcery),
        ])),
    ]),
    minimum: 0,
    maximum: 1,
    reveal: false,
    destination: ZoneKind::Battlefield,
    placement: ZonePlacement::Top,
};

// USG 190 — Goblin Lackey
pub(in crate::card::sets) static GOBLIN_LACKEY: CardRecord = CardRecord::new(
    cards::GOBLIN_LACKEY,
    "Goblin Lackey",
    CardArt::new("9b848caa-aad8-4060-8f86-304a8556de2d", "Jerry Tiritilli"),
    CardSet::UrzasSaga,
    // One connection puts a Siege-Gang Commander down for free, which is the
    // whole reason a 1/1 for one is a format staple.
    CardRules::new_creature(mana_cost!("{R}"), &["Goblin"], 1, 1).with_ability(
        AbilityDef::triggered(
            "Whenever this creature deals damage to a player, you may put a Goblin permanent card from your hand onto the battlefield.",
            TriggerEventDef::damage_to_player(ObjectPredicateDef::Source, PlayerRelation::Any),
            GOBLIN_LACKEY_TRIGGER,
        ),
    ),
);

// USG 191 — Goblin Matron
pub(in crate::card::sets) static GOBLIN_MATRON: CardRecord = CardRecord::new(
    cards::GOBLIN_MATRON,
    "Goblin Matron",
    CardArt::new("9e9e2e5d-ad06-4378-9afb-ffb174e6a5b4", "DiTerlizzi"),
    CardSet::UrzasSaga,
    // Any Goblin card, so it fetches the answer rather than the biggest
    // body: Tinkerer against artifacts, Ringleader for more cards.
    CardRules::new_creature(mana_cost!("{2}{R}"), &["Goblin"], 1, 1).with_ability(
        AbilityDef::triggered(
            "When this creature enters, you may search your library for a Goblin card, reveal that card, put it into your hand, then shuffle.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                None,
                Some(ZoneKind::Battlefield),
            ),
            EffectDef::SearchZone {
                player: EffectRecipientDef::Controller,
                source: ZoneKind::Library,
                object: ObjectPredicateDef::Subtype("Goblin"),
                minimum: 0,
                maximum: ValueDef::Constant(1),
                reveal: true,
                destination: ZoneKind::Hand,
                placement: ZonePlacement::Top,
                shuffle: true,
                enters_tapped: false,
            },
        ),
    ),
);

// USG 290 — Claws of Gix
pub(in crate::card::sets) static CLAWS_OF_GIX: CardRecord = CardRecord::new(
    cards::CLAWS_OF_GIX,
    "Claws of Gix",
    CardArt::new(
        "78372366-8c4c-46ac-bd7c-a735c2b24b5d",
        "Henry G. Higginbotham",
    ),
    CardSet::UrzasSaga,
    CardRules::new_artifact(mana_cost!("{0}")).with_ability(AbilityDef::activated(
        "{1}, Sacrifice a permanent: You gain 1 life.",
        &[
            AbilityCostDef::Mana(mana_cost!("{1}")),
            AbilityCostDef::SacrificePermanent {
                object: ObjectPredicateDef::Any,
                controller: PlayerRelation::You,
            },
        ],
        EffectDef::GainLife {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
        },
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &MONK_REALIST,
    &ANNUL,
    &GOBLIN_LACKEY,
    &GOBLIN_MATRON,
    &CLAWS_OF_GIX,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
