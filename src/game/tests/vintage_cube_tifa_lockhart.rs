//! Tifa Lockhart: a 1/2 trampler that doubles for every land, however the
//! land got there -- and forgets it all at the end of the turn.

use super::*;

/// Tifa on the battlefield with a land in hand and a fetchland out.
fn staged() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(129_000, cards::FOREST, PlayerId::One));
    // The Heath first, so its own arrival is nothing to her: what the tests
    // count is the land she sees after she is on the battlefield.
    game.put_onto_battlefield(PlayerId::One, cards::WINDSWEPT_HEATH)
        .expect("cataloged");
    drain_pending(&mut game);
    let tifa = creature(129_100, cards::TIFA_LOCKHART, PlayerId::One);
    let tifa_id = tifa.card.id;
    game.battlefield.push(tifa);
    let held = card(129_200, cards::MOUNTAIN, PlayerId::One);
    let held_id = held.id;
    game.players[0].hand.push(held);
    game.players[0].lands_played_this_turn = 0;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, tifa_id, held_id)
}

fn power_of(game: &Game, tifa: GameObjectId) -> Option<i16> {
    game.power(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == tifa)
            .expect("she is on the battlefield"),
    )
}

fn settle(game: &mut Game) {
    for _ in 0..16 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
}

/// "It triggers whenever you play a land, as well as whenever a spell or
/// ability puts a land onto the battlefield under your control." A land
/// drop and a cracked fetchland are each worth one doubling.
#[test]
fn a_land_drop_and_a_fetch_each_double_her() {
    let (mut game, tifa, held) = staged();
    assert_eq!(power_of(&game, tifa), Some(1), "a 1/2 to start");

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held))
        .expect("a land drop is available");
    game.apply(PlayerId::One, play).expect("it is played");
    settle(&mut game);
    assert_eq!(power_of(&game, tifa), Some(2), "the land you played counts");

    let fetch = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::WINDSWEPT_HEATH)
        .expect("the Heath is there")
        .card
        .id;
    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == fetch))
        .expect("the fetch is offered");
    game.apply(PlayerId::One, crack).expect("it activates");
    settle(&mut game);

    assert_eq!(
        power_of(&game, tifa),
        Some(4),
        "and so does the one the Heath put there",
    );
}

/// "Until end of turn": the doubling is gone by the next turn, however much
/// of it there was.
#[test]
fn the_doubling_is_gone_next_turn() {
    let (mut game, tifa, held) = staged();
    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held))
        .expect("a land drop is available");
    game.apply(PlayerId::One, play).expect("it is played");
    settle(&mut game);
    assert_eq!(power_of(&game, tifa), Some(2));

    game.cleanup();
    game.check_state_based_actions();

    assert_eq!(
        power_of(&game, tifa),
        Some(1),
        "she is a 1/2 again when the turn is over",
    );
}
