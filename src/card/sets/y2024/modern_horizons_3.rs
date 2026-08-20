//! Modern Horizons 3 cards cataloged as attachment edge cases.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AlternativeCastKindDef, AppliedEffectDef,
    CardArt, CardRules, CardSet, CardType, EffectDef, EffectPaymentCostDef, EffectPaymentDef,
    EffectRecipientDef, InstalledTriggerDef, ObjectPredicateDef, PayOrDef, PlayerRefDef,
    PlayerRelation, PlayerSetDef, TriggerConditionDef, TriggerEventDef, TurnStepDef, ValueDef,
    ZoneKind, ZonePlacement, abilities, cards,
};
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

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&STATIC_PRISON, &COLOSSAL_DREADMASK, &SOWING_MYCOSPAWN];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
