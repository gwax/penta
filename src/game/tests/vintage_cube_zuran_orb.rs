//! Zuran Orb: a free artifact that turns a land into two life, again and
//! again.
//!
//! That one land buys two life, and that an empty board offers nothing, is
//! pinned in `vintage_cube_artifacts`. What is here is the rest of the
//! bargain: whose lands it may eat, and how many of them in one window.

use super::*;

/// The Orb on the battlefield with `mine` lands beside it and `theirs`
/// across the table.
fn staged(mine: usize, theirs: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let orb = game
        .put_onto_battlefield(PlayerId::One, cards::ZURAN_ORB)
        .expect("cataloged");
    for (player, count) in [(PlayerId::One, mine), (PlayerId::Two, theirs)] {
        for index in 0..count {
            let id = 113_000
                + u32::from(player == PlayerId::Two) * 100
                + u32::try_from(index).expect("a few lands");
            game.battlefield.push(creature(id, cards::FOREST, player));
        }
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.players[0].life = 10;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, orb)
}

fn activation(game: &Game, orb: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == orb))
}

/// "Sacrifice a land" is a land you control: theirs is no cost of yours.
#[test]
fn it_cannot_eat_their_lands() {
    let (game, orb) = staged(0, 3);

    assert!(
        activation(&game, orb).is_none(),
        "three Forests across the table pay for nothing",
    );
}

/// Nothing taps and nothing limits it: on their turn, in response to
/// whatever they are doing, the Orb eats the whole board for two life a
/// land.
#[test]
fn it_eats_a_whole_board_at_instant_speed() {
    let (mut game, orb) = staged(3, 0);
    game.active_player = PlayerId::Two;
    game.step = Step::End;
    game.priority = PlayerId::One;

    for _ in 0..3 {
        // Resolving the last activation handed priority back around; the
        // window is still theirs and the Orb is still yours.
        game.priority = PlayerId::One;
        let action = activation(&game, orb).expect("another land to eat");
        game.apply(PlayerId::One, action).expect("it activates");
        drain_pending(&mut game);
    }

    assert_eq!(game.players[0].life, 16, "three lands is six life");
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.definition != cards::FOREST),
        "and the board is what paid for it",
    );
    assert!(
        activation(&game, orb).is_none(),
        "with the last land gone there is nothing left to sacrifice",
    );
    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == orb)
            .is_some_and(|permanent| !permanent.tapped),
        "the Orb never tapped for any of it",
    );
}
