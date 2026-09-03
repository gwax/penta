//! Khans of Tarkir cards cataloged as cross-format rules-engine test cases.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, CardRules, CardSet, CardSupertype, EffectDef, EffectRecipientDef, PlayerRelation,
    ReplacementEffectDef, ReplacementEventDef, TurnKindDef, ValueDef, ZoneKind, ZoneMoveCauseDef,
    abilities,
};
use crate::mana_cost;

// KTK 3 — Ainok Bond-Kin
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static AINOK_BOND_KIN: CardRecord = CardRecord::new(
    "Ainok Bond-Kin",
    "22d2a844-17fc-4628-9591-684555e98f7b",
    "Chris Rahn",
    crate::card::CardSet::KhansOfTarkir,
    crate::card::CardRules::unsupported(),
);

// KTK 22 — Seeker of the Way
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SEEKER_OF_THE_WAY: CardRecord = CardRecord::new(
    "Seeker of the Way",
    "3c17e350-44f7-4413-ad24-7c5d6616effd",
    "Craig J Spearing",
    crate::card::CardSet::KhansOfTarkir,
    crate::card::CardRules::unsupported(),
);

// KTK 59 — Treasure Cruise
pub(in crate::card::sets) static TREASURE_CRUISE: CardRecord = CardRecord::new(
    "Treasure Cruise",
    "7a59d4b1-6cf4-44ec-8a96-1bb7094fea21",
    "Cynthia Sheppard",
    CardSet::KhansOfTarkir,
    CardRules::new_sorcery(mana_cost!("{7}{U}")).with_abilities(&[
        abilities::delve(),
        AbilityDef::spell(
            "Draw three cards.",
            EffectDef::DrawCards {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(3),
            },
        ),
    ]),
);

// KTK 78 — Mardu Skullhunter
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static MARDU_SKULLHUNTER: CardRecord = CardRecord::new(
    "Mardu Skullhunter",
    "dd3ca5e7-96f3-4326-9315-34bb396a054c",
    "Jason Rainville",
    crate::card::CardSet::KhansOfTarkir,
    crate::card::CardRules::unsupported(),
);

// KTK 111 — Hordeling Outburst
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static HORDELING_OUTBURST: CardRecord = CardRecord::new(
    "Hordeling Outburst",
    "a5c1bf52-2737-423a-b340-07448afcaea6",
    "Zoltan Boros",
    crate::card::CardSet::KhansOfTarkir,
    crate::card::CardRules::unsupported(),
);

// KTK 118 — Monastery Swiftspear
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static MONASTERY_SWIFTSPEAR: CardRecord = CardRecord::new(
    "Monastery Swiftspear",
    "b81c6c8b-a9cf-4866-89ba-7f8ad077b836",
    "Steve Argyle",
    crate::card::CardSet::KhansOfTarkir,
    crate::card::CardRules::unsupported(),
);

// KTK 137 — Hooting Mandrills
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static HOOTING_MANDRILLS: CardRecord = CardRecord::new(
    "Hooting Mandrills",
    "090d678c-f0e4-4757-8900-93dfe67aefe9",
    "Mike Bierek",
    crate::card::CardSet::KhansOfTarkir,
    crate::card::CardRules::unsupported(),
);

// KTK 227 — Ugin's Nexus
pub(in crate::card::sets) static UGINS_NEXUS: CardRecord = CardRecord::new(
    "Ugin's Nexus",
    "94002868-a48a-4ea8-bfce-17257078f5db",
    "Sam Burley",
    CardSet::KhansOfTarkir,
    CardRules::new_artifact(mana_cost!("{5}"))
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&[
            AbilityDef::replacement_for(
                "If a player would begin an extra turn, that player skips that turn instead.",
                ReplacementEventDef::WouldBeginTurn {
                    player: PlayerRelation::Any,
                    kind: TurnKindDef::Extra,
                },
                ReplacementEffectDef::ReplaceEventWithNothing,
            ),
            AbilityDef::replacement_for(
                "If Ugin's Nexus would be put into a graveyard from the battlefield, instead exile it and take an extra turn after this one.",
                ReplacementEventDef::WouldMove {
                    from: Some(ZoneKind::Battlefield),
                    to: ZoneKind::Graveyard,
                    cause: ZoneMoveCauseDef::Any,
                },
                ReplacementEffectDef::Sequence(&[
                    ReplacementEffectDef::MoveToZone(ZoneKind::Exile),
                    ReplacementEffectDef::Perform(&EffectDef::TakeExtraTurn {
                        player: EffectRecipientDef::Controller,
                    }),
                ]),
            ),
        ]),
);

// KTK 242 — Scoured Barrens
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SCOURED_BARRENS: CardRecord = CardRecord::new(
    "Scoured Barrens",
    "0824a960-dd89-45c5-90f0-3ec9eb47d9ce",
    "Eytan Zana",
    crate::card::CardSet::KhansOfTarkir,
    crate::card::CardRules::unsupported(),
);

// KTK 246 — Tranquil Cove
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static TRANQUIL_COVE: CardRecord = CardRecord::new(
    "Tranquil Cove",
    "0f840bd2-c4f5-4ac4-918c-91b4feeb8783",
    "John Avon",
    crate::card::CardSet::KhansOfTarkir,
    crate::card::CardRules::unsupported(),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &AINOK_BOND_KIN,
    &SEEKER_OF_THE_WAY,
    &TREASURE_CRUISE,
    &MARDU_SKULLHUNTER,
    &HORDELING_OUTBURST,
    &MONASTERY_SWIFTSPEAR,
    &HOOTING_MANDRILLS,
    &UGINS_NEXUS,
    &SCOURED_BARRENS,
    &TRANQUIL_COVE,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
