//! Library of Alexandria: a land that draws a card for nothing, on the one
//! condition that the hand it is drawing into is exactly seven.

use super::*;

/// Player One with `libraries` Libraries out and a hand of `hand` cards.
fn staged(libraries: usize, hand: usize) -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(128_000 + index, cards::ISLAND, PlayerId::One));
    }
    for index in 0..hand {
        game.players[0].hand.push(card(
            128_100 + u32::try_from(index).expect("a small hand"),
            cards::MOUNTAIN,
            PlayerId::One,
        ));
    }
    let mut ids = Vec::new();
    for index in 0..libraries {
        let land = creature(
            128_200 + u32::try_from(index).expect("a couple of lands"),
            cards::LIBRARY_OF_ALEXANDRIA,
            PlayerId::One,
        );
        ids.push(land.card.id);
        game.battlefield.push(land);
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, ids)
}

/// Whether the draw ability of `library` is on offer.
fn draw_offered(game: &Game, library: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One).iter().any(
        |action| matches!(action, Action::ActivateAbility { source, .. } if *source == library),
    )
}

fn draw_action(game: &Game, library: GameObjectId) -> Action {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == library),
        )
        .expect("the draw is offered")
}

/// "Exactly seven": six is not enough and eight is too many.
#[test]
fn only_a_hand_of_exactly_seven_draws() {
    for (hand, offered) in [(6, false), (7, true), (8, false)] {
        let (game, libraries) = staged(1, hand);
        assert_eq!(
            draw_offered(&game, libraries[0]),
            offered,
            "a hand of {hand}",
        );
    }
}

/// The colourless half asks nothing of the hand: it is there at six cards
/// as readily as at seven.
#[test]
fn the_colorless_half_is_unconditional() {
    let (game, libraries) = staged(1, 6);

    assert!(
        game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::ActivateManaAbility { source, .. } if *source == libraries[0])
        }),
        "the mana ability does not read your hand",
    );
}

/// "You may tap multiples of these in response to each other because the
/// requirement for 7 cards is checked only at the time the ability is
/// announced and not again when it resolves."
#[test]
fn two_libraries_may_both_be_announced_at_seven() {
    let (mut game, libraries) = staged(2, 7);

    let first = draw_action(&game, libraries[0]);
    game.apply(PlayerId::One, first).expect("seven in hand");
    let second = draw_action(&game, libraries[1]);
    game.apply(PlayerId::One, second)
        .expect("still seven: nothing has resolved yet");

    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].hand.len(),
        9,
        "both drew, though the second resolved into a hand of eight",
    );
}
