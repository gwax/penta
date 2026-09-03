//! SPM card records required by supported formats.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, BattlefieldEntryModificationDef, BattlefieldEntryScalarChoiceDef,
    CardArt, CardRules, CardSet, EffectDef, EffectPaymentDef, EffectRecipientDef, PlayerRelation,
    PlayerSetDef, ReplacementChoiceDef, ReplacementEffectDef,
};

// SPM 141 — Rhino's Rampage
// Audit: unsupported — Needs a reflexive excess-damage trigger that chooses its artifact target after the fight.
pub(in crate::card::sets) static RHINOS_RAMPAGE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f668817c-1cab-44c5-b6a8-95113e480d5e"),
    "Rhino's Rampage",
    CardArt::new("f668817c-1cab-44c5-b6a8-95113e480d5e", "Nino Is"),
    CardSet::MarvelsSpiderMan,
    CardRules::unsupported(),
);

// SPM 180 — Multiversal Passage
pub(in crate::card::sets) static MULTIVERSAL_PASSAGE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("21502958-a8e3-494a-9be9-bebbbb1dd9dc"),
    "Multiversal Passage",
    CardArt::new("f5fb426a-5618-4dd4-9c51-0cc847be8c1d", "Pablo Mendoza"),
    CardSet::MarvelsSpiderMan,
    CardRules::new_land(&[]).with_abilities(&[
        AbilityDef::replacement(
            "As this land enters, choose a basic land type. Then you may pay 2 life. If you \
             don't, it enters tapped.",
            ReplacementEffectDef::Sequence(&[
                ReplacementEffectDef::Choose(ReplacementChoiceDef::Scalar(
                    BattlefieldEntryScalarChoiceDef::BASIC_LAND_TYPE,
                )),
                ReplacementEffectDef::PayOr {
                    payment: EffectPaymentDef::life(PlayerSetDef::Related(PlayerRelation::You), 2),
                    if_paid: &[],
                    if_declined: &[ReplacementEffectDef::ModifyBattlefieldEntry(
                        BattlefieldEntryModificationDef::Tapped,
                    )],
                },
            ]),
        ),
        AbilityDef::static_ability(
            "This land is the chosen type.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::set_chosen_basic_land_type(),
            },
        ),
    ]),
);

// SPM 181 — Ominous Asylum
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static OMINOUS_ASYLUM: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("371b03a1-7707-4a8a-8c0e-0272418c801f"),
    "Ominous Asylum",
    CardArt::new("4329f94a-9110-4f07-b4a6-f1ccae97ccc9", "Pavel Kolomeyets"),
    CardSet::MarvelsSpiderMan,
    CardRules::unsupported(),
);

// SPM 183 — Savage Mansion
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SAVAGE_MANSION: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("c172cdb5-aa2c-419d-b8ab-4795f4b7e160"),
    "Savage Mansion",
    CardArt::new("855f59a5-17a8-4aca-8a4d-f98111eba14c", "David Álvarez"),
    CardSet::MarvelsSpiderMan,
    CardRules::unsupported(),
);

// SPM 184 — Sinister Hideout
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SINISTER_HIDEOUT: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("c417f8ce-e156-4c9a-af30-792606d861bd"),
    "Sinister Hideout",
    CardArt::new("23190d7e-5165-49bd-b307-bf81877d228d", "Pavel Kolomeyets"),
    CardSet::MarvelsSpiderMan,
    CardRules::unsupported(),
);

// SPM 185 — Suburban Sanctuary
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static SUBURBAN_SANCTUARY: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("cabf021b-23e9-404d-90c6-eef629e1283e"),
    "Suburban Sanctuary",
    CardArt::new("467df77a-a99c-4cfd-9af4-502eaa2eb2e3", "David Frasheski"),
    CardSet::MarvelsSpiderMan,
    CardRules::unsupported(),
);

// SPM 186 — University Campus
// Audit: unsupported — Card rules have not been implemented.
pub(in crate::card::sets) static UNIVERSITY_CAMPUS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("cd4b9fc5-fe3d-41d9-9d0e-77f1aebef618"),
    "University Campus",
    CardArt::new("2752f21c-f535-4772-a8b3-e97e1339e9c9", "David Álvarez"),
    CardSet::MarvelsSpiderMan,
    CardRules::unsupported(),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &RHINOS_RAMPAGE,
    &MULTIVERSAL_PASSAGE,
    &OMINOUS_ASYLUM,
    &SAVAGE_MANSION,
    &SINISTER_HIDEOUT,
    &SUBURBAN_SANCTUARY,
    &UNIVERSITY_CAMPUS,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
