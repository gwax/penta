//! Fastbond: a hand of lands becomes a turn's worth of mana, at a life for
//! each land after the first.

use super::*;

/// Fastbond on the battlefield under Player One with `lands` in hand.
fn staged(lands: usize) -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.put_onto_battlefield(PlayerId::One, cards::FASTBOND)
        .expect("cataloged");
    drain_pending(&mut game);
    let mut held = Vec::new();
    for index in 0..lands {
        let id = 121_000 + u32::try_from(index).expect("a small hand");
        let card = card(id, cards::FOREST, PlayerId::One);
        held.push(card.id);
        game.players[0].hand.push(card);
    }
    game.players[0].lands_played_this_turn = 0;
    game.players[0].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
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

fn play_land(game: &mut Game, card: GameObjectId) -> bool {
    let Some(action) = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card: played, .. } if *played == card))
    else {
        return false;
    };
    game.apply(PlayerId::One, action).expect("it is played");
    settle(game);
    true
}

fn lands(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == cards::FOREST)
        .count()
}

/// The first land is free; every one after it costs a life.
#[test]
fn every_land_after_the_first_costs_a_life() {
    let (mut game, held) = staged(3);

    assert!(play_land(&mut game, held[0]), "the ordinary land drop");
    assert_eq!(game.players[0].life, 20, "the first one is free");

    assert!(play_land(&mut game, held[1]), "and a second is allowed");
    assert_eq!(game.players[0].life, 19, "which costs a life");

    assert!(play_land(&mut game, held[2]), "and a third");
    assert_eq!(game.players[0].life, 18);
    assert_eq!(lands(&game), 3, "all three are on the battlefield");
}

/// Without Fastbond the second land is not a legal play at all.
#[test]
fn one_land_a_turn_without_it() {
    let (mut game, held) = staged(2);
    game.battlefield
        .retain(|permanent| permanent.card.definition != cards::FASTBOND);

    assert!(play_land(&mut game, held[0]), "the ordinary land drop");
    assert!(
        !play_land(&mut game, held[1]),
        "and nothing offers a second",
    );
    assert_eq!(game.players[0].life, 20, "so no life was paid either");
}

/// The count is per turn: the first land of the next turn is free again.
#[test]
fn the_count_resets_with_the_turn() {
    let (mut game, held) = staged(3);
    play_land(&mut game, held[0]);
    play_land(&mut game, held[1]);
    assert_eq!(game.players[0].life, 19);

    game.players[0].lands_played_this_turn = 0;

    play_land(&mut game, held[2]);
    assert_eq!(game.players[0].life, 19, "the first of a turn is free");
}

/// A land an effect puts onto the battlefield was never played, so it costs
/// nothing and does not use up the free one either.
#[test]
fn a_land_put_onto_the_battlefield_costs_nothing() {
    let (mut game, held) = staged(1);

    game.put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    settle(&mut game);
    assert_eq!(game.players[0].life, 20, "nothing was played");

    assert!(play_land(&mut game, held[0]), "the land drop is untouched");
    assert_eq!(game.players[0].life, 20, "and it is still the first");
}

/// Nothing stops the trigger and nothing checks whether you can afford it:
/// a player at one life who plays a second land has played their last.
#[test]
fn it_will_kill_you_for_a_land_you_did_not_need() {
    let (mut game, held) = staged(2);
    game.players[0].life = 1;

    assert!(play_land(&mut game, held[0]), "the first one is free");
    assert_eq!(game.players[0].life, 1, "and costs nothing");

    assert!(play_land(&mut game, held[1]), "the second one is offered");
    game.check_state_based_actions();

    assert_eq!(game.players[0].life, 0);
    assert!(game.result.is_some(), "one damage was all it took");
}

/// Nothing about it is legendary: a second copy is a second trigger, so the
/// same extra land costs two.
#[test]
fn two_fastbonds_are_two_damage_a_land() {
    let (mut game, held) = staged(2);
    game.put_onto_battlefield(PlayerId::One, cards::FASTBOND)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    assert!(play_land(&mut game, held[0]), "the first land is played");
    assert_eq!(
        game.players[0].life, 20,
        "neither copy asks about the first one",
    );

    assert!(play_land(&mut game, held[1]), "and the second");
    assert_eq!(game.players[0].life, 18, "one damage from each of them");
}
