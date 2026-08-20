//! Modern Horizons 3 cards cataloged as attachment edge cases.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef,
    AppliedEffectDef, CardArt, CardComposition, CardEffectStatus, CardPart, CardRules, CardSet,
    CardStructure, CardSupertype, CardType, ComparisonDef, CounterKind, DoubleFacedKind, EffectDef,
    EffectPaymentCostDef, EffectPaymentDef, EffectRecipientDef, InstalledTriggerDef, ManaColor,
    ObjectPredicateDef, ObjectQueryDef, ObjectSetDef, PayOrDef, PlayOptionDef, PlayerRefDef,
    PlayerRelation, PlayerSetDef, SpellForm, TriggerConditionDef, TriggerEventDef, TurnStepDef,
    ValueDef, ZoneKind, ZonePlacement, abilities, cards,
};
use crate::ids::{CardPartId, PlayOptionId};
use crate::{TargetIndex, mana_cost};

/// "Until this enchantment leaves the battlefield" is one printed ability,
/// so the return rides on the same resolution as a delayed trigger rather
/// than appearing as a second clause the card does not print.
static PRISON_RETURNS_IT: AbilityDef = AbilityDef::triggered(
    "When this enchantment leaves the battlefield, return the exiled card to the battlefield under its owner's control.",
    TriggerEventDef::zone_changed(
        ObjectPredicateDef::Source,
        Some(ZoneKind::Battlefield),
        None,
    ),
    EffectDef::ReturnLinkedExiles {
        zone: ZoneKind::Battlefield,
        grant: None,
        controller: None,
        transformed: false,
    },
);

static PRISON_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Land)),
        zones: &[ZoneKind::Battlefield],
        controller: Some(PlayerRelation::Opponent),
        owner: None,
    },
)];

static PRISON_ENTERS: [EffectDef; 3] = [
    EffectDef::ExileLinkedToSource {
        object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    },
    EffectDef::InstallTrigger(InstalledTriggerDef::once(&PRISON_RETURNS_IT)),
    // The energy arrives with the exile rather than paying for it: the first
    // upkeep tax is already covered, and the second is not.
    EffectDef::AddEnergyCounters {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::Constant(2),
    },
];

static PRISON_SACRIFICE: EffectDef = EffectDef::Sacrifice {
    object: EffectRecipientDef::Source,
};

// MH3 44 — Static Prison
pub(in crate::card::sets) static STATIC_PRISON: CardRecord = CardRecord::new(
    cards::STATIC_PRISON,
    "Static Prison",
    CardArt::new("dd16222e-349c-4a2b-a7c8-8eb35a8ab332", "Jason A. Engle"),
    CardSet::ModernHorizons3,
    // One white answers anything, and the two energy it comes with buy two
    // more turns of holding it. After that the prison opens.
    CardRules::new_enchantment(mana_cost!("{W}")).with_abilities(&[
        AbilityDef::triggered_with_targets(
            "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield. You get {E}{E} (two energy counters).",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                None,
                Some(ZoneKind::Battlefield),
            ),
            &PRISON_TARGET,
            EffectDef::Sequence(&PRISON_ENTERS),
        ),
        AbilityDef::triggered(
            "At the beginning of your first main phase, sacrifice this enchantment unless you pay {E}.",
            TriggerEventDef::StepBegins {
                step: TurnStepDef::PrecombatMain,
                player: PlayerRelation::You,
            },
            EffectDef::PayOr(PayOrDef::unless(
                EffectPaymentDef {
                    payer: PlayerSetDef::One(PlayerRefDef::EffectController),
                    cost: EffectPaymentCostDef::Energy(1),
                },
                &PRISON_SACRIFICE,
            )),
        ),
    ]),
);

