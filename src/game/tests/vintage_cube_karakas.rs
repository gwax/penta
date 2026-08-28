//! Karakas: a legendary land that answers a legend a turn, and hands it back
//! to whoever owns it rather than to whoever was playing with it.

use super::*;

/// Karakas untapped under Player One, with `legend` on the battlefield.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    let karakas = game
        .put_onto_battlefield(PlayerId::One, cards::KARAKAS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.tapped = false;
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, karakas)
}

/// The bounce activation Karakas offers, if it is offered at all.
fn bounce(game: &Game, karakas: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, ability, .. }
            if *source == karakas
                && matches!(ability, AbilityOrigin::Printed { ability, .. }
                    if *ability == AbilityId(1)))
        })
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
    game.check_state_based_actions();
}

/// It answers a legend on the other side of the table.
#[test]
fn it_returns_a_legendary_creature() {
    let (mut game, karakas) = staged();
    game.battlefield
        .push(creature(11_000, cards::TETSUO_UMEZAWA, PlayerId::Two));

    let action = bounce(&game, karakas).expect("a legend is a legal target");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TETSUO_UMEZAWA),
        "the legend left the battlefield",
    );
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::TETSUO_UMEZAWA],
        "and went to its owner's hand",
    );
}

/// "Its owner's hand", not its controller's: a legend somebody has taken
/// control of goes home to the player who owns the card.
#[test]
fn a_stolen_legend_goes_back_to_its_owner() {
    let (mut game, karakas) = staged();
    // Owned by Player Two, on the battlefield under Player One.
    let mut stolen = creature(11_001, cards::TETSUO_UMEZAWA, PlayerId::Two);
    stolen.controller = PlayerId::One;
    game.battlefield.push(stolen);

    let action = bounce(&game, karakas).expect("a legend is a legal target");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    assert!(
        game.players[0].hand.is_empty(),
        "not to the player who was using it",
    );
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::TETSUO_UMEZAWA],
        "but to the player who owns it",
    );
}

/// A creature without the supertype is no target at all.
#[test]
fn an_ordinary_creature_is_not_a_target() {
    let (mut game, karakas) = staged();
    game.battlefield
        .push(creature(11_002, cards::GRIZZLY_BEARS, PlayerId::Two));

    assert!(
        bounce(&game, karakas).is_none(),
        "only a legendary creature answers to it",
    );
}
