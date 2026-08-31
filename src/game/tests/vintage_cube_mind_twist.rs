//! Mind Twist: X cards out of a hand, and nobody chooses which.

use super::*;

/// The five cards Player Two is holding, all distinguishable.
const HELD: [CardDefinitionId; 5] = [
    cards::LIGHTNING_BOLT,
    cards::GRIZZLY_BEARS,
    cards::COUNTERSPELL,
    cards::SERRA_ANGEL,
    cards::FOREST,
];

/// Player One holding a Mind Twist with mana for any X, and Player Two
/// holding `held`.
fn staged(seed: u64, held: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game_with_seed(seed);
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    game.players[PlayerId::Two.index()].graveyard.clear();
    for (index, definition) in held.iter().enumerate() {
        game.players[PlayerId::Two.index()].hand.push(card(
            99_000 + u32::try_from(index).expect("a small hand"),
            *definition,
            PlayerId::Two,
        ));
    }
    let twist = card(99_500, cards::MIND_TWIST, PlayerId::One);
    let twist_id = twist.id;
    game.players[PlayerId::One.index()].hand.push(twist);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 8);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, twist_id)
}

/// Casts it at Player Two for `x` and lets it resolve.
fn twist(game: &mut Game, twist: GameObjectId, x: u16) {
    game.apply(
        PlayerId::One,
        cast_action(twist, vec![Target::Player(PlayerId::Two)], Vec::new(), x),
    )
    .expect("a player is what it targets");
    drain_pending(game);
}

fn discarded(game: &Game) -> Vec<CardDefinitionId> {
    let mut cards = game.players[PlayerId::Two.index()]
        .graveyard
        .iter()
        .map(|card| card.definition)
        .collect::<Vec<_>>();
    cards.sort_unstable();
    cards
}

/// X cards leave the hand for the graveyard.
#[test]
fn it_discards_x_cards() {
    let (mut game, id) = staged(7, &HELD);

    twist(&mut game, id, 2);

    assert_eq!(game.players[PlayerId::Two.index()].hand.len(), 3);
    assert_eq!(
        discarded(&game).len(),
        2,
        "and the two are in the graveyard"
    );
}

/// "At random" is nobody's decision: neither the caster who paid for it nor
/// the player losing the cards is asked which ones go.
#[test]
fn nobody_chooses_which_cards_go() {
    let (mut game, id) = staged(7, &HELD);

    game.apply(
        PlayerId::One,
        cast_action(id, vec![Target::Player(PlayerId::Two)], Vec::new(), 2),
    )
    .expect("a player is what it targets");
    pass_priority_pair(&mut game);

    assert!(
        game.pending_decisions.is_empty(),
        "the discard resolved without asking anyone",
    );
    assert_eq!(discarded(&game).len(), 2, "and it still took two");
}

/// The randomness is the game's seed rather than a fixed order: the same
/// seed takes the same card every time, and across seeds it is not always
/// the same one.
#[test]
fn which_card_goes_follows_the_seed() {
    let taken = (0..8)
        .map(|seed| {
            let (mut game, id) = staged(seed, &HELD);
            twist(&mut game, id, 1);
            let gone = discarded(&game);
            assert_eq!(gone.len(), 1, "one card for X of one");
            gone[0]
        })
        .collect::<Vec<_>>();

    for (seed, expected) in taken.iter().enumerate() {
        let (mut game, id) = staged(u64::try_from(seed).expect("small"), &HELD);
        twist(&mut game, id, 1);
        assert_eq!(
            discarded(&game)[0],
            *expected,
            "seed {seed} takes the same card twice over",
        );
    }
    assert!(
        taken.iter().any(|card| *card != taken[0]),
        "and the seed decides which: {taken:?}",
    );
}

/// X larger than the hand takes the hand and stops there.
#[test]
fn an_x_bigger_than_the_hand_takes_all_of_it() {
    let (mut game, id) = staged(7, &[cards::LIGHTNING_BOLT, cards::FOREST]);

    twist(&mut game, id, 5);

    assert!(
        game.players[PlayerId::Two.index()].hand.is_empty(),
        "there was nothing left to take",
    );
    assert_eq!(discarded(&game).len(), 2);
}

/// X of zero is a legal cast and an empty threat.
#[test]
fn an_x_of_zero_takes_nothing() {
    let (mut game, id) = staged(7, &HELD);

    twist(&mut game, id, 0);

    assert_eq!(game.players[PlayerId::Two.index()].hand.len(), 5);
    assert!(discarded(&game).is_empty());
}

/// "Target player" is any player: it will point at its own caster, which is
/// the only way a Mind Twist is ever a bad idea.
#[test]
fn it_may_be_pointed_at_yourself() {
    let (mut game, id) = staged(7, &HELD);
    game.players[PlayerId::One.index()]
        .hand
        .push(card(99_600, cards::ISLAND, PlayerId::One));

    assert!(
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::CastSpell { card, choices, .. }
                if *card == id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::One)))
        ),
        "your own hand is a legal thing to twist",
    );
}