// MH3 148 — Colossal Dreadmask
pub(in crate::card::sets) static COLOSSAL_DREADMASK: CardRecord = CardRecord::new(
    cards::COLOSSAL_DREADMASK,
    "Colossal Dreadmask",
    CardArt::new("98164430-64c1-465f-b786-45753c965f44", "Caio Monteiro"),
    CardSet::ModernHorizons3,
    CardRules::new_artifact(mana_cost!("{4}{G}{G}"))
        .with_subtypes(&["Equipment"])
        .with_abilities(&[
            abilities::living_weapon(cards::GERM_TOKEN_0_0_BLACK),
            AbilityDef::static_ability(
                "Equipped creature gets +6/+6 and has trample.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::Composite(&[
                        AppliedEffectDef::modify_power_toughness(
                            ValueDef::Constant(6),
                            ValueDef::Constant(6),
                        ),
                        AppliedEffectDef::add_ability(&abilities::trample()),
                    ]),
                },
            ),
            abilities::equip(mana_cost!("{3}{G}{G}"), "Equip {3}{G}{G}"),
        ]),
);

/// The kicked half changes nothing about how the spell resolves: it costs
/// more, and the second cast trigger reads that fact. That is why the
/// alternative carries no instructions of its own.
static MYCOSPAWN_KICKED: TriggerConditionDef =
    TriggerConditionDef::SourceCastWith(AlternativeCastKindDef::Kicked);

static MYCOSPAWN_EXILE_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one_permanent(
    ObjectPredicateDef::HasType(CardType::Land),
)];

static MYCOSPAWN_ABILITIES: [AbilityDef; 4] = [
    // Devoid is the empty printed colour set below; the keyword is here so
    // the card says what it is.
    abilities::devoid(),
    AbilityDef::alternative_cast(
        mana_cost!("{4}{G}{C}"),
        AlternativeCastKindDef::Kicked,
        Some("Kicker {1}{C} (You may pay an additional {1}{C} as you cast this spell.)"),
        EffectDef::None,
    ),
    AbilityDef::triggered(
        "When you cast this spell, search your library for a land card, put it onto the battlefield, then shuffle.",
        TriggerEventDef::SpellCast(ObjectPredicateDef::Source),
        EffectDef::SearchZone {
            player: EffectRecipientDef::Controller,
            source: ZoneKind::Library,
            object: ObjectPredicateDef::HasType(CardType::Land),
            minimum: 0,
            maximum: ValueDef::Constant(1),
            reveal: false,
            destination: ZoneKind::Battlefield,
            placement: ZonePlacement::Top,
            shuffle: true,
            enters_tapped: false,
            binding: None,
            then: None,
        },
    ),
    AbilityDef::triggered_if_with_targets(
        "When you cast this spell, if it was kicked, exile target land.",
        TriggerEventDef::SpellCast(ObjectPredicateDef::Source),
        &MYCOSPAWN_KICKED,
        &MYCOSPAWN_EXILE_TARGET,
        EffectDef::MoveToZone {
            object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            zone: ZoneKind::Exile,
            controller: None,
            placement: ZonePlacement::Top,
            arrival_effect: None,
            attachment: None,
        },
    ),
];

// MH3 170 — Sowing Mycospawn
pub(in crate::card::sets) static SOWING_MYCOSPAWN: CardRecord = CardRecord::new(
    cards::SOWING_MYCOSPAWN,
    "Sowing Mycospawn",
    CardArt::new("cdfadb17-76ad-4d4d-9fa7-33c4b88b4c0a", "Slawomir Maniak"),
    CardSet::ModernHorizons3,
    // Four mana finds a land and six exiles one, and both happen on the cast
    // rather than on arrival -- so countering the creature does not stop
    // either of them.
    CardRules::new_creature(mana_cost!("{3}{G}"), &["Eldrazi", "Fungus"], 3, 3)
        .printed_colors(&[])
        .with_abilities(&MYCOSPAWN_ABILITIES),
);

/// The Cats that matter are the other ones: Ajani dying alongside them does
/// not turn him over, and neither does his own death.
static ANOTHER_CAT_YOU_CONTROL: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::Subtype("Cat"),
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
    ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
]);

