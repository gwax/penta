//! Lorwyn cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, CardArt, CardRules, CardSet, CardType,
    ChoiceVisibilityDef, ChooseDef, EffectDef, EffectRecipientDef, ObjectChoiceBindingDef,
    ObjectPredicateDef, ObjectQueryDef, ObjectRefDef, ObjectSetDef, PlayerRefDef, PlayerRelation,
    PlayerSetDef, TopCardSelectionDef, ValueDef, ZoneKind, ZonePlacement, cards,
};
use crate::ids::{ObjectBindingIndex, TargetIndex};
use crate::mana_cost;

/// The shuffle is the caster's call and comes after the look: having seen
/// the three, you decide whether to keep the arrangement or wash it away.
/// The draw is last either way, so a shuffled Ponder still finds a card.
static PONDER_SHUFFLE_AND_DRAW: EffectDef = EffectDef::Sequence(&[
    EffectDef::May {
        player: EffectRecipientDef::Controller,
        effect: &EffectDef::ShuffleLibrary {
            player: EffectRecipientDef::Controller,
        },
    },
    EffectDef::DrawCards {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::Constant(1),
    },
]);

/// Every card looked at is selected, which is what makes the choice an
/// ordering rather than a filter: all three go back on top, in the order
/// they were named.
static PONDER_LOOK: TopCardSelectionDef = TopCardSelectionDef {
    count: ValueDef::Constant(3),
    object: None,
    minimum: 3,
    maximum: 3,
    select_all_matching: false,
    reveal_selected: false,
    selected_zone: ZoneKind::Library,
    selected_placement: ZonePlacement::Top,
    rest_zone: ZoneKind::Library,
    rest_placement: ZonePlacement::Top,
    selected_order_follows_choice: true,
    then: Some(&PONDER_SHUFFLE_AND_DRAW),
};

// LRW 79 — Ponder
pub(in crate::card::sets) static PONDER: CardRecord = CardRecord::new(
    cards::PONDER,
    "Ponder",
    CardArt::new("ba6b6fc5-5077-4812-b8e9-906783dbaf67", "Mark Tedin"),
    CardSet::Lorwyn,
    // One mana to see four cards deep and keep the best of them, which is
    // why the format has never been able to leave it legal for long.
    CardRules::new_sorcery(mana_cost!("{U}")).with_ability(AbilityDef::spell(
        "Look at the top three cards of your library, then put them back in any order. You may \
         shuffle.\nDraw a card.",
        EffectDef::LookAtTopAndSelect {
            player: EffectRecipientDef::Controller,
            looker: EffectRecipientDef::Controller,
            selection: &PONDER_LOOK,
        },
    )),
);

static A_PLAYER: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Player(PlayerRelation::Any),
)];

static SEIZE_IT: EffectDef = EffectDef::DiscardCards {
    object: EffectRecipientDef::object(ObjectRefDef::Binding(ObjectBindingIndex::PRIMARY)),
};

/// The hand is revealed rather than looked at: everybody sees it, which is
/// what makes the choice checkable and what the card prints.
static THOUGHTSEIZE_EFFECT: [EffectDef; 3] = [
    EffectDef::RevealHand {
        player: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    },
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
        minimum: 1,
        maximum: 1,
        visibility: ChoiceVisibilityDef::Public,
        then: &SEIZE_IT,
    }),
    // Unconditional: a hand of nothing but lands still costs you two.
    EffectDef::LoseLife {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::Constant(2),
    },
];

// LRW 145 — Thoughtseize
pub(in crate::card::sets) static THOUGHTSEIZE: CardRecord = CardRecord::new(
    cards::THOUGHTSEIZE,
    "Thoughtseize",
    CardArt::new("3df8c148-e87d-4043-9d8b-ec72bf8b6d5d", "Aleksi Briclot"),
    CardSet::Lorwyn,
    // One mana, any card, two life. The life is what keeps it honest and it
    // has never been enough.
    CardRules::new_sorcery(mana_cost!("{B}")).with_ability(AbilityDef::spell_with_targets(
        "Target player reveals their hand. You choose a nonland card from it. That player \
         discards that card. You lose 2 life.",
        &A_PLAYER,
        EffectDef::Sequence(&THOUGHTSEIZE_EFFECT),
    )),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&PONDER, &THOUGHTSEIZE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
