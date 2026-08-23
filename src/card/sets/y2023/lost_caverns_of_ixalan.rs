//! Lost Caverns of Ixalan cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AppliedEffectDef, CardArt, CardRules,
    CardSet, CardSupertype, CardType, ChoiceVisibilityDef, ChooseDef, CounterKind, EffectDef,
    EffectPaymentCostDef, EffectPaymentDef, EffectRecipientDef, ExilePlayDurationDef,
    InstalledTriggerDef, ObjectChoiceBindingDef, ObjectPredicateDef, ObjectQueryDef, ObjectRefDef,
    ObjectSetDef, PayOrDef, PlayerRefDef, PlayerRelation, PlayerSetDef, ResolvedEffectDurationDef,
    SpellAdditionalCostDef, TriggerEventDef, ValueDef, ZoneKind, abilities, tokens,
};
use crate::ids::ObjectBindingIndex;
use crate::{TargetIndex, mana_cost};

/// "Until this creature leaves the battlefield" is one printed ability, so
/// the return rides on the same resolution as a delayed trigger rather than
/// appearing as a second clause the card does not print.
static BAT_RETURNS_IT: AbilityDef = AbilityDef::triggered(
    "When this creature leaves the battlefield, return the exiled card to its owner's hand.",
    TriggerEventDef::zone_changed(
        ObjectPredicateDef::Source,
        Some(ZoneKind::Battlefield),
        None,
    ),
    EffectDef::ReturnLinkedExiles {
        object: ObjectPredicateDef::Any,
        counters: None,
        arrival_effect: None,
        zone: ZoneKind::Hand,
        grant: None,
        controller: None,
        transformed: false,
    },
);

static BAT_EXILE: [EffectDef; 2] = [
    EffectDef::ExileLinkedToSource {
        object: EffectRecipientDef::object(ObjectRefDef::Binding(ObjectBindingIndex::PRIMARY)),
    },
    EffectDef::InstallTrigger(InstalledTriggerDef::once(&BAT_RETURNS_IT)),
];

static BAT_LOOKS_AND_MAY_TAKE: [EffectDef; 2] = [
    EffectDef::LookAtHand {
        player: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    },
    // "You may exile" -- a minimum of none, so looking and taking nothing is
    // a legal answer. The Sculler and the Freebooter both must take one.
    EffectDef::Choose(ChooseDef {
        binding: ObjectChoiceBindingDef::Object(ObjectBindingIndex::PRIMARY),
        unchosen: None,
        chooser: PlayerRefDef::EffectController,
        candidates: ObjectSetDef::Query(ObjectQueryDef::owned_by(
            ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Land)),
            &[ZoneKind::Hand],
            PlayerSetDef::One(PlayerRefDef::Target(TargetIndex::PRIMARY)),
        )),
        exclude: None,
        minimum: 0,
        maximum: 1,
        visibility: ChoiceVisibilityDef::Public,
        then: &EffectDef::Sequence(&BAT_EXILE),
    }),
];

static BAT_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Player(PlayerRelation::Opponent),
)];

static DEEP_CAVERN_BAT_ABILITIES: [AbilityDef; 3] = [
    abilities::flying(),
    abilities::lifelink(),
    AbilityDef::triggered_with_targets(
        "When this creature enters, look at target opponent's hand. You may exile a nonland card from it until this creature leaves the battlefield.",
        TriggerEventDef::zone_changed(
            ObjectPredicateDef::Source,
            None,
            Some(ZoneKind::Battlefield),
        ),
        &BAT_TARGET,
        EffectDef::Sequence(&BAT_LOOKS_AND_MAY_TAKE),
    ),
];

static A_CREATURE_ENCHANTMENT_OR_PLANESWALKER: [AbilityTargetDef; 1] =
    [AbilityTargetDef::exactly_one_permanent(
        ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Creature),
            ObjectPredicateDef::HasType(CardType::Enchantment),
            ObjectPredicateDef::HasType(CardType::Planeswalker),
        ]),
    )];

/// "Its controller creates two Map tokens." The Maps are theirs, not yours,
/// and the permanent is already destroyed by the time they arrive -- so the
/// player is read from what the target was rather than from where it is.
static TWO_MAPS_FOR_ITS_CONTROLLER: EffectDef = EffectDef::create_token(tokens::map())
    .with_art(CardArt::new(
        "64839118-09d2-4645-9d3c-f80755ac781f",
        "Francesca Baerald",
    ))
    .with_controller(PlayerRefDef::ControllerOf(ObjectRefDef::Target(
        TargetIndex::PRIMARY,
    )))
    .with_amount(2);

static GET_LOST_EFFECT: [EffectDef; 2] = [
    EffectDef::Destroy {
        object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        can_regenerate: true,
    },
    TWO_MAPS_FOR_ITS_CONTROLLER,
];

// LCI 14 — Get Lost
pub(in crate::card::sets) static GET_LOST: CardRecord = CardRecord::new_with_legacy_id(
    2294,
    "Get Lost",
    CardArt::new("522aa72b-2b8c-484c-872b-f082101cee35", "Eli Minaya"),
    CardSet::LostCavernsOfIxalan,
    // Two mana that answers three card types at instant speed, and the two
    // Maps are what it pays for that: real but slow ones.
    CardRules::new_instant(mana_cost!("{1}{W}")).with_ability(AbilityDef::spell_with_targets(
        "Destroy target creature, enchantment, or planeswalker. Its controller creates two Map \
         tokens.",
        &A_CREATURE_ENCHANTMENT_OR_PLANESWALKER,
        EffectDef::Sequence(&GET_LOST_EFFECT),
    )),
);