/// "Exile Ajani, then return him to the battlefield transformed." One
/// resolution: the exile links him to himself and the return brings him
/// straight back on the other face, under his owner's control.
static AJANI_TURNS_OVER: [EffectDef; 2] = [
    EffectDef::ExileLinkedToSource {
        object: EffectRecipientDef::Source,
    },
    EffectDef::ReturnLinkedExiles {
        zone: ZoneKind::Battlefield,
        grant: None,
        controller: None,
        transformed: true,
    },
];

static CATS_YOU_CONTROL: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::Subtype("Cat"),
    &[ZoneKind::Battlefield],
    PlayerRelation::You,
);

/// "If you control a red permanent other than Ajani." Ajani himself is
/// white, so the clause is about a second permanent rather than about him.
static A_RED_PERMANENT_BESIDES_AJANI: TriggerConditionDef = TriggerConditionDef::ObjectCount {
    query: ObjectQueryDef::matching(
        ObjectPredicateDef::All(&[
            ObjectPredicateDef::Color(ManaColor::Red),
            ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
        ]),
        &[ZoneKind::Battlefield],
        PlayerRelation::You,
    ),
    comparison: ComparisonDef::GreaterOrEqual,
    amount: 1,
};

/// The reflexive "when you do" is folded into this resolution: the token is
/// made, and then the damage happens if the condition holds. What that
/// costs is the separate window between the two and the chance to decline
/// the damage; the target is named as the ability is activated instead of
/// after the token appears, and there is always a legal one because a
/// player is a legal target.
static AJANI_MAKES_A_CAT_AND_MAY_BURN: [EffectDef; 2] = [
    EffectDef::CreateToken {
        token: cards::CAT_WARRIOR_TOKEN_2_1_WHITE,
        count: ValueDef::Constant(1),
        tapped: false,
        attacking: false,
    },
    EffectDef::IfCondition {
        condition: &A_RED_PERMANENT_BESIDES_AJANI,
        then: &EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::CountMatchingObjects(&CREATURES_YOU_CONTROL),
        },
    },
];

static CREATURES_YOU_CONTROL: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::HasType(CardType::Creature),
    &[ZoneKind::Battlefield],
    PlayerRelation::You,
);

static AJANI_BURN_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::AnyTarget,
)];

/// The four types the ultimate lets each opponent keep one of. Order is the
/// printed order, which is the order the questions are asked in.
static AJANI_SPARED_TYPES: [CardType; 4] = [
    CardType::Artifact,
    CardType::Creature,
    CardType::Enchantment,
    CardType::Planeswalker,
];

static AJANI_TURNS_OVER_SEQUENCE: EffectDef = EffectDef::Sequence(&AJANI_TURNS_OVER);

static AJANI_PARIAH_ABILITIES: [AbilityDef; 2] = [
    AbilityDef::triggered(
        "When Ajani enters, create a 2/1 white Cat Warrior creature token.",
        TriggerEventDef::zone_changed(
            ObjectPredicateDef::Source,
            None,
            Some(ZoneKind::Battlefield),
        ),
        EffectDef::CreateToken {
            token: cards::CAT_WARRIOR_TOKEN_2_1_WHITE,
            count: ValueDef::Constant(1),
            tapped: false,
            attacking: false,
        },
    ),
    // One trigger per Cat rather than one per batch. Several Cats dying at
    // once fire it several times, and every firing after the first finds
    // Ajani already exiled and returned as a new object, so it has nothing
    // left to turn over.
    AbilityDef::triggered(
        "Whenever one or more other Cats you control die, you may exile Ajani, then return him to the battlefield transformed under his owner's control.",
        TriggerEventDef::zone_changed(
            ANOTHER_CAT_YOU_CONTROL,
            Some(ZoneKind::Battlefield),
            Some(ZoneKind::Graveyard),
        ),
        EffectDef::May {
            player: EffectRecipientDef::Controller,
            effect: &AJANI_TURNS_OVER_SEQUENCE,
        },
    ),
];

