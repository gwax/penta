//! Duskmourn: House of Horror cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AlternativeCastKindDef, AppliedEffectDef, BattlefieldEntryModificationDef, CardArt,
    CardRules, CardSet, CardType, CardTypeSet, ChoiceVisibilityDef, ChooseDef, CounterKind,
    EffectDef, EffectRecipientDef, ObjectChoiceBindingDef, ObjectPredicateDef, ObjectQueryDef,
    ObjectSetDef, PlayerRefDef, PlayerRelation, PlayerSetDef, ReplacementConditionDef,
    ReplacementEffectDef, SpellAdditionalCostDef, SpendModeDef, TriggerConditionDef,
    TriggerEventDef, TurnStepDef, ValueDef, ZoneKind, ZonePlacement, abilities, cards,
};
use crate::ids::ObjectSetBindingIndex;
use crate::mana_cost;

/// "Other creatures you control with power 2 or less", read as each one
/// enters. The cap below is what makes a batch of them draw one card.
static A_SMALL_CREATURE_YOU_CONTROL: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::Not(&ObjectPredicateDef::Source),
    ObjectPredicateDef::PowerLessThan(ValueDef::Constant(3)),
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
]);

/// What it comes back as. Setting the type line rather than adding to it is
/// what takes the creature away, and the effect lasts as long as the
/// permanent does -- so the next time it dies the clause below finds an
/// enchantment and leaves it in the graveyard.
static ENDURES_AS_AN_ENCHANTMENT: AppliedEffectDef =
    AppliedEffectDef::set_card_types(CardTypeSet::single(CardType::Enchantment));

static IT_WAS_A_CREATURE: TriggerConditionDef = TriggerConditionDef::SourceMatches {
    object: ObjectPredicateDef::HasType(CardType::Creature),
};

static INNOCENCE_RETURNS: EffectDef = EffectDef::MoveToZone {
    object: EffectRecipientDef::Source,
    zone: ZoneKind::Battlefield,
    placement: ZonePlacement::Top,
    controller: None,
    arrival_effect: Some(&ENDURES_AS_AN_ENCHANTMENT),
    attachment: None,
};

static ENDURING_INNOCENCE_ABILITIES: [AbilityDef; 3] = [
    abilities::lifelink(),
    AbilityDef::triggered(
        "Whenever one or more other creatures you control with power 2 or less enter, draw a \
         card. This ability triggers only once each turn.",
        TriggerEventDef::zone_changed(
            A_SMALL_CREATURE_YOU_CONTROL,
            None,
            Some(ZoneKind::Battlefield),
        ),
        EffectDef::DrawCards {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
        },
    )
    .triggering_at_most(1),
    AbilityDef::triggered_if(
        "When this creature dies, if it was a creature, return it to the battlefield under its \
         owner's control. It's an enchantment. (It's not a creature.)",
        TriggerEventDef::zone_changed(
            ObjectPredicateDef::Source,
            Some(ZoneKind::Battlefield),
            Some(ZoneKind::Graveyard),
        ),
        &IT_WAS_A_CREATURE,
        INNOCENCE_RETURNS,
    ),
];

// DSK 6 — Enduring Innocence
pub(in crate::card::sets) static ENDURING_INNOCENCE: CardRecord = CardRecord::new(
    cards::ENDURING_INNOCENCE,
    "Enduring Innocence",
    CardArt::new("6d908299-aac0-46a6-8fa5-780d5b3e0386", "Liiga Smilshkalne"),
    CardSet::DuskmournHouseOfHorror,
    // Answering it costs two cards: one to kill the creature and one for the
    // enchantment that gets up afterwards and keeps drawing.
    CardRules::new_enchantment_creature(mana_cost!("{1}{W}{W}"), &["Sheep", "Glimmer"], 2, 1)
        .with_abilities(&ENDURING_INNOCENCE_ABILITIES),
);

/// "A non-Avatar creature card or a planeswalker card." The Overlord itself
/// is an Avatar, which is what the exclusion is there for: it cannot buy
/// itself back.
static A_NON_AVATAR_CREATURE_OR_PLANESWALKER: ObjectPredicateDef = ObjectPredicateDef::AnyOf(&[
    ObjectPredicateDef::All(&[
        ObjectPredicateDef::HasType(CardType::Creature),
        ObjectPredicateDef::Not(&ObjectPredicateDef::Subtype("Avatar")),
    ]),
    ObjectPredicateDef::HasType(CardType::Planeswalker),
]);

static OVERLORD_TAKES_ONE: EffectDef = EffectDef::MoveToZone {
    object: EffectRecipientDef::objects(ObjectSetDef::Binding(ObjectSetBindingIndex::PRIMARY)),
    zone: ZoneKind::Hand,
    placement: ZonePlacement::Top,
    controller: None,
    arrival_effect: None,
    attachment: None,
};

/// The whole graveyard, not only what the mill just put there: the clause
/// says "from your graveyard" and means it.
static OVERLORD_CHOOSES: EffectDef = EffectDef::Choose(ChooseDef {
    binding: ObjectChoiceBindingDef::Objects(ObjectSetBindingIndex::PRIMARY),
    unchosen: None,
    chooser: PlayerRefDef::EffectController,
    candidates: ObjectSetDef::Query(ObjectQueryDef::owned_by(
        A_NON_AVATAR_CREATURE_OR_PLANESWALKER,
        &[ZoneKind::Graveyard],
        PlayerSetDef::Related(PlayerRelation::You),
    )),
    exclude: None,
    minimum: 0,
    maximum: 1,
    visibility: ChoiceVisibilityDef::Public,
    then: &OVERLORD_TAKES_ONE,
});

