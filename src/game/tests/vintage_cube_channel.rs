//! Channel: two green for a life-for-mana faucet that lasts one turn.
//!
//! What the mana can pay for, and how the planner counts it, is covered
//! where the mana engine is tested. What this file adds is the shape of the
//! permission itself: it does not exist before the spell, it does not
//! survive the turn, and it stops where the life stops.

use super::*;

/// The activations Channel is offering Player One right now.
fn faucet(game: &Game) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| {
            matches!(action, Action::ActivateManaAbility { source, color, .. }
                if *color == ManaColor::Colorless
                    && game
                        .battlefield
                        .iter()
                        .all(|permanent| permanent.card.id != *source))
        })
        .collect()
}

/// A life for a colourless, and the ability is there only because the spell
/// resolved.
#[test]
fn it_turns_life_into_mana_one_point_at_a_time() {
    let mut game = ready_game();
    game.battlefield.clear();
    assert!(
        faucet(&game).is_empty(),
        "nothing offers this before the spell",
    );

    resolve_channel(&mut game);
    let life = game.players[PlayerId::One.index()].life;

    let pay = faucet(&game)
        .into_iter()
        .next()
        .expect("the faucet is open");
    game.apply(PlayerId::One, pay).expect("a life is payable");

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 1,
        "one life",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.colorless,
        1,
        "for one colourless",
    );
}

/// "Once your life total is 0, you can't pay any more life, even if you've
/// somehow not lost the game yet."
#[test]
fn at_no_life_there_is_nothing_left_to_pay_with() {
    let mut game = ready_game();
    game.battlefield.clear();
    resolve_channel(&mut game);

    game.players[PlayerId::One.index()].life = 1;
    assert_eq!(faucet(&game).len(), 1, "one life is one more mana");

    game.players[PlayerId::One.index()].life = 0;
    assert!(
        faucet(&game).is_empty(),
        "and none is the end of the faucet, game over or not",
    );
}

/// "Until end of turn": the permission goes with the turn that made it, and
/// the life it did not spend stays where it is.
#[test]
fn the_faucet_closes_at_the_end_of_the_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    resolve_channel(&mut game);
    assert!(!faucet(&game).is_empty(), "open on the turn it resolved");

    let turn = game.turn;
    for _ in 0..60 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }
    assert!(game.turn > turn, "a whole turn has passed");
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    assert!(faucet(&game).is_empty(), "and shut once that turn is over");
}

/// The faucet has no limit but the life total: five points bought five
/// colourless in one turn, and the pool holds all of them at once.
#[test]
fn it_may_be_paid_over_and_over_in_the_same_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    resolve_channel(&mut game);
    let life = game.players[PlayerId::One.index()].life;

    for _ in 0..5 {
        let pay = faucet(&game)
            .into_iter()
            .next()
            .expect("the faucet is still open");
        game.apply(PlayerId::One, pay).expect("a life is payable");
    }

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 5,
        "five points of life",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.colorless,
        5,
        "for five colourless, all in the pool together",
    );
}

/// "You may pay 1 life": the permission belongs to the player who cast it.
/// The other player's life buys them nothing.
#[test]
fn their_life_is_not_theirs_to_spend() {
    let mut game = ready_game();
    game.battlefield.clear();
    resolve_channel(&mut game);
    // A land of their own, so what they are offered is a real list rather
    // than an empty one.
    game.put_onto_battlefield(PlayerId::Two, cards::FOREST)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::Two;

    let theirs: Vec<ManaColor> = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { color, .. } => Some(color),
            _ => None,
        })
        .collect();
    assert_eq!(
        theirs,
        vec![ManaColor::Green],
        "their Forest and nothing else: the faucet is the caster's alone",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].mana_pool.colorless,
        0,
        "and nothing arrived in their pool",
    );
}