static AJANI_AVENGER_ABILITIES: [AbilityDef; 3] = [
    AbilityDef::activated(
        "+2: Put a +1/+1 counter on each Cat you control.",
        &AJANI_PLUS_TWO_COST,
        EffectDef::AddCounters {
            object: EffectRecipientDef::objects(ObjectSetDef::Query(CATS_YOU_CONTROL)),
            kind: CounterKind::PlusOnePlusOne,
            amount: ValueDef::Constant(1),
        },
    ),
    AbilityDef::activated_with_targets(
        "0: Create a 2/1 white Cat Warrior creature token. When you do, if you control a red permanent other than Ajani, he deals damage equal to the number of creatures you control to any target.",
        &AJANI_ZERO_COST,
        &AJANI_BURN_TARGET,
        EffectDef::Sequence(&AJANI_MAKES_A_CAT_AND_MAY_BURN),
    ),
    AbilityDef::activated(
        "−4: Each opponent chooses an artifact, a creature, an enchantment, and a planeswalker from among the nonland permanents they control, then sacrifices the rest.",
        &AJANI_MINUS_FOUR_COST,
        EffectDef::SacrificeKeepingOnePerType {
            player: EffectRecipientDef::Opponent,
            types: &AJANI_SPARED_TYPES,
        },
    ),
];

static AJANI_PLUS_TWO_COST: [AbilityCostDef; 1] = [AbilityCostDef::Loyalty(2)];
static AJANI_ZERO_COST: [AbilityCostDef; 1] = [AbilityCostDef::Loyalty(0)];
static AJANI_MINUS_FOUR_COST: [AbilityCostDef; 1] = [AbilityCostDef::Loyalty(-4)];

const fn ajani_nacatl_pariah_rules() -> CardRules {
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Cat", "Warrior"], 1, 2)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&AJANI_PARIAH_ABILITIES)
}

const fn ajani_nacatl_avenger_rules() -> CardRules {
    CardRules::new_planeswalker_without_mana_cost(&["Ajani"])
        .with_supertype(CardSupertype::Legendary)
        .with_starting_loyalty(3)
        .with_abilities(&AJANI_AVENGER_ABILITIES)
}

fn ajani_composition() -> CardComposition {
    CardComposition {
        parts: vec![
            CardPart::new(
                CardPartId::PRIMARY,
                "Ajani, Nacatl Pariah",
                ajani_nacatl_pariah_rules(),
            ),
            CardPart::new(
                CardPartId(1),
                "Ajani, Nacatl Avenger",
                ajani_nacatl_avenger_rules(),
            ),
        ],
        structure: CardStructure::DoubleFaced {
            front: CardPartId::PRIMARY,
            back: CardPartId(1),
            kind: DoubleFacedKind::Transforming,
        },
        play_options: vec![PlayOptionDef::cast(
            PlayOptionId::DEFAULT,
            "Ajani, Nacatl Pariah",
            SpellForm::Part(CardPartId::PRIMARY),
            mana_cost!("{1}{W}"),
            CardEffectStatus::Implemented,
        )],
    }
}

// MH3 237 — Ajani, Nacatl Pariah
pub(in crate::card::sets) static AJANI_NACATL_PARIAH: CardRecord = CardRecord::new(
    cards::AJANI_NACATL_PARIAH,
    "Ajani, Nacatl Pariah",
    CardArt::new("0d16e8e0-31b2-4389-afd6-783c501f6fa0", "Chris Rallis"),
    CardSet::ModernHorizons3,
    ajani_nacatl_pariah_rules(),
)
.with_composition(ajani_composition);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &STATIC_PRISON,
    &COLOSSAL_DREADMASK,
    &SOWING_MYCOSPAWN,
    &AJANI_NACATL_PARIAH,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