static OVERLORD_DIGS: [EffectDef; 2] = [
    EffectDef::Mill {
        player: EffectRecipientDef::Controller,
        amount: ValueDef::Constant(4),
        binding: None,
        then: None,
    },
    OVERLORD_CHOOSES,
];

static OVERLORD_EVENTS: [TriggerEventDef; 2] = [
    TriggerEventDef::zone_changed(
        ObjectPredicateDef::Source,
        None,
        Some(ZoneKind::Battlefield),
    ),
    TriggerEventDef::attacks(ObjectPredicateDef::Source),
];

static OVERLORD_ABILITIES: [AbilityDef; 4] = [
    AbilityDef::alternative_cast(
        mana_cost!("{1}{B}"),
        AlternativeCastKindDef::Impending,
        Some(
            "Impending 5—{1}{B} (If you cast this spell for its impending cost, it enters with \
             five time counters and isn't a creature until the last is removed. At the beginning \
             of your end step, remove a time counter from it.)",
        ),
        EffectDef::None,
    ),
    AbilityDef::as_enters_if(
        "If you cast this spell for its impending cost, it enters with five time counters.",
        ReplacementConditionDef::SourceCastWith(AlternativeCastKindDef::Impending),
        ReplacementEffectDef::ModifyBattlefieldEntry(
            BattlefieldEntryModificationDef::AddCounters {
                kind: CounterKind::Time,
                amount: 5,
            },
        ),
    ),
    AbilityDef::triggered(
        "At the beginning of your end step, remove a time counter from this permanent.",
        TriggerEventDef::StepBegins {
            step: TurnStepDef::End,
            player: PlayerRelation::You,
        },
        EffectDef::RemoveCounters {
            object: EffectRecipientDef::Source,
            kind: CounterKind::Time,
            amount: ValueDef::Constant(1),
        },
    ),
    AbilityDef::triggered(
        "Whenever this permanent enters or attacks, mill four cards, then you may return a \
         non-Avatar creature card or a planeswalker card from your graveyard to your hand.",
        TriggerEventDef::AnyOf(&OVERLORD_EVENTS),
        EffectDef::Sequence(&OVERLORD_DIGS),
    ),
];

/// Six cards out of your own graveyard, exiled to pay. Nothing is chosen
/// after the fact: the additional cost travels with the cast.
static EXILE_SIX_CARDS: SpellAdditionalCostDef =
    SpellAdditionalCostDef::new(ObjectPredicateDef::Any, ZoneKind::Graveyard, 6)
        .spent(SpendModeDef::Exile);

static OCULUS_ABILITIES: [AbilityDef; 3] = [
    AbilityDef::spell_with_additional_cost(
        "As an additional cost to cast this spell, exile six cards from your graveyard.",
        &[],
        EXILE_SIX_CARDS,
        EffectDef::None,
    ),
    abilities::flying(),
    AbilityDef::triggered(
        "At the beginning of each opponent's upkeep, manifest dread. (Look at the top two cards \
         of your library. Put one onto the battlefield face down as a 2/2 creature and the other \
         into your graveyard. Turn it face up any time for its mana cost if it's a creature \
         card.)",
        TriggerEventDef::StepBegins {
            step: TurnStepDef::Upkeep,
            player: PlayerRelation::Opponent,
        },
        EffectDef::ManifestDread {
            player: EffectRecipientDef::Controller,
        },
    ),
];

// DSK 42 — Abhorrent Oculus
pub(in crate::card::sets) static ABHORRENT_OCULUS: CardRecord = CardRecord::new(
    cards::ABHORRENT_OCULUS,
    "Abhorrent Oculus",
    CardArt::new("d2705b43-a94a-44c0-8740-82e0b296820c", "Bryan Sola"),
    CardSet::DuskmournHouseOfHorror,
    // A three-mana 5/5 flier for a deck that filled its own graveyard on
    // purpose, and a body every turn afterwards for nothing.
    CardRules::new_creature(mana_cost!("{2}{U}"), &["Eye"], 5, 5).with_abilities(&OCULUS_ABILITIES),
);

// DSK 113 — Overlord of the Balemurk
pub(in crate::card::sets) static OVERLORD_OF_THE_BALEMURK: CardRecord = CardRecord::new(
    cards::OVERLORD_OF_THE_BALEMURK,
    "Overlord of the Balemurk",
    CardArt::new("9b911653-7b96-4cf3-a907-13c5c53a14f7", "Babs Webb"),
    CardSet::DuskmournHouseOfHorror,
    // Two mana for the trigger now and a 5/5 five turns later, which is the
    // whole appeal: the enchantment does the work while the body waits.
    CardRules::new_enchantment_creature(mana_cost!("{3}{B}{B}"), &["Avatar", "Horror"], 5, 5)
        .with_abilities(&OVERLORD_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &ENDURING_INNOCENCE,
    &ABHORRENT_OCULUS,
    &OVERLORD_OF_THE_BALEMURK,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
