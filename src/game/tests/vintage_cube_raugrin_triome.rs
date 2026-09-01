//! Raugrin Triome: three basic land types on a land that costs you the turn,
//! and a cycling clause for the turns you did not want the land.
//!
//! The cycle's shared behaviour -- tapped entry, its three colours, cycling
//! for exactly {3}, cycling at instant speed, and what a fetchland makes of
//! three basic types -- is in `vintage_cube_lands`. What is here is where
//! the cycling clause may be used from, and what the draw it makes counts
//! as.

use super::*;

/// Player One with a Triome in hand and three mana up.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let triome = game
        .build_zone(PlayerId::One, &[cards::RAUGRIN_TRIOME])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = triome.id;
    game.players[0].hand.push(triome);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].lands_played_this_turn = 0;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    (game, id)
}

fn cycling_of(game: &Game, triome: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One).into_iter().find(
        |action| matches!(action, Action::ActivateAbility { source, .. } if *source == triome),
    )
}

/// Cycling is a card ability that works from hand: a Triome already on the
/// battlefield is a land and nothing else, whatever mana is up.
#[test]
fn a_triome_on_the_battlefield_does_not_cycle() {
    let (mut game, triome) = staged();
    assert!(
        cycling_of(&game, triome).is_some(),
        "in hand it cycles for three",
    );

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == triome))
        .expect("a land drop is available");
    game.apply(PlayerId::One, play).expect("it is played");
    drain_pending(&mut game);
    let land = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::RAUGRIN_TRIOME)
        .expect("it arrived")
        .card
        .id;

    assert!(
        cycling_of(&game, land).is_none(),
        "and on the battlefield it does not",
    );
    assert_eq!(
        game.players[0].lands_played_this_turn, 1,
        "playing it spent the land drop",
    );
}

/// Cycling costs no land drop: the land you threw away is still a land you
/// may play afterwards, which is why holding a second one matters.
#[test]
fn cycling_leaves_the_land_drop_unspent() {
    let (mut game, triome) = staged();
    let second = game
        .build_zone(PlayerId::One, &[cards::ZAGOTH_TRIOME])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let second_id = second.id;
    game.players[0].hand.push(second);

    let cycle = cycling_of(&game, triome).expect("three mana cycles it");
    game.apply(PlayerId::One, cycle).expect("it is activated");
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].lands_played_this_turn, 0,
        "cycling is not a land play",
    );
    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if card == second_id)),
        "so the drop is still there for the next one",
    );
}

/// The other side of the surveil land's coin: cycling draws a card, and a
/// Sheoldred watching the table is paid for it.
#[test]
fn the_cycling_draw_is_a_draw() {
    let (mut game, triome) = staged();
    game.battlefield.push(creature(
        98_000,
        cards::SHEOLDRED_THE_APOCALYPSE,
        PlayerId::One,
    ));
    let life = game.players[0].life;
    let hand = game.players[0].hand.len();

    let cycle = cycling_of(&game, triome).expect("three mana cycles it");
    game.apply(PlayerId::One, cycle).expect("it is activated");
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, life + 2, "Sheoldred saw a card drawn");
    assert_eq!(
        game.players[0].hand.len(),
        hand,
        "the Triome left the hand and the card it drew replaced it",
    );
}
