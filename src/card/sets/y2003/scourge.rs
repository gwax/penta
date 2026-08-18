//! Scourge cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, ActivationTimingDef,
    AppliedEffectDef, BasicLandType, CardArt, CardRules, CardSet, EffectDef, EffectPaymentCostDef,
    EffectPaymentDef, EffectRecipientDef, ObjectPredicateDef, PayOrDef, PlayerRelation,
    PlayerSetDef, TriggerEventDef, ValueDef, ZoneKind, ZonePlacement, abilities, cards,
};
use crate::ids::TargetIndex;
use crate::mana_cost;

static GOBLIN_SPELLS: ObjectPredicateDef = ObjectPredicateDef::Subtype("Goblin");

/// The cycling half: X is settled by the payment rather than by a cast, so
/// the branch that makes the tokens reads back what was actually paid.
static DECREE_SOLDIERS: EffectDef = EffectDef::CreateToken {
    token: cards::SOLDIER_TOKEN_1_1_WHITE,
    count: ValueDef::PaidAmount,
    tapped: false,
};

static DECREE_CYCLING_TRIGGER: EffectDef = EffectDef::PayOr(PayOrDef::optional(
    EffectPaymentDef {
        payer: PlayerSetDef::Related(PlayerRelation::You),
        cost: EffectPaymentCostDef::ChosenGenericMana,
    },
    &DECREE_SOLDIERS,
));

// SCG 8 — Decree of Justice
pub(in crate::card::sets) static DECREE_OF_JUSTICE: CardRecord = CardRecord::new(
    cards::DECREE_OF_JUSTICE,
    "Decree of Justice",
    CardArt::new("5e8a7e5c-2a37-4e73-b5c9-b8a4b9d0b6e9", "Adam Rex"),
    CardSet::Scourge,
    // Cast for Angels when the game went long, cycled for Soldiers at the end
    // of an opponent's turn when it did not. Landstill wants the second mode
    // far more often than the first.
    CardRules::new_sorcery(mana_cost!("{X}{X}{2}{W}{W}")).with_abilities(&[
        AbilityDef::spell(
            "Create X 4/4 white Angel creature tokens with flying.",
            EffectDef::CreateToken {
                token: cards::ANGEL_TOKEN_4_4_WHITE,
                count: ValueDef::ChosenX,
                tapped: false,
            },
        ),
        abilities::cycling(
            "Cycling {2}{W} ({2}{W}, Discard this card: Draw a card.)",
            mana_cost!("{2}{W}"),
        ),
        AbilityDef::triggered(
            "When you cycle this card, you may pay {X}. If you do, create X 1/1 white Soldier creature tokens.",
            TriggerEventDef::Cycled,
            DECREE_CYCLING_TRIGGER,
        ),
    ]),
);

// SCG 12 — Eternal Dragon
pub(in crate::card::sets) static ETERNAL_DRAGON: CardRecord = CardRecord::new(
    cards::ETERNAL_DRAGON,
    "Eternal Dragon",
    CardArt::new("0596928c-2b20-4dbb-aa78-3ab6c3ce0d72", "Justin Sweet"),
    CardSet::Scourge,
    // Three cards in one: a land early, a threat late, and a threat again
    // every turn after that. Control decks play it as a one-of because it
    // never runs out.
    CardRules::new_creature(mana_cost!("{5}{W}{W}"), &["Dragon", "Spirit"], 5, 5).with_abilities(&[
        abilities::flying(),
        AbilityDef::activated(
            "{3}{W}{W}: Return this card from your graveyard to your hand. Activate only during your upkeep.",
            &[AbilityCostDef::Mana(mana_cost!("{3}{W}{W}"))],
            EffectDef::MoveToZone {
                object: EffectRecipientDef::Source,
                zone: ZoneKind::Hand,
                placement: ZonePlacement::Top,
                controller: None,
                arrival_effect: None,
            },
        )
        .with_source_zones(&[ZoneKind::Graveyard])
        .with_activation_timing(ActivationTimingDef::YourUpkeep),
        abilities::typecycling(
            "Plainscycling {2} ({2}, Discard this card: Search your library for a Plains card, reveal it, put it into your hand, then shuffle.)",
            mana_cost!("{2}"),
            ObjectPredicateDef::HasAnyBasicLandType(&[BasicLandType::Plains]),
        ),
    ]),
);

// SCG 97 — Goblin Warchief
pub(in crate::card::sets) static GOBLIN_WARCHIEF: CardRecord = CardRecord::new(
    cards::GOBLIN_WARCHIEF,
    "Goblin Warchief",
    CardArt::new(
        "66864a4b-8924-40ef-a337-15b12413a158",
        "Greg Hildebrandt & Tim Hildebrandt",
    ),
    CardSet::Scourge,
    // The haste is what makes the discount matter: a Goblin cast for one
    // less that also attacks the turn it lands.
    CardRules::new_creature(mana_cost!("{1}{R}{R}"), &["Goblin", "Warrior"], 2, 2).with_abilities(
        &[
            AbilityDef::static_ability(
                "Goblin spells you cast cost {1} less to cast.",
                EffectDef::ReduceMatchingSpellCostBy {
                    spell: GOBLIN_SPELLS,
                    caster: PlayerRelation::You,
                    amount: ValueDef::Constant(1),
                },
            ),
            AbilityDef::static_ability(
                "Goblins you control have haste.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::matching_objects(
                        ObjectPredicateDef::Subtype("Goblin"),
                        &[ZoneKind::Battlefield],
                        PlayerRelation::You,
                    ),
                    effect: AppliedEffectDef::add_ability(&abilities::haste()),
                },
            ),
        ],
    ),
);

// SCG 103 — Siege-Gang Commander
pub(in crate::card::sets) static SIEGE_GANG_COMMANDER: CardRecord = CardRecord::new(
    cards::SIEGE_GANG_COMMANDER,
    "Siege-Gang Commander",
    CardArt::new(
        "92e78cec-aaf9-4fe8-887b-b7e356d63315",
        "Christopher Moeller",
    ),
    CardSet::Scourge,
    // Four bodies for five mana, and the ability turns any of them --
    // including itself -- into two damage anywhere.
    CardRules::new_creature(mana_cost!("{3}{R}{R}"), &["Goblin"], 2, 2).with_abilities(&[
        AbilityDef::triggered(
            "When this creature enters, create three 1/1 red Goblin creature tokens.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                None,
                Some(ZoneKind::Battlefield),
            ),
            EffectDef::CreateToken {
                token: cards::GOBLIN_TOKEN_1_1_RED,
                count: ValueDef::Constant(3),
                tapped: false,
            },
        ),
        AbilityDef::activated_with_targets(
            "{1}{R}, Sacrifice a Goblin: This creature deals 2 damage to any target.",
            &[
                AbilityCostDef::Mana(mana_cost!("{1}{R}")),
                AbilityCostDef::SacrificePermanent {
                    object: ObjectPredicateDef::Subtype("Goblin"),
                    controller: PlayerRelation::You,
                },
            ],
            &[AbilityTargetDef::exactly_one(
                AbilityTargetPredicate::AnyTarget,
            )],
            EffectDef::DealDamage {
                recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                amount: ValueDef::Constant(2),
            },
        ),
    ]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &DECREE_OF_JUSTICE,
    &ETERNAL_DRAGON,
    &GOBLIN_WARCHIEF,
    &SIEGE_GANG_COMMANDER,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
