//! Hymn to Tourach: two cards out of a hand, chosen by nobody.

use super::*;

/// Player One holding a Hymn with the mana for it, and `theirs` cards in
/// Player Two's hand.
fn staged(theirs: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    game.players[PlayerId::Two.index()].graveyard.clear();
    for index in 0..theirs {
        game.players[PlayerId::Two.index()].hand.push(card(
            102_000 + u32::try_from(index).expect("a small hand"),
            cards::LIGHTNING_BOLT,
            PlayerId::Two,
        ));
    }
    let hymn = card(102_100, cards::HYMN_TO_TOURACH, PlayerId::One);
    let hymn_id = hymn.id;
    game.players[PlayerId::One.index()].hand.push(hymn);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, hymn_id)
}

fn cast_at_them(game: &mut Game, hymn: GameObjectId) {
    game.apply(
        PlayerId::One,
        cast_action(hymn, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("a player is what it targets");
    pass_priority_pair(game);
    drain_pending(game);
}

/// Two cards, and neither player picks which: nothing is asked of anybody.
#[test]
fn it_takes_two_and_asks_nobody() {
    let (mut game, hymn) = staged(4);

    cast_at_them(&mut game, hymn);

    assert!(
        game.pending_decisions.is_empty(),
        "at random is nobody's choice to make",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        2,
        "two of their four are gone",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].graveyard.len(),
        2,
        "and both are in their graveyard",
    );
}

/// "Discards two cards" takes what there is: a hand of one loses the one.
#[test]
fn a_hand_of_one_loses_that_one() {
    let (mut game, hymn) = staged(1);

    cast_at_them(&mut game, hymn);

    assert!(game.players[PlayerId::Two.index()].hand.is_empty());
    assert_eq!(
        game.players[PlayerId::Two.index()].graveyard.len(),
        1,
        "one card is all it could take",
    );
}

/// And an empty hand loses nothing at all, while the Hymn is spent.
#[test]
fn an_empty_hand_loses_nothing() {
    let (mut game, hymn) = staged(0);

    cast_at_them(&mut game, hymn);

    assert!(game.players[PlayerId::Two.index()].graveyard.is_empty());
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::HYMN_TO_TOURACH),
        "the sorcery resolved into an empty hand and is spent",
    );
}

/// It is a sorcery: their turn is no time for it, however much black is up.
#[test]
fn it_waits_for_your_own_main_phase() {
    let (mut game, hymn) = staged(4);
    let castable = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == hymn))
    };
    assert!(castable(&game), "your own main phase is its window");

    game.active_player = PlayerId::Two;
    game.priority = PlayerId::One;
    assert!(!castable(&game), "and their main phase is not");

    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    assert!(!castable(&game), "nor is your own combat");
}