/// One cost with two ways to pay it. The life is the way a deck with an
/// empty hand still casts this, which is what keeps it playable late.
static DISCARD_A_CARD_OR_PAY_THREE_LIFE: SpellAdditionalCostDef =
    SpellAdditionalCostDef::new(ObjectPredicateDef::Any, ZoneKind::Hand, 1).or_pay_life(3);

static A_CREATURE_OR_PLANESWALKER: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Creature),
            ObjectPredicateDef::HasType(CardType::Planeswalker),
        ]),
        zones: &[ZoneKind::Battlefield],
        controller: None,
        owner: None,
    },
)];

// LCI 91 — Bitter Triumph
pub(in crate::card::sets) static BITTER_TRIUMPH: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("05bdd22c-3e11-4c29-bdfa-d3dfc0e90a9f"),
    "Bitter Triumph",
    CardArt::new("05bdd22c-3e11-4c29-bdfa-d3dfc0e90a9f", "Donato Giancola"),
    CardSet::LostCavernsOfIxalan,
    // Two mana for unconditional removal at instant speed, and the card or
    // the three life is the whole restriction: it answers anything, and it
    // never answers it for free.
    CardRules::new_instant(mana_cost!("{1}{B}")).with_ability(
        AbilityDef::spell_with_additional_cost(
            "As an additional cost to cast this spell, discard a card or pay 3 life.\nDestroy \
             target creature or planeswalker.",
            &A_CREATURE_OR_PLANESWALKER,
            DISCARD_A_CARD_OR_PAY_THREE_LIFE,
            EffectDef::destroy_target(TargetIndex::PRIMARY, true),
        ),
    ),
);

// LCI 102 — Deep-Cavern Bat
pub(in crate::card::sets) static DEEP_CAVERN_BAT: CardRecord = CardRecord::new_with_legacy_id(
    2161,
    "Deep-Cavern Bat",
    CardArt::new("69c68c95-b788-43b1-9f22-1b22c5a00b25", "Campbell White"),
    CardSet::LostCavernsOfIxalan,
    CardRules::new_creature(mana_cost!("{1}{B}"), &["Bat"], 1, 1)
        .with_abilities(&DEEP_CAVERN_BAT_ABILITIES),
);

static INTI_TRAMPLE: AbilityDef = abilities::trample();

/// "It gains trample until end of turn" -- the creature that took the
/// counter, which is the one the trigger targeted.
static INTI_PUMP: [EffectDef; 2] = [
    EffectDef::AddCounters {
        object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        kind: CounterKind::PlusOnePlusOne,
        amount: ValueDef::Constant(1),
    },
    EffectDef::Apply {
        recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        effect: AppliedEffectDef::add_ability(&INTI_TRAMPLE),
        duration: ResolvedEffectDurationDef::UntilEndOfTurn,
    },
];

static AN_ATTACKING_CREATURE: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::All(&[
            ObjectPredicateDef::HasType(CardType::Creature),
            ObjectPredicateDef::Attacking,
        ]),
        zones: &[ZoneKind::Battlefield],
        controller: None,
        owner: None,
    },
)];

static INTI_ABILITIES: [AbilityDef; 2] = [
    // The target is declared as the attack trigger goes on the stack rather
    // than when the discard is made, which is the one place this differs
    // from the printed reflexive trigger. "Whenever you attack" guarantees
    // an attacking creature, so there is always something to name.
    AbilityDef::triggered_with_targets(
        "Whenever you attack, you may discard a card. When you do, put a +1/+1 counter on target \
         attacking creature. It gains trample until end of turn.",
        TriggerEventDef::attack_declared(ObjectPredicateDef::Any, 1, None),
        &AN_ATTACKING_CREATURE,
        EffectDef::PayOr(PayOrDef::optional(
            EffectPaymentDef {
                payer: PlayerSetDef::Related(PlayerRelation::You),
                cost: EffectPaymentCostDef::Discard(1),
            },
            &EffectDef::Sequence(&INTI_PUMP),
        )),
    ),
    // One trigger for the whole discard however many cards it took, and the
    // card it finds is playable into your own turn when the discard
    // happened on somebody else's.
    AbilityDef::triggered(
        "Whenever you discard one or more cards, exile the top card of your library. You may play \
         that card until your next end step.",
        TriggerEventDef::DiscardedCards(PlayerRelation::You),
        EffectDef::ExileTopOfLibraryToPlay {
            player: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
            free: false,
            face_down: false,
            duration: ExilePlayDurationDef::UntilYourNextEndStep,
        },
    ),
];

// LCI 156 — Inti, Seneschal of the Sun
pub(in crate::card::sets) static INTI_SENESCHAL_OF_THE_SUN: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("fa7a55aa-ae61-4933-b7a4-dcc55dac6fcd"),
    "Inti, Seneschal of the Sun",
    CardArt::new(
        "fa7a55aa-ae61-4933-b7a4-dcc55dac6fcd",
        "Victor Adame Minguez",
    ),
    CardSet::LostCavernsOfIxalan,
    // Two mana that turns every spare card into a bigger attack and a new
    // card, and the two halves feed each other: the discard he asks for is
    // the discard the second clause is watching for.
    CardRules::new_creature(mana_cost!("{1}{R}"), &["Human", "Knight"], 2, 2)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&INTI_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &GET_LOST,
    &BITTER_TRIUMPH,
    &DEEP_CAVERN_BAT,
    &INTI_SENESCHAL_OF_THE_SUN,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
