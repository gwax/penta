//! Courser of Kruphix: a body that turns the top of your library into a
//! second hand for lands, and shows it to the table while it does.

use super::*;

/// Player One with a Courser out and `library` stacked so the last entry is
/// on top.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    let courser = game
        .put_onto_battlefield(PlayerId::One, cards::COURSER_OF_KRUPHIX)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [1, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    (game, courser)
}

fn top(game: &Game) -> GameObjectId {
    game.players[0].library.last().expect("a library").id
}

fn land_play_of(game: &Game, card: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card: id, .. } if *id == card))
}

fn settle(game: &mut Game) {
    for _ in 0..16 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// "Revealed", not "you may look": the card is public, so the other player
/// plans around it too.
#[test]
fn the_top_card_is_revealed_to_both_players() {
    let (game, courser) = staged(&[cards::FOREST]);
    let card = top(&game);

    assert_eq!(
        game.observe(PlayerId::One).revealed_library_top,
        Some((card, cards::FOREST)),
        "its controller sees their own top card",
    );
    assert_eq!(
        game.observe(PlayerId::Two).opponent_revealed_library_top,
        Some((card, cards::FOREST)),
        "and so does the player across the table",
    );

    let mut game = game;
    game.battlefield
        .retain(|permanent| permanent.card.id != courser);
    assert_eq!(
        game.observe(PlayerId::One).revealed_library_top,
        None,
        "with the Courser gone the library is face down again",
    );
    assert_eq!(
        game.observe(PlayerId::Two).opponent_revealed_library_top,
        None,
    );
}

/// Only the player with the Courser plays revealed: it says "your library",
/// so the opponent's stays where it was.
#[test]
fn it_does_not_reveal_the_other_library() {
    let (game, _courser) = staged(&[cards::FOREST]);

    assert_eq!(
        game.observe(PlayerId::Two).revealed_library_top,
        None,
        "the opponent still cannot see their own top card",
    );
    assert_eq!(
        game.observe(PlayerId::One).opponent_revealed_library_top,
        None,
        "and neither can the Courser's controller",
    );
}

/// A land on top is playable from there, and playing it feeds the Courser's
/// own landfall.
#[test]
fn a_land_on_top_can_be_played_for_a_life() {
    let (mut game, _courser) = staged(&[cards::FOREST]);
    let forest = top(&game);

    let play = land_play_of(&game, forest).expect("the top land is playable");
    game.apply(PlayerId::One, play).expect("it is played");
    settle(&mut game);

    assert!(game.players[0].library.is_empty(), "it left the library");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "and arrived on the battlefield",
    );
    assert_eq!(game.players[0].life, 21, "landfall paid its one life");
}

/// It is a land drop like any other, so the second land of the turn is not
/// on offer however many are stacked up.
#[test]
fn it_still_costs_your_land_drop() {
    let (mut game, _courser) = staged(&[cards::FOREST, cards::MOUNTAIN]);
    let mountain = top(&game);

    let play = land_play_of(&game, mountain).expect("the top land is playable");
    game.apply(PlayerId::One, play).expect("it is played");
    settle(&mut game);

    let forest = top(&game);
    assert_eq!(
        game.observe(PlayerId::One).revealed_library_top,
        Some((forest, cards::FOREST)),
        "the next card is revealed as soon as it is on top",
    );
    assert!(
        land_play_of(&game, forest).is_none(),
        "but this turn's land has already been played",
    );
}

/// Lands only. A spell on top is revealed to everyone and castable by
/// nobody.
#[test]
fn a_spell_on_top_is_visible_but_not_castable() {
    let (game, _courser) = staged(&[cards::LIGHTNING_BOLT]);
    let bolt = top(&game);

    assert_eq!(
        game.observe(PlayerId::One).revealed_library_top,
        Some((bolt, cards::LIGHTNING_BOLT)),
        "revealed all the same",
    );
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == bolt)),
        "the permission is for lands only",
    );
}

/// The landfall clause reads "a land you control", not "a land from your
/// library": an ordinary land drop out of hand gains the life too.
#[test]
fn landfall_does_not_care_where_the_land_came_from() {
    let (mut game, _courser) = staged(&[cards::LIGHTNING_BOLT]);
    let hand_land = game
        .build_zone(PlayerId::One, &[cards::ISLAND])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let island = hand_land.id;
    game.players[0].hand.push(hand_land);

    let play = land_play_of(&game, island).expect("a land in hand is playable");
    game.apply(PlayerId::One, play).expect("it is played");
    settle(&mut game);

    assert_eq!(game.players[0].life, 21, "one life for the land drop");
}
