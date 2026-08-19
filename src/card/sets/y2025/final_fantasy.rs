//! Final Fantasy cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, CardArt, CardComposition, CardEffectStatus, CardPart, CardRules,
    CardSet, CardStructure, CardSupertype, CardType, DamageEventMatcherDef, DamageKindDef,
    DamageRecipientMatcherDef, DamageSourceMatcherDef, DoubleFacedKind, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, ObjectRefDef, PlayOptionDef, PlayerRelation,
    ResolvedEffectDurationDef, SpellForm, TriggerConditionDef, TriggerEventDef, ValueDef, ZoneKind,
    abilities, cards,
};
use crate::{CardPartId, PlayOptionId, mana_cost};

/// The front half's payoff, and the reason the card is played: hitting hard
/// enough to halve your own life is what turns Cecil over. Untapping is part
/// of the same clause, so a Cecil that traded its attack for the transform
/// comes back ready to block.
static CECIL_TURNS_OVER: [EffectDef; 2] = [
    EffectDef::Untap {
        object: EffectRecipientDef::Source,
    },
    EffectDef::Transform {
        object: EffectRecipientDef::Source,
    },
];

static CECIL_TRANSFORM_CHECK: TriggerConditionDef =
    TriggerConditionDef::ControllerLifeAtMostHalfStartingLife;

static CECIL_TRANSFORMS: EffectDef = EffectDef::Sequence(&CECIL_TURNS_OVER);

/// "You lose that much life. Then if ..." is one clause resolving in order:
/// the life is lost first, so the very damage that cost it can be what brings
/// the total low enough to turn the card over.
static CECIL_DARKNESS: [EffectDef; 2] = [
    EffectDef::LoseLife {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::TriggerEventAmount,
    },
    EffectDef::IfCondition {
        condition: &CECIL_TRANSFORM_CHECK,
        then: &CECIL_TRANSFORMS,
    },
];

static CECIL_DARK_KNIGHT_ABILITIES: [AbilityDef; 2] = [
    abilities::deathtouch(),
    AbilityDef::triggered(
        "Darkness — Whenever Cecil deals damage, you lose that much life. Then if your life total is less than or equal to half your starting life total, untap Cecil and transform it.",
        TriggerEventDef::DamageDealt(DamageEventMatcherDef {
            kind: DamageKindDef::Any,
            source: DamageSourceMatcherDef::Object(ObjectRefDef::Source),
            recipient: DamageRecipientMatcherDef::Any,
        }),
        EffectDef::Sequence(&CECIL_DARKNESS),
    ),
];

const fn cecil_dark_knight_rules() -> CardRules {
    CardRules::new_creature(mana_cost!("{B}"), &["Human", "Knight"], 2, 3)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&CECIL_DARK_KNIGHT_ABILITIES)
}

/// "Other attacking creatures" excludes Cecil and takes in the opponent's
/// too, on the rare turn both sides are attacking at once.
static OTHER_ATTACKING_CREATURES: EffectRecipientDef = EffectRecipientDef::matching_objects(
    ObjectPredicateDef::All(&[
        ObjectPredicateDef::HasType(CardType::Creature),
        ObjectPredicateDef::Attacking,
        ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
    ]),
    &[ZoneKind::Battlefield],
    PlayerRelation::Any,
);

static INDESTRUCTIBLE: AbilityDef = abilities::indestructible();

static CECIL_REDEEMED_PALADIN_ABILITIES: [AbilityDef; 2] = [
    abilities::lifelink(),
    AbilityDef::triggered(
        "Protect — Whenever Cecil attacks, other attacking creatures gain indestructible until end of turn.",
        TriggerEventDef::attacks(ObjectPredicateDef::Source),
        EffectDef::Apply {
            recipient: OTHER_ATTACKING_CREATURES,
            effect: AppliedEffectDef::add_ability(&INDESTRUCTIBLE),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    ),
];

/// The back face has no printed mana cost and is white, where the front is
/// black: transforming changes the colour it defends in.
const fn cecil_redeemed_paladin_rules() -> CardRules {
    CardRules::new_creature_without_mana_cost(&["Human", "Knight"], 4, 4)
        .printed_colors(&[ManaColor::White])
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&CECIL_REDEEMED_PALADIN_ABILITIES)
}

fn cecil_composition() -> CardComposition {
    CardComposition {
        parts: vec![
            CardPart::new(
                CardPartId::PRIMARY,
                "Cecil, Dark Knight",
                cecil_dark_knight_rules(),
            ),
            CardPart::new(
                CardPartId(1),
                "Cecil, Redeemed Paladin",
                cecil_redeemed_paladin_rules(),
            ),
        ],
        structure: CardStructure::DoubleFaced {
            front: CardPartId::PRIMARY,
            back: CardPartId(1),
            kind: DoubleFacedKind::Transforming,
        },
        play_options: vec![PlayOptionDef::cast(
            PlayOptionId::DEFAULT,
            "Cecil, Dark Knight",
            SpellForm::Part(CardPartId::PRIMARY),
            mana_cost!("{B}"),
            CardEffectStatus::Implemented,
        )],
    }
}

// FIN 91 — Cecil, Dark Knight
pub(in crate::card::sets) static CECIL_DARK_KNIGHT: CardRecord = CardRecord::new(
    cards::CECIL_DARK_KNIGHT,
    "Cecil, Dark Knight",
    CardArt::new("026e7167-d665-43d0-a51e-8df2d68cdb5e", "Josu Hernaiz"),
    CardSet::FinalFantasy,
    cecil_dark_knight_rules(),
)
.with_composition(cecil_composition);

/// A land you control, not any land: the opponent's fetchland does nothing
/// for her.
static A_LAND_YOU_CONTROL: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Land),
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
]);

/// Doubling is +X/+0 where X is her power as this resolves, so two landfalls
/// in a turn compound: the second reads the size the first left behind.
static TIFA_DOUBLES: [AbilityDef; 2] = [
    abilities::trample(),
    AbilityDef::triggered(
        "Landfall — Whenever a land you control enters, double Tifa Lockhart's power until end of turn.",
        TriggerEventDef::zone_changed(A_LAND_YOU_CONTROL, None, Some(ZoneKind::Battlefield)),
        EffectDef::Apply {
            recipient: EffectRecipientDef::Source,
            effect: AppliedEffectDef::modify_power_toughness(
                ValueDef::SourcePower,
                ValueDef::Constant(0),
            ),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    ),
];

// FIN 206 — Tifa Lockhart
pub(in crate::card::sets) static TIFA_LOCKHART: CardRecord = CardRecord::new(
    cards::TIFA_LOCKHART,
    "Tifa Lockhart",
    CardArt::new("fb781323-2746-405d-a9b2-e778c037a6e9", "Laurel Austin"),
    CardSet::FinalFantasy,
    CardRules::new_creature(mana_cost!("{1}{G}"), &["Human", "Monk"], 1, 2)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&TIFA_DOUBLES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&CECIL_DARK_KNIGHT, &TIFA_LOCKHART];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
